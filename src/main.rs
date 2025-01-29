use std::{fs, io::Write, mem::transmute};

fn main() {
    let circle = Image::circle(30);
    circle.write();
}

struct Image {
    width: u16,
    height: u16,

    data: Vec<Pixel>,
}
impl Image {
    fn write(&self) {
        let mut buf = Vec::new();

        writeln!(&mut buf, "P6").unwrap();
        writeln!(&mut buf, "{} {}", self.width, self.height).unwrap();
        buf.write_all(unsafe { transmute::<&[Pixel], &[u8]>(&self.data) }) // For some reason not yet supported by TransmuteFrom
            .unwrap();

        fs::write("out.ppm", buf).unwrap();
    }
    fn circle(radius: u16) -> Self {
        let diameter = radius * 2;
        let mut data = Vec::with_capacity((diameter * diameter * 3) as usize);

        for x in 0..diameter {
            for y in 0..diameter {
                if x * x + y * y < radius * radius {
                    data.push(Pixel::WHITE);
                } else {
                    data.push(Pixel::BLACK)
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

#[repr(C)]
struct Pixel {
    inner: [u8; 3],
}
impl Pixel {
    const WHITE: Self = Self {
        inner: [u8::MAX; 3],
    };
    const BLACK: Self = Self { inner: [0; 3] };
}
