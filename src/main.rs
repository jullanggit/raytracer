use std::{io::Write, mem::transmute};

fn main() {
    println!("Hello, world!");
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
    }
}

#[repr(C)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
}
