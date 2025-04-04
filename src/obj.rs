use std::{collections::HashMap, fs};

use crate::{
    Color,
    config::push_material,
    material::{Material, MaterialKind},
    shapes::Triangle,
    vec3::Vec3,
};

pub fn parse(path: &str, materials: &mut Vec<Material>) -> Vec<Triangle> {
    let string = fs::read_to_string(path).unwrap();
    let lines = string.lines();

    let material_file = lines
        .clone()
        .find(|line| line.starts_with("mtllib"))
        .map(|line| fs::read_to_string(format!("obj/{}", &line[7..])).unwrap());

    let name_index = parse_materials(materials, material_file.as_deref());

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

    let mut triangles = Vec::new();

    // skip(1): first object is after the first o
    for object in string.split("\no ").skip(1) {
        let mut lines = object.lines();

        // either the material specified with usemtl or the default one
        let index = lines.find(|line| line.starts_with("usemtl")).map_or_else(
            // default material
            || {
                push_material(
                    Material::new(MaterialKind::Lambertian, Color([0.5; 3])),
                    materials,
                )
            },
            // usemtl
            |line| *name_index.get(&line[7..]).expect("Undefined material"),
        );

        lines
            .filter(|line| line.starts_with('f')) // get faces
            .flat_map(|line| {
                // get vertices and normals
                let mut iter = line[1..].split_whitespace().map(|part| {
                    let mut parts = part.splitn(3, '/');

                    let vertex_index: usize = parts.next().unwrap().parse().unwrap();
                    parts.next(); // skip texture
                    let normal_index: usize = parts.next().unwrap().parse().unwrap();

                    (vertices[vertex_index - 1], normals[normal_index - 1])
                });

                let first = iter.next().unwrap();

                // Fan triangulation
                // TODO: maybe use a better approach
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
            .collect_into(&mut triangles);
    }

    triangles
}

/// Returns a `HashMap` of (material name -> material index)
// TODO: parse some more properties
fn parse_materials<'a>(
    materials: &mut Vec<Material>,
    material_file: Option<&'a str>,
) -> HashMap<&'a str, u16> {
    let mut name_index = HashMap::new();
    if let Some(material_file) = material_file {
        // skip(1): skip header etc
        for material_section in material_file.split("newmtl ").skip(1) {
            let mut lines = material_section.lines();

            let name = lines.next().unwrap();

            let diffuse_color = lines
                .find(|line| line.starts_with("Kd"))
                .map(|line| Color::from(&line[3..]))
                .unwrap();

            let material = Material::new(MaterialKind::Lambertian, diffuse_color);

            let index = push_material(material, materials);

            name_index.insert(name, index);
        }
    }

    name_index
}
