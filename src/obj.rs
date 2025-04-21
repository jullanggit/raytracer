use std::{collections::HashMap, fs};

use crate::{
    Color,
    config::push_material,
    material::{ColorKind, Material, MaterialKind},
    shapes::{NormalsTextureCoordinates, Triangle},
    vec3::{NormalizedVec3, Vec3},
};

#[expect(clippy::type_complexity)]
#[inline(always)]
pub fn parse(
    path: &str,
    materials: &mut Vec<Material>,
    texture_coordinates_out: &mut Vec<[[f32; 2]; 3]>,
    normals_out: &mut Vec<[NormalizedVec3; 3]>,
    barycentric_precomputed: &mut Vec<[f32; 4]>,
) -> Vec<Triangle> {
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

    let texture_coordinates: Vec<[f32; 2]> = lines
        .clone()
        .filter(|line| line.starts_with("vt"))
        .map(|line| {
            let mut iter = line[3..].trim().split(' ').map(|str| str.parse().unwrap());
            let texture_coordinates = [iter.next().unwrap(), iter.next().unwrap()];

            if iter.next().is_some_and(|value| value != 0.) {
                eprintln!("Warning: only 2d texture coordinates are currently supported")
            }
            texture_coordinates
        })
        .collect();

    let normals: Vec<Vec3> = lines
        .clone()
        .filter(|line| line.starts_with("vn"))
        .map(|line| line[3..].trim().into())
        .collect();

    let mut triangles = Vec::new();

    // skip(1): first object is after the first o
    for object in string.split("\no ").skip(1) {
        let lines = object.lines();

        // either the material specified with usemtl or the default one
        let material_index = lines
            .clone()
            .find(|line| line.starts_with("usemtl"))
            .map_or_else(
                // default material
                || {
                    push_material(
                        Material::new(MaterialKind::Lambertian, ColorKind::Solid(Color([0.5; 3]))),
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
                    // (vertex, texture, normal)
                    let mut indices = part
                        .split('/')
                        .zip([vertices.len(), texture_coordinates.len(), normals.len()])
                        .map(|(str_index, len)| {
                            let index: isize = str_index.parse().unwrap();

                            #[expect(clippy::cast_sign_loss)] // we check for negative index
                            if index < 0 {
                                len - index.unsigned_abs()
                            } else {
                                index as usize - 1
                            }
                        });
                    (
                        vertices[indices.next().unwrap()],
                        indices.next().map(|index| texture_coordinates[index]),
                        indices.next().map(|index| normals[index]),
                    )
                });

                let (vertex1, tc1, normal1) = iter.next().unwrap();

                // Fan triangulation
                // TODO: maybe use a better approach
                iter.map_windows(
                    |&[(vertex2, tc2, normal2), (vertex3, tc3, normal3)]: &[_; 2]| {
                        // has texture coordinates
                        let texture_coordinates_index = if let Some(tc1) = tc1
                            && let Some(tc2) = tc2
                            && let Some(tc3) = tc3
                        {
                            let index = texture_coordinates.len();
                            texture_coordinates_out.push([tc1, tc2, tc3]);
                            Some(index as u32)
                        } else {
                            None
                        };
                        // has vertex normals
                        let normals_index = if let Some(normal1) = normal1
                            && let Some(normal2) = normal2
                            && let Some(normal3) = normal3
                        {
                            let normal_index = normals_out.len();

                            normals_out.push([
                                normal1.normalize(),
                                normal2.normalize(),
                                normal3.normalize(),
                            ]);

                            #[expect(clippy::cast_possible_truncation)]
                            Some(normal_index as u32)
                        } else {
                            None
                        };
                        let mut barycentric_precomputed_index = || {
                            let e1 = vertex2 - vertex1;
                            let e2 = vertex3 - vertex1;

                            let (d00, d01, d11) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));

                            let index = barycentric_precomputed.len();

                            barycentric_precomputed.push([d00, d01, d11, d00 * d11 - d01.powi(2)]);

                            index as u32
                        };
                        let normals_texture_coordinates =
                            match (texture_coordinates_index, normals_index) {
                                (Some(texture_coordinates_index), Some(normals_index)) => {
                                    NormalsTextureCoordinates::Both {
                                        normals_index,
                                        texture_coordinates_index,
                                        barycentric_precomputed_index:
                                            barycentric_precomputed_index(),
                                    }
                                }
                                (Some(texture_coordinates_index), None) => {
                                    NormalsTextureCoordinates::TextureCoordinates {
                                        texture_coordinates_index,
                                        barycentric_precomputed_index:
                                            barycentric_precomputed_index(),
                                    }
                                }
                                (None, Some(normals_index)) => NormalsTextureCoordinates::Normals {
                                    normals_index,
                                    barycentric_precomputed_index: barycentric_precomputed_index(),
                                },
                                (None, None) => NormalsTextureCoordinates::None,
                            };

                        Triangle::new(
                            vertex1,
                            vertex2,
                            vertex3,
                            normals_texture_coordinates,
                            material_index,
                        )
                    },
                )
                .collect_into(&mut triangles);
            });
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
                .map(|line| Color::from(&line[3..]));

            let diffuse_texture = lines.find(|line| line.starts_with("map_Kd")).map(|line| {
                let file_name = &line[6..];
                ColorKind::texture_from_ppm_p6(file_name)
            });

            let material = Material::new(
                MaterialKind::Lambertian,
                match (diffuse_texture, diffuse_color) {
                    (Some(diffuse_texture), _) => diffuse_texture,
                    (None, Some(diffuse_color)) => ColorKind::Solid(diffuse_color),
                    (None, None) => ColorKind::Solid(Color([0.5; 3])),
                },
            );

            let index = push_material(material, materials);

            name_index.insert(name, index);
        }
    }

    name_index
}
