use std::fs;

use crate::{Camera, Light, Scene, Screen, Sphere};

pub fn parse() -> Scene {
    let string = fs::read_to_string("scene").unwrap();

    let mut iter = string.lines();

    let mut screen = None;
    let mut camera = None;
    let mut spheres = None;
    let mut light = None;

    while screen.is_none() | camera.is_none() | spheres.is_none() | light.is_none() {
        match iter.next().unwrap().split_once('(').unwrap() {
            ("screen", value) => {
                let mut values = value[..value.len() - 1].split(", "); // Skip closing parenthesis

                screen = Some(Screen::new(
                    values.next().unwrap().into(),
                    values.next().unwrap().into(),
                    values.next().unwrap().into(),
                    values.next().unwrap().parse().unwrap(),
                    values.next().unwrap().parse().unwrap(),
                ));

                assert!(values.next().is_none());
            }
            ("camera", value) => camera = Some(Camera::new(value[..value.len() - 1].into())),
            ("spheres", value) => {
                let spheres_string = value[1..value.len() - 2].split("), ("); // Skip closing parenthesis

                let mut inner_spheres = Vec::new();

                for sphere_string in spheres_string {
                    let mut parts = sphere_string.split(", ");

                    inner_spheres.push(Sphere::new(
                        parts.next().unwrap().into(),
                        parts.next().unwrap().parse().unwrap(),
                        parts.next().unwrap().into(),
                    ));

                    assert!(parts.next().is_none());
                }

                spheres = Some(inner_spheres);
            }
            ("light", value) => {
                let mut values = value[..value.len() - 1].split(", "); // Skip closing parenthesis

                light = Some(Light::new(
                    values.next().unwrap().into(),
                    values.next().unwrap().into(),
                ));

                assert!(values.next().is_none());
            }
            (other, value) => panic!("Unknown entry {other} with value {value}"),
        }
    }

    Scene {
        screen: screen.unwrap(),
        camera: camera.unwrap(),
        spheres: spheres.unwrap(),
        light: light.unwrap(),
    }
}
