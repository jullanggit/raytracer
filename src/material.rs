use std::ops::Neg;

use crate::{
    Color, Ray, rng,
    vec3::{NormalizedVec3, Vec3},
};

#[derive(Debug, PartialEq)]
pub struct Material {
    kind: MaterialKind,
    color: Color<f32>,
}

impl Material {
    pub const fn new(kind: MaterialKind, color: Color<f32>) -> Self {
        Self { kind, color }
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

                Some((
                    Ray::new(
                        hit_point,
                        // Avoid division by zero etc.
                        if direction.near_zero() {
                            normal
                        } else {
                            direction
                        },
                    ),
                    self.color,
                ))
            }
            MaterialKind::Metal { fuzziness } => {
                let direction = ray.direction.reflect(normal); // reflection

                if fuzziness == 0.0 {
                    Some((Ray::new(hit_point, direction), self.color))
                } else {
                    // add fuzziness
                    let direction =
                        (*direction.inner() + NormalizedVec3::random() * fuzziness).normalize();

                    // Return None if the ray would end up in the object
                    (direction.dot(normal) > 0.)
                        .then_some((Ray::new(hit_point, direction), self.color))
                }
            }
            MaterialKind::Glass { refractive_index } => {
                // If it enters or exits the shape
                let (refractive_index, normal) = if ray.direction.dot(normal) < 0. {
                    (1. / refractive_index, normal)
                } else {
                    (refractive_index, -normal)
                };

                let cos = ray.direction.neg().dot(normal).min(1.);
                let sin = (1. - cos * cos).sqrt();

                let reflectance = {
                    let r0 = (1. - refractive_index) / (1. + refractive_index);
                    let r0 = r0 * r0;
                    r0 + (1. - r0) * (1. - cos).powi(5)
                };

                let direction = if refractive_index * sin < 1.0 || reflectance < rng::f32() {
                    // refract
                    let perpendicular = (*ray.direction.inner() + normal * cos) * refractive_index;
                    let discriminant = 1. - refractive_index * refractive_index * (1. - cos * cos);
                    let parallel = normal * -discriminant.sqrt();

                    let out = perpendicular + parallel;
                    debug_assert!(
                        (perpendicular + parallel).is_normalized(),
                        "vector: {out:?}, length: {:?}",
                        out.length()
                    );

                    NormalizedVec3::new(out)
                } else {
                    ray.direction.reflect(normal)
                };

                Some((Ray::new(hit_point, direction), Color([1.; 3])))
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MaterialKind {
    Lambertian,
    Metal { fuzziness: f32 },
    Glass { refractive_index: f32 },
}
impl From<&str> for MaterialKind {
    fn from(value: &str) -> Self {
        let mut split = value.split_whitespace();
        let kind = split.next().unwrap();

        match kind {
            "lambertian" => Self::Lambertian,
            "metal" => Self::Metal {
                fuzziness: split.next().unwrap().parse().unwrap(),
            },
            "glass" => Self::Glass {
                refractive_index: split.next().unwrap().parse().unwrap(),
            },
            other => panic!("Unknown material: {other}"),
        }
    }
}
