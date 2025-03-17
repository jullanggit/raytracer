use std::fs;

use crate::{Color, shapes::Triangle, vec3::Vec3};

pub fn parse(path: &str) -> Vec<Triangle> {
    let string = fs::read_to_string(path).unwrap();
    let lines = string.lines();

    let vertices: Vec<Vec3> = lines
        .clone()
        .filter(|line| line.starts_with("v "))
        .map(|line| line[1..].trim().into())
        .collect();

    lines
        .filter(|line| line.starts_with('f'))
        .flat_map(|line| {
            let mut iter = line[1..].split_whitespace().map(|vertex| {
                vertices[vertex.split_once('/').unwrap().0.parse::<usize>().unwrap() - 1]
            });

            let first = iter.next().unwrap();

            iter.map_windows(move |vertices: &[Vec3; 2]| {
                Triangle::new(first, vertices[0], vertices[1], Color([0.5; 3]))
            })
        })
        .collect()
}
