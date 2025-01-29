#![feature(file_buffered)]

use std::fs::File;
use std::io::Write as _;
use std::mem::transmute;

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
        // Pixel is just a [u8;3], so &self.data is a &[[u8;3]], which can be safely flattened to a &[u8]
        file.write_all(unsafe { transmute::<&[Pixel], &[u8]>(&self.data) }) // For some reason not yet supported by TransmuteFrom
            .unwrap();

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
