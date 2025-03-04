use std::ops::{Add, Div, Mul, Sub};

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
            x: self.y * rhs.z - rhs.y - self.z,
            y: self.z * rhs.x - rhs.z - self.x,
            z: self.x * rhs.y - rhs.x - self.y,
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

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct NormalizedVec3(Vec3);
impl NormalizedVec3 {
    pub const fn inner(&self) -> &Vec3 {
        &self.0
    }
}
