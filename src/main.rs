// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]

mod config;
mod vec3;

use std::{array, fs::File, io::Write as _, mem::size_of, ops::Mul, slice};

use vec3::{NormalizedVec3, Vec3};

fn main() {
    let scene = config::parse();
    let image = scene.render();
    image.write_ppm_p6();
}

struct Image {
    width: usize,
    height: usize,

    data: Vec<Color<u8>>,
}
impl Image {
    fn new(width: usize, height: usize) -> Self {
        let data = Vec::with_capacity(width * height);
        Self {
            width,
            height,
            data,
        }
    }
    fn write_ppm_p6(&self) {
        let mut file = File::create("target/out.ppm").unwrap();

        // Write ppm header
        writeln!(&mut file, "P6\n{} {} 255", self.width, self.height).unwrap();

        // SAFETY:
        // - `Pixel` is a `repr(transparent)` wrapper around [u8;3],
        // - so `self.data` is effectively a &[[u8;3]]
        // - [u8;3] and u8 have the same alignment
        // - We adjust the length of the resulting slice
        file.write_all(unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast::<u8>(),
                self.data.len() * size_of::<Color<u8>>(),
            )
        })
        .unwrap();

        file.flush().unwrap();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
/// A rgb color
/// for Color<f32> values should be between 0 and 1
struct Color<T>([T; 3]);

impl Mul for Color<f32> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(array::from_fn(|index| self.0[index] * rhs.0[index]))
    }
}
impl Mul<f32> for Color<f32> {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0.map(|num| num * rhs))
    }
}
impl From<Color<f32>> for Color<u8> {
    #[expect(clippy::cast_possible_truncation)] // We check in debug mode
    #[expect(clippy::cast_sign_loss)]
    fn from(value: Color<f32>) -> Self {
        Self(value.0.map(|num| {
            debug_assert!((0.0..1.).contains(&num));

            (num * 255.) as u8
        }))
    }
}
impl From<&str> for Color<f32> {
    fn from(value: &str) -> Self {
        let mut values = value.split(' ').map(|value| value.parse().unwrap());

        Self([
            values.next().unwrap(),
            values.next().unwrap(),
            values.next().unwrap(),
        ])
    }
}

#[derive(PartialEq)]
struct Sphere {
    center: Vec3,
    radius: f32,
    color: Color<f32>,
}
impl Sphere {
    const fn new(center: Vec3, radius: f32, color: Color<f32>) -> Self {
        Self {
            center,
            radius,
            color,
        }
    }
}

#[derive(Debug)]
struct Ray {
    origin: Vec3,
    direction: NormalizedVec3,
}
impl Ray {
    const fn new(origin: Vec3, direction: NormalizedVec3) -> Self {
        Self { origin, direction }
    }
}

trait Intersects {
    /// The scale at which the ray intersects the object
    fn intersects(&self, ray: &Ray) -> Option<f32>;
}
impl Intersects for Sphere {
    // See `ray_sphere_intersection_derivation.latex` for the formula used here
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let delta_origin = ray.origin - self.center;

        let delta_origin_direction = delta_origin.dot(*ray.direction.inner());
        let discriminant = delta_origin_direction * delta_origin_direction
            - delta_origin.dot(delta_origin)
            + self.radius * self.radius;

        if discriminant < 0.0 {
            return None; // No solution to quadratic formula
        }

        // The first intersection point
        let t1 = -delta_origin_direction - discriminant.sqrt();

        // If t1 is positive (in front of the origin), return it, as
        // t1 is always closer than t2, because we subtract,
        // instead of add the discriminant (which is always positive)
        if t1 > 0.0 {
            Some(t1)
        } else {
            // The second intersection point
            let t2 = -delta_origin_direction + discriminant.sqrt();

            // If t2 is positive, return it, else None
            (t2 > 0.0).then_some(t2)
        }
    }
}

struct Scene {
    screen: Screen,
    camera: Camera,
    spheres: Vec<Sphere>,
    light: Light,
}

impl Scene {
    // The only precision loss is turning the resolution into floats, which is fine
    #[expect(clippy::cast_precision_loss)]
    fn render(&self) -> Image {
        let row_step = self.screen.top_edge / (self.screen.resolution_width - 1) as f32;
        let column_step = self.screen.left_edge / (self.screen.resolution_height - 1) as f32;

        let mut image = Image::new(self.screen.resolution_width, self.screen.resolution_height);
        for y in 0..self.screen.resolution_height {
            for x in 0..self.screen.resolution_width {
                let pixel_position =
                    self.screen.top_left + row_step * x as f32 + column_step * y as f32;

                let ray = Ray::new(
                    self.camera.position,
                    (pixel_position - self.camera.position).normalize(),
                );

                // Find closest (sphere, distance)
                let color = if let Some((sphere, distance)) = self
                    .spheres
                    .iter()
                    .filter_map(|sphere| sphere.intersects(&ray).map(|distance| (sphere, distance)))
                    .min_by(|&(_, distance1), &(_, distance2)| {
                        distance1
                            .partial_cmp(&distance2)
                            .expect("Ordering between distances should exist")
                    }) {
                    let hit_point = ray.origin + *ray.direction.inner() * distance;

                    let normal = (hit_point - sphere.center).normalize();

                    let light_direction = (self.light.position - hit_point).normalize();
                    let light_ray = Ray::new(hit_point, light_direction);

                    // If the ray to the light source intersects any other sphere
                    if self
                        .spheres
                        .iter()
                        .filter(|&other_sphere| *other_sphere != *sphere)
                        .any(|sphere| sphere.intersects(&light_ray).is_some())
                    {
                        Color([0; 3])
                    } else {
                        // How straight the light is falling on the surface
                        let color_coefficient =
                            light_direction.inner().dot(*normal.inner()).max(0.); // Can maybe be optimised to not consider cases where the normal points away from the light

                        (self.light.color * sphere.color * color_coefficient).into()
                    }
                } else {
                    Color([0; 3])
                };

                image.data.push(color);
            }
        }
        image
    }
}

struct Screen {
    top_left: Vec3,
    top_edge: Vec3,
    left_edge: Vec3,
    resolution_width: usize,
    resolution_height: usize,
}
impl Screen {
    const fn new(
        top_left: Vec3,
        top_edge: Vec3,
        left_edge: Vec3,
        resolution_width: usize,
        resolution_height: usize,
    ) -> Self {
        Self {
            top_left,
            top_edge,
            left_edge,
            resolution_width,
            resolution_height,
        }
    }
}

struct Camera {
    position: Vec3,
}
impl Camera {
    const fn new(position: Vec3) -> Self {
        Self { position }
    }
}

struct Light {
    position: Vec3,
    color: Color<f32>,
}
impl Light {
    const fn new(position: Vec3, color: Color<f32>) -> Self {
        Self { position, color }
    }
}
