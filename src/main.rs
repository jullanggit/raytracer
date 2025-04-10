#![feature(let_chains)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(iter_map_windows)]
#![feature(iter_collect_into)]
#![feature(transmutability)]
#![feature(portable_simd)]
#![feature(iter_partition_in_place)]
#![feature(new_range_api)]
#![feature(substr_range)]
// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]

mod bvh;
mod config;
mod material;
mod obj;
mod rng;
mod shapes;
mod vec3;

use crate::shapes::{Plane, Sphere};
use std::{
    array,
    collections::BinaryHeap,
    fs::File,
    io::Write as _,
    mem::size_of,
    ops::{Add, Mul},
    slice,
    sync::Mutex,
    thread::{self, available_parallelism},
};

use bvh::{BvhNode, HeapEntry};
use material::{Material, Scatter};
use shapes::Triangle;
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
        let data = vec![Color([0; 3]); width * height]; // zero-initialise vec
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
// TODO: use Vec3 internally
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
impl Add for Color<f32> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(array::from_fn(|index| self.0[index] + rhs.0[index]))
    }
}
impl From<Color<f32>> for Color<u8> {
    #[expect(clippy::cast_possible_truncation)] // We check in debug mode
    #[expect(clippy::cast_sign_loss)]
    fn from(value: Color<f32>) -> Self {
        Self(value.0.map(|num| {
            debug_assert!((0.0..=1.).contains(&num));

            // gamma 2 correction
            (num.sqrt() * 255.) as u8
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

#[derive(Debug)]
struct Scene {
    screen: Screen,
    camera: Camera,
    shapes: Shapes,
    bvhs: Bvhs,
    materials: Vec<Material>,
}

#[derive(Debug)]
struct Shapes {
    spheres: Box<[Sphere]>,
    planes: Box<[Plane]>,
    triangles: Box<[Triangle]>,
}
impl Shapes {
    const fn new(spheres: Box<[Sphere]>, planes: Box<[Plane]>, triangles: Box<[Triangle]>) -> Self {
        Self {
            spheres,
            planes,
            triangles,
        }
    }
}

type BvhWrapper<T> = Box<[BvhNode<T>]>;

#[derive(Debug)]
struct Bvhs {
    spheres: BvhWrapper<Sphere>,
    planes: BvhWrapper<Plane>,
    triangles: BvhWrapper<Triangle>,
}
impl Bvhs {
    const fn new(
        spheres: BvhWrapper<Sphere>,
        planes: BvhWrapper<Plane>,
        triangles: BvhWrapper<Triangle>,
    ) -> Self {
        Self {
            spheres,
            planes,
            triangles,
        }
    }
}

impl Scene {
    const fn new(
        screen: Screen,
        camera: Camera,
        bvhs: Bvhs,
        shapes: Shapes,
        materials: Vec<Material>,
    ) -> Self {
        Self {
            screen,
            camera,
            shapes,
            bvhs,
            materials,
        }
    }

    // The only precision loss is turning the resolution into floats, which is fine
    #[expect(clippy::cast_precision_loss)]
    fn render(&self) -> Image {
        let row_step = self.screen.top_edge / (self.screen.resolution_width - 1) as f32;
        let column_step = self.screen.left_edge / (self.screen.resolution_height - 1) as f32;

        let mut image = Image::new(self.screen.resolution_width, self.screen.resolution_height);

        let num_threads: usize = available_parallelism().unwrap().into();

        let chunk_size = (self.screen.resolution_width * self.screen.resolution_height)
            / (num_threads * num_threads);

        let chunks = Mutex::new(image.data.chunks_mut(chunk_size).enumerate());

        thread::scope(|scope| {
            for _ in 0..num_threads {
                scope.spawn(|| {
                    let mut bvh_heap = BinaryHeap::new();

                    loop {
                        let mut next = chunks.lock().unwrap().next();

                        if let Some((chunk_index, ref mut chunk)) = next {
                            // For every (x,y) pixel
                            for i in 0..chunk.len() {
                                let offset_i = chunk_index * chunk_size + i; // correct offset

                                let x = offset_i % self.screen.resolution_width;
                                #[expect(clippy::integer_division)]
                                let y = offset_i / self.screen.resolution_width;

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

                                        self.ray_color(
                                            &ray,
                                            self.screen.max_bounces,
                                            &self.materials,
                                            &mut bvh_heap,
                                        )
                                    })
                                    .take(self.screen.samples_per_pixel)
                                    .reduce(|acc, element| {
                                        Color(array::from_fn(|index| {
                                            acc.0[index] + element.0[index]
                                        }))
                                    })
                                    .unwrap_or_default()
                                    .0
                                    .map(|e| e / self.screen.samples_per_pixel as f32),
                                );

                                chunk[i] = color.into();
                            }
                        } else {
                            break;
                        }
                    }
                });
            }
        });

        image
    }
    fn ray_color(
        &self,
        ray: &Ray,
        remaining_depth: usize,
        materials: &[Material],
        bvh_heap: &mut BinaryHeap<HeapEntry>, // is reused across shape types
    ) -> Color<f32> {
        if remaining_depth == 0 {
            return Color([0.; 3]);
        }

        let nearest_intersection =
            BvhNode::closest_shape(ray, &self.shapes.spheres, &self.bvhs.spheres, bvh_heap)
                .into_iter()
                .chain(BvhNode::closest_shape(
                    ray,
                    &self.shapes.planes,
                    &self.bvhs.planes,
                    bvh_heap,
                ))
                .chain(BvhNode::closest_shape(
                    ray,
                    &self.shapes.triangles,
                    &self.bvhs.triangles,
                    bvh_heap,
                ))
                .min_by(|&(a, ..), &(b, ..)| a.partial_cmp(&b).unwrap());

        nearest_intersection.map_or_else(
            || {
                let a = 0.5 * (ray.direction.inner().y + 1.0);

                Color([0.2, 0.2, 0.8]) * (1.0 - a) + Color([1.; 3]) * a
            },
            |(_, hit_point, normal, shape_material_index)| {
                let shape_material = &materials[shape_material_index as usize];

                match shape_material.scatter(ray, normal, hit_point) {
                    Scatter::Scattered(ray, attenuation) => {
                        // calculate color of scattered ray and mix it with the current color
                        attenuation * self.ray_color(&ray, remaining_depth - 1, materials, bvh_heap) // TODO: see if just multiplying the colors is right
                    }
                    Scatter::Absorbed => Color([0.; 3]),
                    Scatter::Light(color) => color,
                }
            },
        )
    }
}

#[derive(Debug)]
struct Screen {
    top_left: Vec3,
    top_edge: Vec3,
    left_edge: Vec3,
    resolution_width: usize,
    resolution_height: usize,
    samples_per_pixel: usize,
    max_bounces: usize,
}
impl Screen {
    const fn new(
        top_left: Vec3,
        top_edge: Vec3,
        left_edge: Vec3,
        resolution_width: usize,
        resolution_height: usize,
        samples_per_pixel: usize,
        max_bounces: usize,
    ) -> Self {
        Self {
            top_left,
            top_edge,
            left_edge,
            resolution_width,
            resolution_height,
            samples_per_pixel,
            max_bounces,
        }
    }
}

#[derive(Debug)]
struct Camera {
    position: Vec3,
}
impl Camera {
    const fn new(position: Vec3) -> Self {
        Self { position }
    }
}
