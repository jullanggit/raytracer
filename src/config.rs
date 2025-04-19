use std::str::Split;

use crate::{
    Bvhs, Camera, Plane, Scene, Screen, Shapes, Sphere, bvh::BvhNode, material::Material, obj,
    shapes::Triangle, vec3::Vec3,
};

#[expect(clippy::too_many_lines)]
pub fn parse(string: &str) -> Scene {
    let mut iter = string.lines();

    // init values
    let mut incremental = None;
    let mut screen = None;
    let mut camera = None;
    let mut spheres = None;
    let mut planes = None;
    let mut triangles = None;
    let mut normals = None;
    let mut materials = Vec::new();

    // parse
    while screen.is_none()
        | camera.is_none()
        | spheres.is_none()
        | planes.is_none()
        | triangles.is_none()
    {
        let next = iter.next().unwrap();
        // split into field and value
        match next[..next.len() - 1].split_once('(').unwrap() {
            ("incremental", value) => {
                incremental = Some(value.parse().unwrap());
            }
            ("screen", value) => {
                screen = Some(single_item_parse(value, |values| {
                    Screen::new(
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        values.next().unwrap().parse().unwrap(),
                        values.next().unwrap().parse().unwrap(),
                        values.next().unwrap().parse().unwrap(),
                        values.next().unwrap().parse().unwrap(),
                    )
                }));
            }
            ("camera", value) => camera = Some(Camera::new(value[..value.len()].into())),
            ("spheres", value) => {
                spheres = Some(multi_item_parse(value, |values| {
                    Sphere::new(
                        values.next().unwrap().into(),
                        values.next().unwrap().parse().unwrap(),
                        push_material_with_values(values, &mut materials),
                    )
                }));
            }
            ("planes", value) => {
                planes = Some(multi_item_parse(value, |values| {
                    Plane::new(
                        values.next().unwrap().into(),
                        Vec3::normalize(values.next().unwrap().into()),
                        push_material_with_values(values, &mut materials),
                    )
                }));
            }
            ("triangles", value) => {
                let triangles = triangles.get_or_insert_with(Vec::new);

                triangles.append(&mut multi_item_parse(value, |values| {
                    Triangle::new(
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        None,
                        push_material_with_values(values, &mut materials),
                    )
                }));
            }
            ("obj", value) => {
                let triangles = triangles.get_or_insert_with(Vec::new);
                let normals = normals.get_or_insert_with(Vec::new);

                for (mut new_triangles, mut new_normals) in multi_item_parse(value, |value| {
                    obj::parse(
                        &format!("obj/{}.obj", value.next().unwrap()),
                        &mut materials,
                    )
                }) {
                    triangles.append(&mut new_triangles);
                    normals.append(&mut new_normals);
                }
            }
            (other, value) => panic!("Unknown entry {other} with value {value}"),
        }
    }

    // wrap
    let screen = screen.unwrap();
    let mut spheres = spheres.unwrap().into_boxed_slice();
    let mut planes = planes.unwrap().into_boxed_slice();
    let mut triangles = triangles.unwrap().into_boxed_slice();
    let normals = normals.map_or_else(Box::default, |normals| normals.into_boxed_slice());

    if let Some(amount) = incremental {
        assert!(amount != 0);
        assert!(screen.samples_per_pixel % amount == 0);
    }

    Scene::new(
        incremental,
        screen,
        camera.unwrap(),
        Bvhs::new(
            BvhNode::new(&mut spheres).into_boxed_slice(),
            BvhNode::new(&mut planes).into_boxed_slice(),
            BvhNode::new(&mut triangles).into_boxed_slice(),
        ),
        Shapes::new(spheres, planes, triangles, normals),
        materials,
    )
}

fn push_material_with_values(values: &mut Split<&str>, materials: &mut Vec<Material>) -> u16 {
    push_material(
        Material::new(values.next().unwrap().into(), values.next().unwrap().into()),
        materials,
    )
}

/// Push the given material to materials and returns the index
pub fn push_material(material: Material, materials: &mut Vec<Material>) -> u16 {
    materials
        .iter()
        .position(|existing_material| *existing_material == material)
        .unwrap_or_else(|| {
            let index = materials.len();
            materials.push(material);
            index
        })
        .try_into()
        .unwrap()
}

fn single_item_parse<T>(value: &str, mut f: impl FnMut(&mut Split<&str>) -> T) -> T {
    let mut values = value.split(", "); // Skip closing parenthesis with len - 1

    let parsed = f(&mut values);

    assert!(values.next().is_none());

    parsed
}

fn multi_item_parse<T>(str: &str, mut f: impl FnMut(&mut Split<&str>) -> T) -> Vec<T> {
    let mut parsed = Vec::new();

    if str.len() > 1 {
        let values = str[1..str.len() - 1].split("), ("); // Skip opening and closing parentheses with 1..len - 1

        for value in values {
            parsed.push(single_item_parse(value, &mut f));
        }
    }

    parsed
}
