#![feature(file_buffered)]

use std::fs::File;
use std::io::Write as _;
use std::ops::Sub;
use std::slice;

fn main() {
    let circle = Image::circle(200);
    circle.write_ppm_p6();
}

struct Image {
    width: usize,
    height: usize,

    data: Vec<Pixel>,
}
impl Image {
    fn write_ppm_p6(&self) {
        let mut file = File::create("target/out.ppm").unwrap();

        // Write ppm header
        writeln!(&mut file, "P6\n{} {} 255", self.width, self.height).unwrap();

        // SAFETY:
        // - `Pixel` is a `repr(transparent)` wrapper around [u8;3],
        // - so `self.data` is effectively a &[[u8;3]]
        // - [u8;3] and u8 have the same alignment
        // - We adjust the length of the resulting slice
        file.write_all(unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast::<u8>(),
                self.data.len() * size_of::<Pixel>(),
            )
        })
        .unwrap();

        file.flush().unwrap();
    }
    #[expect(clippy::cast_precision_loss)]
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss)]
    fn circle(radius: usize) -> Self {
        let diameter = radius * 2;
        let mut data = Vec::with_capacity(diameter * diameter);

        let color_scale = 255. / diameter as f32;

        for x in 0..diameter {
            for y in 0..diameter {
                let dx = x.abs_diff(radius);
                let dy = y.abs_diff(radius);

                let color_x = (x as f32 * color_scale) as u8;
                let color_y = (y as f32 * color_scale) as u8;
                if dx * dx + dy * dy < radius * radius {
                    data.push(Pixel([color_x, 0, color_y]));
                } else {
                    data.push(Pixel([color_y, 0, color_x]));
                }
            }
        }

        Self {
            width: diameter,
            height: diameter,
            data,
        }
    }
    const fn flatten_indices(&self, x: usize, y: usize) -> usize {
        x + y * self.width
    }
}

#[repr(transparent)]
struct Pixel([u8; 3]);

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

struct Sphere {
    center: Vec3,
    radius: f32,
}

struct Ray {
    origin: Vec3,
    // A normalized Vec3
    direction: Vec3,
}

trait Intersects {
    /// The scale at which the ray intersects the object
    fn intersects(&self, ray: &Ray) -> Option<f32>;
}
impl Intersects for Sphere {
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let origin_to_center = ray.origin - self.center;

        // The direction is normalized, so a=1
        let b = origin_to_center.dot(ray.direction);
        let c = origin_to_center.dot(origin_to_center) - self.radius * self.radius;

        let discriminant = 4.0 * (b * b - c);

        if discriminant < 0.0 {
            return None; // No solution to quadratic formula
        }

        // The first intersection point
        let t1 = -b - discriminant.sqrt();

        // If t1 is positive (in front of the origin), return it
        // t1 is always <= t2, because we subtract, instead of add the discriminant (always positive)
        if t1 > 0.0 {
            Some(t1)
        } else {
            // The second intersection point
            let t2 = -b + discriminant.sqrt();

            if t2 > 0.0 { Some(t2) } else { None }
        }
    }
}
