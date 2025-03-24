use std::fs;

use crate::{Color, config::push_material, material::Material, shapes::Triangle, vec3::Vec3};

pub fn parse(path: &str, materials: &mut Vec<Material>) -> Vec<Triangle> {
    let string = fs::read_to_string(path).unwrap();
    let lines = string.lines();

    let vertices: Vec<Vec3> = lines
        .clone()
        .filter(|line| line.starts_with("v "))
        .map(|line| line[2..].trim().into())
        .collect();

    let normals: Vec<Vec3> = lines
        .clone()
        .filter(|line| line.starts_with("vn"))
        .map(|line| line[2..].trim().into())
        .collect();

    let index = push_material(Material::new(Color([0.5; 3])), materials);

    lines
        .filter(|line| line.starts_with('f'))
        .flat_map(|line| {
            let mut iter = line[1..].split_whitespace().map(|part| {
                let mut parts = part.splitn(3, '/');

                let vertex_index: usize = parts.next().unwrap().parse().unwrap();
                parts.next(); // skip texture
                let normal_index: usize = parts.next().unwrap().parse().unwrap();

                (vertices[vertex_index - 1], normals[normal_index - 1])
            });

            let first = iter.next().unwrap();

            iter.map_windows(move |vertices: &[_; 2]| {
                Triangle::new(
                    first.0,
                    vertices[0].0,
                    vertices[1].0,
                    [
                        first.1.normalize(),
                        vertices[0].1.normalize(),
                        vertices[1].1.normalize(),
                    ],
                    index,
                )
            })
        })
        .collect()
}
