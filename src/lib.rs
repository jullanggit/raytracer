#![feature(super_let)]
#![feature(let_chains)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(iter_map_windows)]
#![feature(iter_collect_into)]
#![feature(transmutability)]
#![feature(portable_simd)]
#![feature(iter_partition_in_place)]
#![feature(new_range_api)]
#![feature(substr_range)]
#![feature(iter_array_chunks)]
// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::similar_names)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

pub mod bvh;
pub mod config;
pub mod material;
pub mod obj;
pub mod rng;
pub mod shapes;
pub mod vec3;

pub static SCENE: OnceLock<Scene> = OnceLock::new();

use crate::shapes::{Plane, Sphere};
use std::{
    array,
    fs::File,
    io::{Seek as _, Write as _},
    ops::{Add, Div, Mul, MulAssign},
    os::unix::fs::FileExt as _,
    slice,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, available_parallelism},
};

use bvh::BvhNode;
use material::{Material, Scatter};
use shapes::Triangle;
use vec3::{NormalizedVec3, Vec3};

/// A ppm p6 image
pub struct Image {
    file: File,
    data: Vec<Color<u8>>,
}
impl Image {
    /// (Self, header end offset)
    fn new(width: usize, height: usize) -> (Self, u64) {
        let mut file = File::create("target/out.ppm").unwrap();
        // Write ppm header
        writeln!(&mut file, "P6\n{width} {height} 255").unwrap();

        let data = vec![Color([0; 3]); width * height]; // zero-initialise vec

        let position = file.stream_position().unwrap();

        (Self { file, data }, position)
    }
    pub fn write(&mut self) {
        self.file.write_all(Color::as_bytes(&self.data)).unwrap();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
/// A rgb color
/// for Color<f32> values should be between 0 and 1
pub struct Color<T>([T; 3]);

impl Color<f32> {
    fn color_correct(self) -> Self {
        // gamma 2 correction
        Self(self.0.map(f32::sqrt))
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        Self(array::from_fn(|i| self.0[i] * (1. - t) + other.0[i] * t))
    }
}

impl Color<u8> {
    const fn as_bytes(slice: &[Self]) -> &[u8] {
        // SAFETY:
        // - `Self` is a `repr(transparent)` wrapper around [u8;3],
        // - so `self.data` is effectively a &[[u8;3]]
        // - [u8;3] and u8 have the same alignment
        // - We adjust the length of the resulting slice
        unsafe { slice::from_raw_parts(slice.as_ptr().cast::<u8>(), size_of_val(slice)) }
    }
}

impl Mul for Color<f32> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(array::from_fn(|index| self.0[index] * rhs.0[index]))
    }
}
impl MulAssign for Color<f32> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = Self(array::from_fn(|index| self.0[index] * rhs.0[index]));
    }
}
impl Mul<f32> for Color<f32> {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0.map(|num| num * rhs))
    }
}
impl Div<f32> for Color<f32> {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0.map(|num| num / rhs))
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

            (num * 255.) as u8
        }))
    }
}
impl From<Color<u8>> for Color<f32> {
    fn from(value: Color<u8>) -> Self {
        Self(value.0.map(f32::from).map(|e| e / 255.))
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
pub struct Ray {
    origin: Vec3,
    direction: NormalizedVec3,
}
impl Ray {
    const fn new(origin: Vec3, direction: NormalizedVec3) -> Self {
        Self { origin, direction }
    }
}

#[derive(Debug)]
pub struct Scene {
    incremental: Option<usize>,
    screen: Screen,
    camera: Camera,
    shapes: Shapes,
    bvhs: Bvhs,
    materials: Vec<Material>,
}

#[derive(Debug)]
pub struct Shapes {
    spheres: Box<[Sphere]>,
    planes: Box<[Plane]>,
    triangles: Box<[Triangle]>,
    vertex_normals: Box<[[NormalizedVec3; 3]]>,
    texture_coordinates: Box<[[[f32; 2]; 3]]>,
    /// [d00, d01, d11, denominator]
    barycentric_precomputed: Box<[[f32; 4]]>,
}
impl Shapes {
    const fn new(
        spheres: Box<[Sphere]>,
        planes: Box<[Plane]>,
        triangles: Box<[Triangle]>,
        vertex_normals: Box<[[NormalizedVec3; 3]]>,
        texture_coordinates: Box<[[[f32; 2]; 3]]>,
        barycentric_precomputed: Box<[[f32; 4]]>,
    ) -> Self {
        Self {
            spheres,
            planes,
            triangles,
            vertex_normals,
            texture_coordinates,
            barycentric_precomputed,
        }
    }
}

type BvhWrapper<T> = Box<[BvhNode<T>]>;

#[derive(Debug)]
pub struct Bvhs {
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
        incremental: Option<usize>,
        screen: Screen,
        camera: Camera,
        bvhs: Bvhs,
        shapes: Shapes,
        materials: Vec<Material>,
    ) -> Self {
        Self {
            incremental,
            screen,
            camera,
            shapes,
            bvhs,
            materials,
        }
    }

    // The only precision loss is turning the resolution into floats, which is fine
    #[expect(clippy::cast_precision_loss)]
    pub fn render(&self) {
        let row_step = self.screen.top_edge / (self.screen.resolution_width - 1) as f32;
        let column_step = self.screen.left_edge / (self.screen.resolution_height - 1) as f32;

        let (mut image, offset) =
            Image::new(self.screen.resolution_width, self.screen.resolution_height);

        let num_threads: usize = available_parallelism().unwrap().into();

        #[expect(clippy::integer_division)]
        let chunk_size = (self.screen.resolution_width * self.screen.resolution_height)
            / (num_threads * num_threads);

        let chunks = image
            .data
            .chunks_mut(chunk_size)
            .map(Mutex::new)
            .collect::<Vec<_>>();

        // the amount of samples to perform at once
        let sample_chunk_size = self.incremental.unwrap_or(self.screen.samples_per_pixel);

        // this division is always clean as we assert that incremental cleanly divides samples per pixel
        #[expect(clippy::integer_division)]
        let num_sample_chunks = self.screen.samples_per_pixel / sample_chunk_size;

        let total_work = chunks.len() * num_sample_chunks;
        let work_counter = AtomicUsize::new(0);

        thread::scope(|scope| {
            for _ in 0..num_threads {
                scope.spawn(|| {
                    let mut bvh_stack = Vec::new();

                    loop {
                        let work_index = work_counter.fetch_add(1, Ordering::Relaxed);
                        if work_index >= total_work {
                            break;
                        }

                        #[expect(clippy::integer_division)]
                        let sample_iteration = work_index / chunks.len();
                        let chunk_index = work_index % chunks.len();

                        let mut chunk = chunks[chunk_index].lock().unwrap();

                        // For every (x,y) pixel
                        for i in 0..chunk.len() {
                            // correct offset
                            let offset_i = chunk_index * chunk_size + i;

                            let x = offset_i % self.screen.resolution_width;
                            #[expect(clippy::integer_division)]
                            let y = offset_i / self.screen.resolution_width;

                            let color = Color(
                                std::iter::repeat_with(|| {
                                    let pixel_position = self.screen.top_left
                                                + row_step * (x as f32 + rng::f32() / 2.) // Add random variation
                                                + column_step * (y as f32 + rng::f32() / 2.);

                                    let ray = Ray::new(
                                        self.camera.position,
                                        (pixel_position - self.camera.position).normalize(),
                                    );

                                    self.ray_color(ray, &self.materials, &mut bvh_stack)
                                })
                                .take(sample_chunk_size)
                                // average colors
                                .reduce(|acc, element| {
                                    Color(array::from_fn(|index| {
                                        // by first adding them up
                                        acc.0[index] + element.0[index]
                                    }))
                                })
                                .unwrap_or_default()
                                .0
                                // and then dividing by samples
                                .map(|e| e / sample_chunk_size as f32),
                            );

                            if self.incremental.is_some() {
                                // average with last incremental iteration
                                chunk[i] = ((Color::<f32>::from(chunk[i])
                                    * sample_iteration as f32
                                    + color.color_correct())
                                    / (sample_iteration as f32 + 1.))
                                    .into();
                            } else {
                                chunk[i] = color.color_correct().into();
                            }
                        }

                        if self.incremental.is_some() {
                            let written = image
                                .file
                                .write_at(
                                    Color::as_bytes(*chunk),
                                    offset
                                        + chunk_index as u64
                                            * chunk_size as u64
                                            * size_of::<Color<u8>>() as u64,
                                )
                                .unwrap();

                            assert!(
                                written == size_of_val(*chunk),
                                "written: {written}, chunk len: {}",
                                chunk.len()
                            );
                        }
                    }
                });
            }
        });

        if self.incremental.is_none() {
            image.write();
        }

        image.file.flush().unwrap();
    }

    #[inline(always)]
    fn ray_color(
        &self,
        ray: Ray,
        materials: &[Material],
        bvh_stack: &mut Vec<(f32, u32)>, // is reused across shape types
    ) -> Color<f32> {
        let mut current_ray = ray;
        let mut current_color = None;

        for _ in 0..self.screen.max_bounces {
            let nearest_intersection = BvhNode::closest_shape(
                &current_ray,
                &self.shapes.spheres,
                &self.bvhs.spheres,
                bvh_stack,
            )
            .into_iter()
            .chain(BvhNode::closest_shape(
                &current_ray,
                &self.shapes.planes,
                &self.bvhs.planes,
                bvh_stack,
            ))
            .chain(BvhNode::closest_shape(
                &current_ray,
                &self.shapes.triangles,
                &self.bvhs.triangles,
                bvh_stack,
            ))
            .min_by(|&(a, ..), &(b, ..)| a.partial_cmp(&b).unwrap());

            match nearest_intersection {
                // skybox
                None => {
                    let a = 0.5 * (current_ray.direction.inner().y + 1.0); // y scaled to 0.5-1

                    *current_color.get_or_insert(Color([1.; 3])) *=
                        Color([0.2, 0.2, 0.8]) * (1.0 - a) + Color([1.; 3]) * a;

                    break;
                }
                // scattter
                Some((_, hit_point, (normal, texture_coordinates), shape_material_index)) => {
                    let shape_material = &materials[shape_material_index as usize];

                    match shape_material.scatter(&current_ray, normal, hit_point) {
                        Scatter::Scattered(ray, color) => {
                            // calculate color of scattered ray and mix it with the current color
                            *current_color.get_or_insert(Color([1.; 3])) *=
                                color.sample(texture_coordinates);

                            current_ray = ray;
                        }
                        Scatter::Absorbed => {
                            current_color = Some(Color([0.; 3]));
                            break;
                        }
                        Scatter::Light(color) => {
                            *current_color.get_or_insert(Color([1.; 3])) *=
                                color.sample(texture_coordinates);
                            break;
                        }
                    }
                }
            }
        }

        current_color.unwrap_or(Color([0.; 3]))
    }
}

#[derive(Debug)]
pub struct Screen {
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
pub struct Camera {
    position: Vec3,
}
impl Camera {
    const fn new(position: Vec3) -> Self {
        Self { position }
    }
}
