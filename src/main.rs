#![feature(let_chains)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(iter_map_windows)]
#![feature(transmutability)]
#![feature(portable_simd)]
// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]

mod config;
mod obj;
mod rng;
mod shapes;
mod vec3;

use crate::shapes::{Plane, Sphere};
use std::{array, fs::File, io::Write as _, mem::size_of, ops::Mul, slice};

use shapes::{Shape, Triangle};
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
            debug_assert!((0.0..=1.).contains(&num));

            (num * 255.) as u8
        }))
    }
}
#[expect(clippy::fallible_impl_from)] // TODO: Remove once we care about crashes
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

struct Scene {
    screen: Screen,
    camera: Camera,
    spheres: Vec<Sphere>,
    planes: Vec<Plane>,
    triangles: Vec<Triangle>,
    light: Light,
}

impl Scene {
    // The only precision loss is turning the resolution into floats, which is fine
    #[expect(clippy::cast_precision_loss)]
    fn render(&self) -> Image {
        // Helper functions for iterating over the different shapes
        fn smallest_shape_intersection<'a, S: Shape + 'a>(
            iter: impl IntoIterator<Item = &'a S>,
            ray: &Ray,
        ) -> Option<(f32, Vec3, NormalizedVec3, Color<f32>)> {
            iter.into_iter()
                .filter_map(|shape| shape.intersects(ray).map(|time| (shape, time)))
                .min_by(|&(_, time1), &(_, time2)| {
                    time1
                        .partial_cmp(&time2)
                        .expect("Ordering between times should exist")
                })
                .map(|(shape, time)| {
                    let hit_point = ray.origin + *ray.direction.inner() * time;

                    (time, hit_point, shape.normal(&hit_point), shape.color())
                })
        }
        fn is_occluded<'a, S: Shape + 'a>(
            iter: impl IntoIterator<Item = &'a S>,
            light_ray: &Ray,
        ) -> bool {
            iter.into_iter()
                .filter(|shape| shape.intersects(light_ray).is_some())
                .nth(1) // allow one intersection with the object itself
                .is_some()
        }

        let row_step = self.screen.top_edge / (self.screen.resolution_width - 1) as f32;
        let column_step = self.screen.left_edge / (self.screen.resolution_height - 1) as f32;

        let mut image = Image::new(self.screen.resolution_width, self.screen.resolution_height);

        // For every (x,y) pixel
        for y in 0..self.screen.resolution_height {
            for x in 0..self.screen.resolution_width {
                // Multiple samples
                let color = Color(
                    std::iter::repeat_with(|| {
                        let pixel_position = self.screen.top_left
                            + row_step * (x as f32 + rng::f32() / 2.) // Add random variation
                            + column_step * (y as f32 + rng::f32() / 2.);

                        let ray = Ray::new(
                            self.camera.position,
                            (pixel_position - self.camera.position).normalize(),
                        );

                        if let Some((_, hit_point, normal, color)) =
                            smallest_shape_intersection(&self.spheres, &ray)
                                .into_iter()
                                .chain(smallest_shape_intersection(&self.planes, &ray))
                                .chain(smallest_shape_intersection(&self.triangles, &ray))
                                .min_by(|&(a, _, _, _), &(b, _, _, _)| a.partial_cmp(&b).unwrap())
                        {
                            let light_direction = (self.light.position - hit_point).normalize();
                            let light_ray = Ray::new(hit_point, light_direction);

                            // If the ray to the light source is occluded by any other shape
                            if is_occluded(&self.spheres, &light_ray)
                                || is_occluded(&self.planes, &light_ray)
                                || is_occluded(&self.triangles, &light_ray)
                            {
                                Color::default()
                            } else {
                                // How straight the light is falling on the surface
                                let color_coefficient =
                                    light_direction.inner().dot(*normal.inner()).max(0.); // Can maybe be optimised to not consider cases where the normal points away from the light

                                self.light.color * color * color_coefficient
                            }
                        } else {
                            Color::default()
                        }
                    })
                    .take(self.screen.samples_per_pixel)
                    .reduce(|acc, element| {
                        Color(array::from_fn(|index| acc.0[index] + element.0[index]))
                    })
                    .unwrap_or_default()
                    .0
                    .map(|e| e / self.screen.samples_per_pixel as f32),
                );

                image.data.push(color.into());
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
    samples_per_pixel: usize,
}
impl Screen {
    const fn new(
        top_left: Vec3,
        top_edge: Vec3,
        left_edge: Vec3,
        resolution_width: usize,
        resolution_height: usize,
        samples_per_pixel: usize,
    ) -> Self {
        Self {
            top_left,
            top_edge,
            left_edge,
            resolution_width,
            resolution_height,
            samples_per_pixel,
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
