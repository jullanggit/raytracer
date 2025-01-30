#![feature(file_buffered)]

use std::fs::File;
use std::io::Write as _;
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
