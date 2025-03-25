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
            MaterialKind::Metal { fuzziness } => {
                let direction = (*ray.direction.reflect(normal).inner()
                    + *NormalizedVec3::random().inner() * fuzziness)
                    .normalize();

                (direction.inner().dot(*normal.inner()) > 0.)
                    .then_some((Ray::new(hit_point, direction), self.color))
            }
            MaterialKind::Glass { refractive_index } => {
                let refracted_direction = ray.direction.refract(normal, refractive_index);

                Some((Ray::new(hit_point, refracted_direction), Color([1.; 3])))
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
