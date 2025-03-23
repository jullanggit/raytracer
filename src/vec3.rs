use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::rng;

/// A right-handed cartesian coordinate
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - rhs.y * self.z,
            y: self.z * rhs.x - rhs.z * self.x,
            z: self.x * rhs.y - rhs.x * self.y,
        }
    }
    pub fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }
    pub fn normalize(self) -> NormalizedVec3 {
        NormalizedVec3(self / self.length())
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[expect(clippy::fallible_impl_from)] // TODO: Remove once we care about crashes
impl From<&str> for Vec3 {
    fn from(value: &str) -> Self {
        let mut values = value.split(' ').map(|value| value.parse().unwrap());

        Self {
            x: values.next().unwrap(),
            y: values.next().unwrap(),
            z: values.next().unwrap(),
        }
    }
}

// TODO: add some more methods, so that you don't have to do .inner() all the time
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct NormalizedVec3(Vec3);
impl NormalizedVec3 {
    pub const fn inner(&self) -> &Vec3 {
        &self.0
    }
    pub fn random() -> Self {
        Vec3::new(rng::f32() - 0.5, rng::f32() - 0.5, rng::f32() - 0.5).normalize() // -0.5..0.5
    }
}

impl Neg for NormalizedVec3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-*self.inner())
    }
}

impl Add for NormalizedVec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self::Output {
        self.0 + rhs.0
    }
}
