use std::fs;

use crate::{Color, shapes::Triangle, vec3::Vec3};

pub fn parse(path: &str) -> Vec<Triangle> {
    let string = fs::read_to_string(path).unwrap();
    let lines = string.lines();

    let vertices = lines
        .clone()
        .filter(|line| line.starts_with("v "))
        .map(|line| line[2..].trim().into());

    let normals = lines
        .clone()
        .filter(|line| line.starts_with("vn"))
        .map(|line| line[2..].trim().into());

    let combined: Vec<(Vec3, Vec3)> = vertices.zip(normals).collect();

    lines
        .filter(|line| line.starts_with('f'))
        .flat_map(|line| {
            let mut iter = line[1..].split_whitespace().map(|vertex| {
                combined[vertex.split_once('/').unwrap().0.parse::<usize>().unwrap() - 1]
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
                    Color([0.5; 3]),
                )
            })
        })
        .collect()
}
