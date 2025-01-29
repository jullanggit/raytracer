#![feature(file_buffered)]

use std::fs::File;
use std::io::Write as _;

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
        let mut file = File::create_buffered("target/out.ppm").unwrap();

        // Write ppm headers
        writeln!(&mut file, "P6").unwrap();
        writeln!(&mut file, "{} {} {}", self.width, self.height, 255).unwrap();

        for pixel in &self.data {
            file.write_all(&pixel.inner).unwrap();
        }

        file.flush().unwrap();
    }
    fn circle(radius: usize) -> Self {
        let diameter = radius * 2;
        let mut data = Vec::with_capacity(diameter * diameter * 3);

        for x in 0..diameter {
            for y in 0..diameter {
                let dx = x.abs_diff(radius);
                let dy = y.abs_diff(radius);

                if dx * dx + dy * dy < radius * radius {
                    data.push(Pixel::WHITE);
                } else {
                    data.push(Pixel::BLACK);
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

struct Pixel {
    inner: [u8; 3],
}
impl Pixel {
    const WHITE: Self = Self {
        inner: [u8::MAX; 3],
    };
    const BLACK: Self = Self { inner: [0; 3] };
}
