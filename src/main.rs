#![feature(file_buffered)]
// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]

use std::fs::File;
use std::io::Write as _;
use std::ops::{Add, Div, Mul, Sub};
use std::slice;

fn main() {
    let screen = Screen::new(
        Vec3::new(-0.5, 0.5, 10.),
        Vec3::new(1., 0., 0.),
        Vec3::new(0., -1., 0.),
        1000,
        1000,
    );
    let camera = Camera::new(Vec3::new(0., 0., 20.));
    let sphere = Sphere::new(Vec3::new(0., 0., 0.), 1.);
    let scene = Scene::new(screen, camera, sphere);
    let image = scene.render();
    image.write_ppm_p6();
}

struct Image {
    width: usize,
    height: usize,

    data: Vec<Pixel>,
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
                self.data.len() * size_of::<Pixel>(),
            )
        })
        .unwrap();

        file.flush().unwrap();
    }
}

#[repr(transparent)]
struct Pixel([u8; 3]);

#[derive(Clone, Copy, Debug)]
/// A right-handed cartesian coordinate
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
    fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }
    fn normalize(self) -> NormalizedVec3 {
        NormalizedVec3(self / self.length())
    }
}
impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct NormalizedVec3(Vec3);

struct Sphere {
    center: Vec3,
    radius: f32,
}
impl Sphere {
    const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
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

        let delta_origin_direction = delta_origin.dot(ray.direction.0);
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
    sphere: Sphere,
}

impl Scene {
    const fn new(screen: Screen, camera: Camera, sphere: Sphere) -> Self {
        Self {
            screen,
            camera,
            sphere,
        }
    }
    // The only precision loss is turning the resolution into floats, which is fine
    #[expect(clippy::cast_precision_loss)]
    fn render(&self) -> Image {
        let row_step = self.screen.top_edge / self.screen.resolution_width as f32;
        let column_step = self.screen.left_edge / self.screen.resolution_height as f32;

        let mut image = Image::new(self.screen.resolution_width, self.screen.resolution_height);
        for y in 0..self.screen.resolution_height {
            for x in 0..self.screen.resolution_width {
                let pixel_position =
                    self.screen.top_left + row_step * x as f32 + column_step * y as f32;

                let ray = Ray::new(
                    self.camera.position,
                    (pixel_position - self.camera.position).normalize(),
                );

                if self.sphere.intersects(&ray).is_some() {
                    image.data.push(Pixel([255, 255, 255]));
                } else {
                    image.data.push(Pixel([0, 0, 0]));
                }
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
