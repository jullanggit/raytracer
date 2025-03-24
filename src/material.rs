use crate::{
    Color, Ray,
    vec3::{NormalizedVec3, Vec3},
};

#[derive(Debug, PartialEq)]
pub struct Material {
    kind: MaterialKind,
    color: Color<f32>,
}

impl Material {
    pub const fn new(color: Color<f32>) -> Self {
        Self {
            kind: MaterialKind::Lambertian,
            color,
        }
    }

    /// Returns the scattered ray, if it wasn't absorbed
    // TODO: see if we need to put color into this
    pub fn scatter(
        &self,
        ray: &Ray,
        normal: NormalizedVec3,
        hit_point: Vec3,
    ) -> Option<(Ray, Color<f32>)> {
        match self.kind {
            MaterialKind::Lambertian => {
                let direction = (normal + NormalizedVec3::random()).normalize();

                // Avoid division by zero etc.
                Some((
                    Ray::new(
                        hit_point,
                        if direction.near_zero() {
                            normal
                        } else {
                            direction
                        },
                    ),
                    self.color,
                ))
            }
            MaterialKind::Metal => {
                let direction = ray.direction.reflect(normal);

                Some((Ray::new(hit_point, direction), self.color))
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MaterialKind {
    Lambertian,
    Metal,
}
