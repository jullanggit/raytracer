use std::{collections::HashMap, fs};

use crate::{
    Color,
    config::push_material,
    material::{Material, MaterialKind},
    shapes::Triangle,
    vec3::{NormalizedVec3, Vec3},
};

#[expect(clippy::type_complexity)]
#[inline(always)]
pub fn parse(
    path: &str,
    materials: &mut Vec<Material>,
) -> (Vec<Triangle>, Vec<([NormalizedVec3; 3], [f32; 4])>) {
    let string = fs::read_to_string(path).expect("Failed to read obj file");
    let lines = string.lines();

    let material_file = lines
        .clone()
        .find(|line| line.starts_with("mtllib"))
        .map(|line| {
            fs::read_to_string(format!("obj/{}", &line[7..])).expect("Failed to read mtl file")
        });

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

    let mut normals_out = Vec::new();

    let mut triangles = Vec::new();

    // skip(1): first object is after the first o
    for object in string.split("\no ").skip(1) {
        let lines = object.lines();

        // either the material specified with usemtl or the default one
        let index = lines
            .clone()
            .find(|line| line.starts_with("usemtl"))
            .map_or_else(
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
            .for_each(|line| {
                // get vertices and normals
                let mut iter = line[1..].split_whitespace().map(|part| {
                    let mut parts = part.splitn(3, '/');

                    let vertex_index: usize = {
                        let index: isize = parts.next().unwrap().parse().unwrap();

                        #[expect(clippy::cast_sign_loss)] // we check for negative index
                        if index < 0 {
                            vertices.len() - index.unsigned_abs()
                        } else {
                            index as usize - 1
                        }
                    };
                    parts.next(); // skip texture
                    let mut normal_index = || {
                        let index: isize = parts.next()?.parse().ok()?;

                        #[expect(clippy::cast_sign_loss)] // we check for negative index
                        Some(if index < 0 {
                            normals.len() - index.unsigned_abs()
                        } else {
                            index as usize - 1
                        })
                    };

                    (
                        vertices[vertex_index],
                        normal_index().and_then(|index| normals.get(index)),
                    )
                });

                let (vertex1, normal1) = iter.next().unwrap();

                // Fan triangulation
                // TODO: maybe use a better approach
                iter.map_windows(|&[(vertex2, normal2), (vertex3, normal3)]: &[_; 2]| {
                    if let Some(normal1) = normal1
                        && let Some(normal2) = normal2
                        && let Some(normal3) = normal3
                    {
                        let e1 = vertex2 - vertex1;
                        let e2 = vertex3 - vertex1;

                        let (d00, d01, d11) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));

                        let normal_index = normals_out.len();

                        normals_out.push((
                            [
                                normal1.normalize(),
                                normal2.normalize(),
                                normal3.normalize(),
                            ],
                            [d00, d01, d11, d00 * d11 - d01.powi(2)],
                        ));

                        #[expect(clippy::cast_possible_truncation)]
                        Triangle::new(vertex1, vertex2, vertex3, Some(normal_index as u32), index)
                    } else {
                        Triangle::new(vertex1, vertex2, vertex3, None, index)
                    }
                })
                .collect_into(&mut triangles);
            });
    }

    (triangles, normals_out)
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
                .map_or(Color([0.5; 3]), |line| Color::from(&line[3..]));

            let material = Material::new(MaterialKind::Lambertian, diffuse_color);

            let index = push_material(material, materials);

            name_index.insert(name, index);
        }
    }

    name_index
}
