use std::{fs, str::Split};

use crate::{
    Camera, Light, Plane, Scene, Screen, Sphere, material::Material, obj, shapes::Triangle,
    vec3::Vec3,
};

pub fn parse() -> Scene {
    let string = fs::read_to_string("scene").unwrap();

    let mut iter = string.lines();

    let mut screen = None;
    let mut camera = None;
    let mut spheres = None;
    let mut planes = None;
    let mut triangles = None;
    let mut light = None;

    let mut materials = Vec::new();

    while screen.is_none() | camera.is_none() | spheres.is_none() | light.is_none() {
        match iter.next().unwrap().split_once('(').unwrap() {
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
            ("camera", value) => camera = Some(Camera::new(value[..value.len() - 1].into())),
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
                    Triangle::default_normal(
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        values.next().unwrap().into(),
                        push_material_with_values(values, &mut materials),
                    )
                }));
            }
            ("light", value) => {
                light = Some(single_item_parse(value, |values| {
                    Light::new(values.next().unwrap().into(), values.next().unwrap().into())
                }));
            }
            ("obj", value) => {
                let triangles = triangles.get_or_insert_with(Vec::new);

                multi_item_parse(value, |value| {
                    obj::parse(
                        &format!("obj/{}.obj", value.next().unwrap()),
                        &mut materials,
                    )
                })
                .into_iter()
                .for_each(|mut vec| triangles.append(&mut vec));
            }
            (other, value) => panic!("Unknown entry {other} with value {value}"),
        }
    }

    Scene {
        screen: screen.unwrap(),
        camera: camera.unwrap(),
        spheres: spheres.unwrap(),
        planes: planes.unwrap(),
        triangles: triangles.unwrap(),
        materials,
        light: light.unwrap(),
    }
}

fn push_material_with_values(values: &mut Split<&str>, materials: &mut Vec<Material>) -> u16 {
    push_material(Material::new(values.next().unwrap().into()), materials)
}
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
    let mut values = value[..value.len() - 1].split(", "); // Skip closing parenthesis with len - 1

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
