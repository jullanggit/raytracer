use std::{
    array,
    ops::{Add, Deref, Div, Mul, Neg, Sub},
};

use crate::rng::Random;

// I know this is all way to generic, but its fun :D

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector<const DIMENSIONS: usize, T: Copy>(pub [T; DIMENSIONS]);
impl<const DIMENSIONS: usize, T: Copy> Vector<DIMENSIONS, T> {
    pub fn combine<F, O>(self, other: &Self, f: F) -> Vector<DIMENSIONS, O>
    where
        F: Fn(T, T) -> O,
        O: Copy,
    {
        Vector(array::from_fn(|index| f(self.0[index], other.0[index])))
    }
    pub fn dot(self, other: Self) -> T
    where
        T: Add<Output = T> + Mul<Output = T>,
    {
        (self * other)
            .0
            .into_iter()
            .reduce(|acc, e| acc + e)
            .unwrap()
    }
    pub fn length_squared(&self) -> T
    where
        T: Add<Output = T> + Clone + Mul<Output = T>,
    {
        self.dot(*self)
    }
}
impl<T: Copy> Vector<3, T> {
    pub fn cross(self, other: Self) -> Self
    where
        T: Clone + Mul,
        <T as Mul>::Output: Sub<Output = T> + Copy,
    {
        let yzx = |vector: Self| Self([vector.y(), vector.z(), vector.x()]);
        let zxy = |vector: Self| Self([vector.z(), vector.x(), vector.y()]);

        yzx(self) * zxy(other) - zxy(self) * yzx(other)
    }
}
impl<const DIMENSIONS: usize, T> Neg for Vector<DIMENSIONS, T>
where
    T: Copy + Neg,
    T::Output: Copy,
{
    type Output = Vector<DIMENSIONS, T::Output>;
    fn neg(self) -> Self::Output {
        Vector(self.0.map(|e| -e))
    }
}
macro_rules! impl_vec_float {
    ($($Type:ident),*) => {
        $(
            impl<const DIMENSIONS: usize> Vector<DIMENSIONS, $Type> {
                pub fn length(&self) -> $Type
                {
                    self.length_squared().sqrt()
                }
                pub fn normalize(self) -> NormalizedVector<DIMENSIONS, $Type>
                {
                    NormalizedVector(self.clone() / self.length())
                }
                pub fn is_normalized(&self) -> bool {
                    const TOLERANCE: $Type = 1e-5;
                    self.length() <= 1. + TOLERANCE && self.length() >= 1. - TOLERANCE
                }
                pub fn near_zero(&self) -> bool {
                    self.0.map(|e| e.abs() < $Type::EPSILON) == [true; _]
                }
                /// Element-wise min
                pub fn min(self, other: Self) -> Self {
                    self.combine(&other, $Type::min)
                }
                /// Element-wise max
                pub fn max(self, other: Self) -> Self {
                    self.combine(&other, $Type::max)
                }
            }
        )*
    };
}
impl_vec_float!(f16, f32, f64, f128);
macro_rules! impl_vec_op {
    ($(($Trait:ident, $method:ident)),*) => {
        $(
            impl<const DIMENSIONS: usize, T> $Trait for Vector<DIMENSIONS, T>
            where
                T: $Trait + Copy,
                T::Output: Copy
            {
                type Output = Vector<DIMENSIONS, T::Output>;
                fn $method(self, rhs: Self) -> Self::Output {
                    self.combine(&rhs, $Trait::$method)
                }
            }
            impl<const DIMENSIONS: usize, T> $Trait<T> for Vector<DIMENSIONS, T>
            where
                T: $Trait + Copy,
                T::Output: Copy
            {
                type Output = Vector<DIMENSIONS, T::Output>;
                fn $method(self, rhs: T) -> Self::Output {
                    Vector(self.0.map(|e| e.$method(rhs.clone())))
                }
            }
        )*
    };
}
impl_vec_op!((Add, add), (Sub, sub), (Mul, mul), (Div, div));
macro_rules! access_vec {
    ($($name:ident => $index:expr),*) => {
        $(
            impl<const DIMENSIONS: usize, T: Copy> Vector<DIMENSIONS, T>
            where
                // compile-time assertion on index
               [(); DIMENSIONS - $index -1]:
            {
                #[inline]
                pub const fn $name(&self) -> T {
                    self.0[$index]
                }
            }
        )*
    };
}
access_vec!(x => 0, y => 1, z => 2, w => 3);

pub type Vec3 = Vector<3, f32>;

impl From<&str> for Vec3 {
    fn from(value: &str) -> Self {
        let mut values = value.split(' ').map(|value| value.parse().unwrap());

        Self(array::from_fn(|_| values.next().unwrap()))
    }
}

/// A vector with length 1
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedVector<const DIMENSIONS: usize, T: Copy>(Vector<DIMENSIONS, T>);
impl<const DIMENSIONS: usize, T: Copy> Deref for NormalizedVector<DIMENSIONS, T> {
    type Target = Vector<DIMENSIONS, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<const DIMENSIONS: usize, T> Neg for NormalizedVector<DIMENSIONS, T>
where
    T: Copy + Neg,
    T::Output: Copy,
{
    type Output = NormalizedVector<DIMENSIONS, T::Output>;
    fn neg(self) -> Self::Output {
        NormalizedVector(self.0.neg())
    }
}
impl<const DIMENSIONS: usize> Random for NormalizedVector<DIMENSIONS, f32> {
    fn random() -> Self {
        Vector(array::from_fn(|_| f32::random() - 0.5)).normalize()
    }
}
macro_rules! impl_normalized_vec_float {
    ($($Type:ident),*) => {
        $(
            impl<const DIMENSIONS: usize> NormalizedVector<DIMENSIONS, $Type> {
                pub fn new(vector: Vector<DIMENSIONS, $Type>) -> Self {
                    debug_assert!(vector.is_normalized(), "vector: {vector:?}, len: {:?}", vector.length());

                    Self(vector)
                }
                pub fn reflect(&self, normal: Self) -> Self
                {
                    Self(**self - *normal * 2. * self.dot(*normal))
                }
            }
        )*
    };
}
impl_normalized_vec_float!(f16, f32, f64, f128);

pub type NormalizedVec3 = NormalizedVector<3, f32>;
