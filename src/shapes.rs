use std::{array, f32, fmt::Debug};

use crate::{
    Ray,
    vec3::{NormalizedVec3, Vec3},
};

/// The min distance an intersection has to have for it to count
const MIN_DISTANCE: f32 = 0.001;

pub trait Intersects {
    /// The time at which the ray intersects the object
    fn intersects(&self, ray: &Ray) -> Option<f32>;
}

pub trait Shape: Intersects + Debug {
    /// Calculates the normal of a point on the shape's surface
    fn normal(&self, point: &Vec3) -> NormalizedVec3;

    fn material_index(&self) -> u16;

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
    material_index: u16,
}
impl Sphere {
    pub const fn new(center: Vec3, radius: f32, material_index: u16) -> Self {
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

        let delta_origin_direction = delta_origin.dot(*ray.direction.inner());
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
    fn normal(&self, point: &Vec3) -> NormalizedVec3 {
        (*point - self.center).normalize()
    }

    fn material_index(&self) -> u16 {
        self.material_index
    }

    fn centroid(&self) -> Vec3 {
        self.center
    }

    fn min(&self) -> Vec3 {
        self.center - Vec3::splat(self.radius)
    }

    fn max(&self) -> Vec3 {
        self.center + Vec3::splat(self.radius)
    }
}

#[derive(Debug)]
pub struct Plane {
    point: Vec3,
    normal: NormalizedVec3,
    material_index: u16,
}

impl Plane {
    pub const fn new(point: Vec3, normal: NormalizedVec3, material_index: u16) -> Self {
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
        let denominator = self.normal.inner().dot(*ray.direction.inner());

        if denominator.abs() < f32::EPSILON {
            return None; // Ray is parallel to the plane
        }

        let numerator = self.normal.inner().dot(ray.origin - self.point);

        let t = -(numerator / denominator);

        // Ensure intersection is in front of ray origin
        (t > MIN_DISTANCE).then_some(t)
    }
}
impl Shape for Plane {
    fn normal(&self, _point: &Vec3) -> NormalizedVec3 {
        self.normal // The normal of a plane is the same at all points on it
    }

    fn material_index(&self) -> u16 {
        self.material_index
    }

    fn centroid(&self) -> Vec3 {
        self.point
    }

    fn min(&self) -> Vec3 {
        Vec3::splat(f32::NEG_INFINITY)
    }

    fn max(&self) -> Vec3 {
        Vec3::splat(f32::INFINITY)
    }
}

#[derive(Debug)]
pub struct Triangle {
    a: Vec3,
    /// The edge from a to b
    e1: Vec3,
    /// The edge from a to c
    e2: Vec3,
    normals: [NormalizedVec3; 3], // TODO: maybe extract these, they're 1/2 the size of the whole triangle
    // 4 bytes smaller than adding an option around normals, due to alignment
    different_normals: bool,
    material_index: u16,
}
impl Triangle {
    pub fn new(
        a: Vec3,
        b: Vec3,
        c: Vec3,
        normals: [NormalizedVec3; 3],
        material_index: u16,
    ) -> Self {
        Self {
            a,
            e1: b - a,
            e2: c - a,
            different_normals: true,
            normals,
            material_index,
        }
    }
    /// Create a Triangle with Vertex normals set to the normal of the overall Triangle
    pub fn default_normals(a: Vec3, b: Vec3, c: Vec3, material_index: u16) -> Self {
        debug_assert!(a != b && a != c && b != c); // Triangle with two equal points

        let e1 = b - a;
        let e2 = c - a;

        Self {
            a,
            e1,
            e2,
            different_normals: false,
            normals: [e1.cross(e2).normalize(); 3],
            material_index,
        }
    }
    #[expect(clippy::suspicious_operation_groupings)] // clippy doesn't like d01 * d01
    fn barycentric_coordinates(&self, point: &Vec3) -> [f32; 3] {
        let ap = *point - self.a; // a -> p

        // Dot products
        // TODO: d00-d11 and the denominator can be precomputed
        let d00 = self.e1.dot(self.e1);
        let d01 = self.e1.dot(self.e2);
        let d11 = self.e2.dot(self.e2);
        let d20 = ap.dot(self.e1);
        let d21 = ap.dot(self.e2);

        // Barycentric coordinates
        let denominator = d00 * d11 - d01 * d01;
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

        let ray_cross_e2 = ray.direction.inner().cross(self.e2);
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
        let v = inv_det * ray.direction.inner().dot(s_cross_e1);
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
    fn normal(&self, point: &Vec3) -> NormalizedVec3 {
        if self.different_normals {
            let barycentric_coordinates = self.barycentric_coordinates(point);

            let weighted_normals: [_; 3] = array::from_fn(|index| {
                *self.normals[index].inner() * barycentric_coordinates[index]
            });

            (weighted_normals[0] + weighted_normals[1] + weighted_normals[2]).normalize()
        } else {
            self.normals[0]
        }
    }
    fn material_index(&self) -> u16 {
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
