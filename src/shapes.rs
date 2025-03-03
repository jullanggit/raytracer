use crate::{
    Color, Ray,
    vec3::{NormalizedVec3, Vec3},
};

pub trait Shape {
    /// The time at which the ray intersects the object
    fn intersects(&self, ray: &Ray) -> Option<f32>;

    /// Calculates the normal of a point on the shape's surface
    fn normal(&self, point: &Vec3) -> NormalizedVec3;

    /// The color of the shape
    fn color(&self) -> Color<f32>;
}

#[derive(PartialEq)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
    color: Color<f32>,
}
impl Sphere {
    pub const fn new(center: Vec3, radius: f32, color: Color<f32>) -> Self {
        Self {
            center,
            radius,
            color,
        }
    }
}
impl Shape for Sphere {
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

    fn normal(&self, point: &Vec3) -> NormalizedVec3 {
        (*point - self.center).normalize()
    }

    fn color(&self) -> Color<f32> {
        self.color
    }
}

pub struct Plane {
    point: Vec3,
    normal: NormalizedVec3,
    color: Color<f32>,
}

impl Plane {
    pub const fn new(point: Vec3, normal: NormalizedVec3, color: Color<f32>) -> Self {
        Self {
            point,
            normal,
            color,
        }
    }
}
impl Shape for Plane {
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let denominator = self.normal.inner().dot(*ray.direction.inner());

        if denominator.abs() < f32::EPSILON {
            return None; // Ray is parallel to the plane
        }

        let numerator = self.normal.inner().dot(ray.origin - self.point);

        let t = -(numerator / denominator);

        if t <= 0. {
            return None; // Intersection at or behind the ray's origin
        }

        Some(t)
    }

    fn normal(&self, _point: &Vec3) -> NormalizedVec3 {
        self.normal // The normal of a plane is the same at all points on it
    }

    fn color(&self) -> Color<f32> {
        self.color
    }
}
