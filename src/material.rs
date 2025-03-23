use crate::Color;

#[derive(Debug, PartialEq)]
pub struct Material {
    pub kind: MaterialKind,
    pub color: Color<f32>,
}

impl Material {
    pub const fn new(color: Color<f32>) -> Self {
        Self {
            kind: MaterialKind::Lambertian,
            color,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MaterialKind {
    Lambertian,
}
