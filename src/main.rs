use std::fs;

use raytracer::{SCENE, config};

fn main() {
    let scene = SCENE.get_or_init(|| config::parse(&fs::read_to_string("scene").unwrap()));
    scene.render();
}
