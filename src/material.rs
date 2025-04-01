use std::ops::Neg as _;

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

    /// Returns the scattered ray, if it wasn't absorbed or the light color
    pub fn scatter(&self, ray: &Ray, normal: NormalizedVec3, hit_point: Vec3) -> Scatter {
        let hit_point = hit_point + normal * 1e-4;

        // helper to create a Scattered with the given normal and direction
        let scattered = |normal: NormalizedVec3, direction: NormalizedVec3| -> Scatter {
            Scatter::Scattered(Ray::new(hit_point + normal * 1e-4, direction), self.color)
        };

        match self.kind {
            MaterialKind::Lambertian => {
                let direction = (normal + NormalizedVec3::random()).normalize();

                scattered(
                    normal,
                    // Avoid division by zero etc.
                    if direction.near_zero() {
                        normal
                    } else {
                        direction
                    },
                )
            }
            MaterialKind::Metal { fuzziness } => {
                // reflection
                let direction = ray.direction.reflect(normal);

                if fuzziness == 0.0 {
                    scattered(normal, direction)
                } else {
                    // add fuzziness
                    let direction =
                        (*direction.inner() + NormalizedVec3::random() * fuzziness).normalize();

                    // Return None if the ray would end up in the object
                    if direction.dot(normal) > 0. {
                        scattered(normal, direction)
                    } else {
                        Scatter::Absorbed
                    }
                }
            }
            MaterialKind::Glass { refractive_index } => {
                let outside = ray.direction.dot(normal) < 0.;

                // If it enters or exits the shape
                let (refractive_index, normal) = if outside {
                    (1. / refractive_index, normal)
                } else {
                    (refractive_index, -normal)
                };

                let cos = ray.direction.neg().dot(normal).min(1.);
                let sin = (1. - cos * cos).sqrt();

                // schlick approximation
                let reflectance = {
                    let r0 = (1. - refractive_index) / (1. + refractive_index);
                    let r0 = r0 * r0;
                    r0 + (1. - r0) * (1. - cos).powi(5)
                };

                if refractive_index * sin > 1.0 || rng::f32() < reflectance {
                    let direction = ray.direction.reflect(normal);
                    scattered(normal, direction)
                } else {
                    // refract
                    let perpendicular = (*ray.direction.inner() + normal * cos) * refractive_index;
                    let discriminant = 1. - refractive_index * refractive_index * (1. - cos * cos);
                    let parallel = normal * -discriminant.sqrt();

                    scattered(
                        if outside { -normal } else { normal }, // Offset into the sphere on enter
                        NormalizedVec3::new(perpendicular + parallel),
                    )
                }
            }
            MaterialKind::Light => Scatter::Light(self.color),
        }
    }
}

pub enum Scatter {
    Absorbed,
    Scattered(Ray, Color<f32>),
    Light(Color<f32>),
}

#[derive(Debug, PartialEq)]
pub enum MaterialKind {
    Lambertian,
    Metal { fuzziness: f32 },
    Glass { refractive_index: f32 },
    Light,
}
#[expect(clippy::fallible_impl_from)] // TODO: Remove once we care about crashes
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
            "light" => Self::Light,
            other => panic!("Unknown material: {other}"),
        }
    }
}
