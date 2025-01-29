use std::fs;

fn main() {
    let circle = Image::circle(30);
    circle.write_ppm_p6();
}

struct Image {
    width: u16,
    height: u16,

    data: Vec<Pixel>,
}
impl Image {
    fn write_ppm_p6(&self) {
        let mut buf = Vec::new();

        // Write ppm headers
        buf.extend_from_slice(b"P6\n");
        buf.extend_from_slice(format!("{} {} {}\n", self.width, self.height, 255).as_bytes());

        for pixel in &self.data {
            buf.extend_from_slice(&pixel.inner);
        }

        assert!(self.data.len() % self.width as usize == 0);

        fs::write("out.ppm", buf).unwrap();
    }
    fn circle(radius: u16) -> Self {
        let diameter = radius * 2;
        let mut data = Vec::with_capacity((diameter * diameter * 3) as usize);

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
    fn flatten_indices(&self, x: usize, y: usize) -> usize {
        x + y * self.width as usize
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
