use std::{
    array,
    f32::{
        self,
        consts::{PI, TAU},
    },
    fmt::Debug,
};

use crate::{
    Ray, SCENE,
    indices::HasIndexer,
    indices::Indexer,
    material::Material,
    vec3::{Vector, NormalizedVec3, Vec3},
};

/// The min distance an intersection has to have for it to count
const MIN_DISTANCE: f32 = 0.001;

pub type MaterialIndexer =
    Indexer<<Material as HasIndexer>::IndexerType, <Material as HasIndexer>::Data>;

pub trait Intersects {
    /// The time at which the ray intersects the object
    fn intersects(&self, ray: &Ray) -> Option<f32>;
}

pub trait Shape: Intersects + Debug {
    /// Calculates the normal of a point on the shape's surface
    fn normal_and_texture_coordinates(&self, point: &Vec3) -> (NormalizedVec3, [f32; 2]);

    fn material_index(&self) -> MaterialIndexer;

    // BVH
    fn centroid(&self) -> Vec3;
    /// The minimum point of the AABB enclosing the shape
    fn min(&self) -> Vec3;
    /// The maximum point of the AABB enclosing the shape
    fn max(&self) -> Vec3;
}

#[derive(Debug, PartialEq)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
    material_index: MaterialIndexer,
}
impl Sphere {
    pub const fn new(center: Vec3, radius: f32, material_index: MaterialIndexer) -> Self {
        Self {
            center,
            radius,
            material_index,
        }
    }
}
impl Intersects for Sphere {
    // See `ray_sphere_intersection_derivation.latex` for the formula used here
    #[inline(always)]
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let delta_origin = ray.origin - self.center;

        let delta_origin_direction = delta_origin.dot(*ray.direction);
        let discriminant = delta_origin_direction * delta_origin_direction
            - delta_origin.dot(delta_origin)
            + self.radius * self.radius;

        if discriminant < 0. {
            return None; // No solution to quadratic formula
        }

        // The first intersection point
        let t1 = -delta_origin_direction - discriminant.sqrt();

        // If t1 is positive (in front of the origin), return it, as
        // t1 is always closer than t2, because we subtract,
        // instead of add the discriminant (which is always positive)
        if t1 > MIN_DISTANCE {
            Some(t1)
        } else {
            // The second intersection point
            let t2 = -delta_origin_direction + discriminant.sqrt();

            // If t2 is positive, return it, else None
            (t2 > MIN_DISTANCE).then_some(t2)
        }
    }
}
impl Shape for Sphere {
    // uses spherical mapping for texture coordinates
    fn normal_and_texture_coordinates(&self, point: &Vec3) -> (NormalizedVec3, [f32; 2]) {
        (
            (*point - self.center).normalize::<f32>(),
            [
                0.5 + point.z().atan2(point.x()) / TAU,
                0.5 - point.y().asin() / PI,
            ],
        )
    }

    fn material_index(&self) -> MaterialIndexer {
        self.material_index
    }

    fn centroid(&self) -> Vec3 {
        self.center
    }

    fn min(&self) -> Vec3 {
        self.center - Vector::new([self.radius; _])
    }

    fn max(&self) -> Vec3 {
        self.center + Vector::new([self.radius; _])
    }
}

#[derive(Debug)]
pub struct Plane {
    point: Vec3,
    normal: NormalizedVec3,
    material_index: MaterialIndexer,
}

impl Plane {
    pub const fn new(point: Vec3, normal: NormalizedVec3, material_index: MaterialIndexer) -> Self {
        Self {
            point,
            normal,
            material_index,
        }
    }
}

impl Intersects for Plane {
    #[inline(always)]
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let denominator = self.normal.dot(*ray.direction);

        if denominator.abs() < f32::EPSILON {
            return None; // Ray is parallel to the plane
        }

        let numerator = self.normal.dot(ray.origin - self.point);

        let t = -(numerator / denominator);

        // Ensure intersection is in front of ray origin
        (t > MIN_DISTANCE).then_some(t)
    }
}
impl Shape for Plane {
    fn normal_and_texture_coordinates(&self, point: &Vec3) -> (NormalizedVec3, [f32; 2]) {
        let delta = *point - self.point;
        (
            // The normal of a plane is the same at all points on it
            self.normal,
            // tile after 5 units
            [delta.x() % 5., delta.y() % 5.],
        )
    }

    fn material_index(&self) -> MaterialIndexer {
        self.material_index
    }

    fn centroid(&self) -> Vec3 {
        self.point
    }

    fn min(&self) -> Vec3 {
        Vector::new([f32::NEG_INFINITY; 3])
    }

    fn max(&self) -> Vec3 {
        Vector::new([f32::INFINITY; 3])
    }
}

#[derive(Debug)]
pub struct Triangle {
    a: Vec3,
    /// The edge from a to b
    e1: Vec3,
    /// The edge from a to c
    e2: Vec3,
    normals_texture_coordinates: NormalsTextureCoordinates,
    material_index: MaterialIndexer,
}
impl Triangle {
    pub fn new(
        a: Vec3,
        b: Vec3,
        c: Vec3,
        normals_texture_coordinates: NormalsTextureCoordinates,
        material_index: MaterialIndexer,
    ) -> Self {
        Self {
            a,
            e1: b - a,
            e2: c - a,
            normals_texture_coordinates,
            material_index,
        }
    }
    fn barycentric_coordinates(
        &self,
        point: &Vec3,
        [d00, d01, d11, denominator]: [f32; 4],
    ) -> [f32; 3] {
        let ap = *point - self.a; // a -> p

        // Dot products
        let d20 = ap.dot(self.e1);
        let d21 = ap.dot(self.e2);

        // Barycentric coordinates
        let v = (d11 * d20 - d01 * d21) / denominator;
        let w = (d00 * d21 - d01 * d20) / denominator;
        let u = 1. - v - w;

        [u, v, w]
    }
}
impl Intersects for Triangle {
    // Möller-Trumbore intersection algorithm
    #[inline(always)]
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        // Using f32::EPSILON causes some slight edge misalignments (coming from the naive triangulation) to become visible
        const TOLERANCE: f32 = 2e-6;

        let ray_cross_e2 = ray.direction.cross(self.e2);
        let det = self.e1.dot(ray_cross_e2);

        if det.abs() < TOLERANCE {
            return None; // Ray is parallel to triangle.
        }

        let inv_det = 1.0 / det;
        let s = ray.origin - self.a;
        let u = inv_det * s.dot(ray_cross_e2);
        if !(0.0..=1.).contains(&u) {
            return None;
        }

        let s_cross_e1 = s.cross(self.e1);
        let v = inv_det * ray.direction.dot(s_cross_e1);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = inv_det * self.e2.dot(s_cross_e1);

        // Ensure intersection is in front of ray origin
        (t > MIN_DISTANCE).then_some(t)
    }
}
impl Shape for Triangle {
    #[inline(always)]
    #[expect(clippy::wildcard_enum_match_arm)]
    fn normal_and_texture_coordinates(&self, point: &Vec3) -> (NormalizedVec3, [f32; 2]) {
        use NormalsTextureCoordinates::{Both, None, Normals, TextureCoordinates};

        let default_normal = || self.e1.cross(self.e2).normalize::<f32>();

        let scene = SCENE.get().unwrap();
        let barycentric_coordinates = match self.normals_texture_coordinates {
            Both {
                barycentric_precomputed_index,
                ..
            }
            | Normals {
                barycentric_precomputed_index,
                ..
            }
            | TextureCoordinates {
                barycentric_precomputed_index,
                ..
            } => self.barycentric_coordinates(
                point,
                *barycentric_precomputed_index.index(&*scene.shapes.barycentric_precomputed),
            ),
            None => return (default_normal(), Default::default()), // triangles with textures should also have texture coordinates
        };

        let normal = match self.normals_texture_coordinates {
            Both { normals_index, .. } | Normals { normals_index, .. } => {
                let normals = normals_index.index(&*scene.shapes.vertex_normals);

                let weighted_normals: [_; 3] =
                    array::from_fn(|index| *normals[index] * barycentric_coordinates[index]);

                (weighted_normals[0] + weighted_normals[1] + weighted_normals[2]).normalize()
            }
            _ => default_normal(),
        };
        let texture_coordinates = match self.normals_texture_coordinates {
            Both {
                texture_coordinates_index,
                ..
            }
            | TextureCoordinates {
                texture_coordinates_index,
                ..
            } => {
                let texture_coordinates =
                    texture_coordinates_index.index(&*scene.shapes.texture_coordinates);

                let weighted_texture_coordinates: [_; 3] = array::from_fn(|index| {
                    texture_coordinates[index].map(|e| e * barycentric_coordinates[index])
                });

                array::from_fn(|index| {
                    weighted_texture_coordinates[0][index]
                        + weighted_texture_coordinates[1][index]
                        + weighted_texture_coordinates[2][index]
                })
            }
            _ => Default::default(),
        };

        (normal, texture_coordinates)
    }
    fn material_index(&self) -> MaterialIndexer {
        self.material_index
    }

    fn centroid(&self) -> Vec3 {
        self.a + (self.e1 + self.e2) / 3.
    }

    fn min(&self) -> Vec3 {
        let b = self.a + self.e1;
        let c = self.a + self.e2;

        self.a.min(b).min(c)
    }

    fn max(&self) -> Vec3 {
        let b = self.a + self.e1;
        let c = self.a + self.e2;

        self.a.max(b).max(c)
    }
}

type NormalsIndexer = Indexer<u32, [NormalizedVec3; 3]>;
type TextureCoordinatesIndexer = Indexer<u32, [[f32; 2]; 3]>;
type BarycentricPrecomputedIndexer = Indexer<u32, [f32; 4]>;

#[derive(Debug, PartialEq)]
pub enum NormalsTextureCoordinates {
    Both {
        normals_index: NormalsIndexer,
        texture_coordinates_index: TextureCoordinatesIndexer,
        barycentric_precomputed_index: BarycentricPrecomputedIndexer,
    },
    Normals {
        normals_index: NormalsIndexer,
        barycentric_precomputed_index: BarycentricPrecomputedIndexer,
    },
    TextureCoordinates {
        texture_coordinates_index: TextureCoordinatesIndexer,
        barycentric_precomputed_index: BarycentricPrecomputedIndexer,
    },
    None,
}
