use std::{fs, ops::Neg as _};

use crate::{
    Ray,
    indices::HasIndexer,
    mmap::Pixel,
    rng::Random as _,
    vec3::{Color, Lerp as _, New as _, NormalizedVector3, Point3},
};

#[derive(Debug, PartialEq)]
pub struct Material {
    kind: MaterialKind,
    color_kind: ColorKind,
}

impl Material {
    pub const fn new(kind: MaterialKind, color_kind: ColorKind) -> Self {
        Self { kind, color_kind }
    }

    /// Returns the scattered ray, if it wasn't absorbed or the light color
    pub fn scatter<'a>(
        &'a self,
        ray: &Ray,
        normal: NormalizedVector3,
        hit_point: Point3,
    ) -> Scatter<'a> {
        let hit_point = hit_point + normal.to_vector() * 1e-4;

        match self.kind {
            MaterialKind::Lambertian => {
                let direction = (normal + NormalizedVector3::random()).normalize::<f32>();

                Scatter::Scattered(
                    Ray::new(
                        hit_point,
                        // Avoid division by zero etc.
                        if direction.to_vector().near_zero() {
                            normal
                        } else {
                            direction
                        },
                    ),
                    &self.color_kind,
                )
            }
            MaterialKind::Metal { fuzziness } => {
                // reflection
                let direction = ray.direction.reflect(normal);

                if fuzziness == 0.0 {
                    Scatter::Scattered(Ray::new(hit_point, direction), &self.color_kind)
                } else {
                    // add fuzziness
                    let direction =
                        (direction + NormalizedVector3::random() * fuzziness).normalize();

                    // Return None if the ray would end up in the object
                    if direction.dot(normal) > 0. {
                        Scatter::Scattered(Ray::new(hit_point, direction), &self.color_kind)
                    } else {
                        Scatter::Absorbed
                    }
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

                // schlick approximation
                let reflectance = {
                    let r0 = (1. - refractive_index) / (1. + refractive_index);
                    let r0 = r0 * r0;
                    r0 + (1. - r0) * (1. - cos).powi(5)
                };

                let direction = if refractive_index * sin > 1.0 || f32::random() < reflectance {
                    ray.direction.reflect(normal)
                } else {
                    // refract
                    let perpendicular = (ray.direction + normal * cos) * refractive_index;
                    let discriminant = 1. - refractive_index * refractive_index * (1. - cos * cos);
                    let parallel = normal * -discriminant.sqrt();

                    NormalizedVector3::new(perpendicular + parallel)
                };

                Scatter::Scattered(Ray::new(hit_point, direction), &self.color_kind)
            }
            MaterialKind::Light => Scatter::Light(&self.color_kind),
        }
    }
}
impl HasIndexer for Material {
    // TODO: change back to u16 and figure out why Internet complains that it isnt usize
    type IndexerType = usize;
}

pub enum Scatter<'a> {
    Absorbed,
    Scattered(Ray, &'a ColorKind),
    Light(&'a ColorKind),
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

#[derive(Debug, PartialEq)]
pub enum ColorKind {
    Solid(Color<3, f32>),
    Texture {
        width: u32,
        height: u32,
        data: Box<[Pixel]>,
    },
}
impl ColorKind {
    pub fn texture_from_ppm_p6(file: &str) -> Self {
        let contents = fs::read(file).unwrap();

        assert_eq!(&contents[0..2], b"P6");

        let mut base = 3;
        let [width, height] = [b' ', b'\n'].map(|pat| {
            let length = contents[base..]
                .iter()
                .take_while(|&&byte| byte != pat)
                .count();

            let num = str::from_utf8(&contents[base..base + length])
                .unwrap()
                .parse()
                .unwrap();

            base += length + 1;

            num
        });

        assert_eq!(&contents[base..base + 3], b"255");

        // could be done with reinterpretation, but this is not performance critical
        let data: Box<[Pixel]> = contents
            .into_iter()
            .skip(base + 4)
            .array_chunks()
            .map(Color::new)
            .collect();

        assert_eq!(data.len(), width as usize * height as usize);

        Self::Texture {
            width,
            height,
            data,
        }
    }
    /// x & y: 0..=1
    #[expect(clippy::cast_precision_loss)]
    pub fn sample(&self, coords: [f32; 2]) -> Color<3, f32> {
        // tile
        let [x, y] = coords.map(|e: f32| e.fract().rem_euclid(1.));
        let y = 1. - y; // flip y-axis

        debug_assert!(
            coords.map(|e| (0.0..=1.).contains(&e)) == [true; 2],
            "{coords:?}"
        );

        match *self {
            Self::Solid(color) => color,
            // bilinear interpolation
            Self::Texture {
                width,
                height,
                ref data,
            } => {
                let [(x0, x1, dx), (y0, y1, dy)] = [(x, width), (y, height)].map(|(e, max)| {
                    // scale e
                    let e = e * (max - 1) as f32;

                    let e0f = e.floor();

                    // get pixels
                    // we check for valid range in debug mode
                    #[expect(clippy::cast_sign_loss)]
                    #[expect(clippy::cast_possible_truncation)]
                    let e0 = e0f as usize;
                    let e1 = (e0 + 1).min(max as usize - 1); // clamp to image space

                    // distance
                    let de = e - e0f;

                    (e0, e1, de)
                });

                let [c00, c01, c10, c11]: [Color<_, f32>; _] =
                    [[x0, y0], [x0, y1], [x1, y0], [x1, y1]]
                        .map(|[x, y]| data[x + y * width as usize].to_float_color::<f32>());

                let c0 = c00.lerp(c10, dx);
                let c1 = c01.lerp(c11, dx);

                c0.lerp(c1, dy)
            }
        }
    }
}
