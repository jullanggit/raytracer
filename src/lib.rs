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
#![feature(generic_arg_infer)]
#![feature(generic_const_exprs)]
#![feature(f16)]
#![feature(f128)]
// TODO: Remove this when optimising
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::similar_names)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

pub mod bvh;
pub mod config;
pub mod cpu_affinity;
pub mod material;
pub mod mmap;
pub mod obj;
pub mod rng;
pub mod shapes;
pub mod vec3;

pub static SCENE: OnceLock<Scene> = OnceLock::new();

use crate::shapes::{Plane, Sphere};
use std::{
    array,
    io::{Write as _, stdout},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread::{self, available_parallelism},
};

use bvh::BvhNode;
use cpu_affinity::set_cpu_affinity;
use material::{Material, Scatter};
use mmap::{ColorChannel, MmapFile, Pixel};
use rng::Random as _;
use shapes::Triangle;
use vec3::{NormalizedVec3, ToFloatColor, ToNaturalColor as _, Vec3, Vector};

/// A ppm p6 image
pub struct Image {
    file: MmapFile,
    header_offset: usize,
}
impl Image {
    fn new(width: usize, height: usize) -> Self {
        let header = format!("P6\n{width} {height} {}\n", ColorChannel::MAX);
        let mut file = MmapFile::new(
            "target/out.ppm",
            header.len() + width * height * size_of::<Pixel>(),
        );

        file.as_slice_mut().write_all(header.as_bytes()).unwrap();

        Self {
            file,
            header_offset: header.len(),
        }
    }
    fn data(&mut self) -> &mut [Pixel] {
        // SAFETY:
        // - All bit patterns are valid Pixels
        unsafe { self.file.as_casted_slice_mut(self.header_offset) }
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
    continue_sampling: Option<usize>,
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
        continue_sampling: Option<usize>,
        screen: Screen,
        camera: Camera,
        bvhs: Bvhs,
        shapes: Shapes,
        materials: Vec<Material>,
    ) -> Self {
        Self {
            incremental,
            continue_sampling,
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

        let mut image = Image::new(self.screen.resolution_width, self.screen.resolution_height);
        let data = image.data();

        let num_threads: usize = available_parallelism().unwrap().into();

        #[expect(clippy::integer_division)]
        let chunk_size = (self.screen.resolution_width * self.screen.resolution_height)
            / (num_threads * num_threads);

        let chunks = data
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

        let cpu = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..num_threads {
                scope.spawn(|| {
                    // set cpu affinity
                    let cpu = cpu.fetch_add(1, Ordering::Relaxed);
                    set_cpu_affinity(cpu);

                    let mut bvh_stack = Vec::new();

                    loop {
                        let work_index = work_counter.fetch_add(1, Ordering::Relaxed);
                        if work_index >= total_work {
                            break;
                        }

                        #[expect(clippy::integer_division)]
                        let sample_iteration =
                            (work_index / chunks.len()) + self.continue_sampling.unwrap_or(0);
                        let chunk_index = work_index % chunks.len();

                        // report progress
                        if chunk_index == chunks.len() - 1 {
                            print!(
                                "\rSamples: {}",
                                self.continue_sampling.unwrap_or(0)
                                    + (sample_iteration + 1 - self.continue_sampling.unwrap_or(0))
                                        * sample_chunk_size
                            );
                            stdout().flush().unwrap();
                        }

                        let mut chunk = chunks[chunk_index].lock().unwrap();

                        // For every (x,y) pixel
                        for i in 0..chunk.len() {
                            // correct offset
                            let offset_i = chunk_index * chunk_size + i;

                            let x = offset_i % self.screen.resolution_width;
                            #[expect(clippy::integer_division)]
                            let y = offset_i / self.screen.resolution_width;

                            let color = Vector(
                                std::iter::repeat_with(|| {
                                    let pixel_position = self.screen.top_left
                                                + row_step * (x as f32 + f32::random() / 2.) // Add random variation
                                                + column_step * (y as f32 + f32::random() / 2.);

                                    let ray = Ray::new(
                                        self.camera.position,
                                        (pixel_position - self.camera.position).normalize(),
                                    );

                                    self.ray_color(ray, &self.materials, &mut bvh_stack)
                                })
                                .take(sample_chunk_size)
                                // average colors
                                .reduce(|acc, element| {
                                    Vector(array::from_fn(|index| {
                                        // by first adding them up
                                        acc.0[index] + element.0[index]
                                    }))
                                })
                                .unwrap_or_default()
                                .0
                                // and then dividing by samples
                                .map(|e| e / sample_chunk_size as f32),
                            );

                            if self.incremental.is_some() || self.continue_sampling.is_some() {
                                // average with last iteration
                                chunk[i] =
                                    ((ToFloatColor::<Vector<_, f32>>::to_float_color(chunk[i])
                                        * sample_iteration as f32
                                        + color.color_correct())
                                        / (sample_iteration as f32 + 1.))
                                        .to_natural_color();
                            } else {
                                chunk[i] = color.color_correct().to_natural_color();
                            }
                        }
                    }
                });
            }
        });
    }

    #[inline(always)]
    fn ray_color(
        &self,
        ray: Ray,
        materials: &[Material],
        bvh_stack: &mut Vec<(f32, u32)>, // is reused across shape types
    ) -> Vector<3, f32> {
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
                    let a = 0.5 * (current_ray.direction.y() + 1.0); // y scaled to 0.5-1

                    let current_color = current_color.get_or_insert(Vector([1.; 3]));
                    *current_color = *current_color
                        * (Vector([0.2, 0.2, 0.8]) * (1.0 - a) + Vector([1.; 3]) * a);

                    break;
                }
                // scattter
                Some((_, hit_point, (normal, texture_coordinates), shape_material_index)) => {
                    let shape_material = &materials[shape_material_index as usize];

                    match shape_material.scatter(&current_ray, normal, hit_point) {
                        Scatter::Scattered(ray, color) => {
                            // calculate color of scattered ray and mix it with the current color
                            let current_color = current_color.get_or_insert(Vector([1.; 3]));
                            *current_color = *current_color * color.sample(texture_coordinates);

                            current_ray = ray;
                        }
                        Scatter::Absorbed => {
                            current_color = Some(Vector([0.; 3]));
                            break;
                        }
                        Scatter::Light(color) => {
                            let current_color = current_color.get_or_insert(Vector([1.; 3]));
                            *current_color = *current_color * color.sample(texture_coordinates);
                            break;
                        }
                    }
                }
            }
        }

        current_color.unwrap_or(Vector([0.; 3]))
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
