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
    #[inline(always)]
    pub fn combine<F, O>(self, other: &Self, f: F) -> Vector<DIMENSIONS, O>
    where
        F: Fn(T, T) -> O,
        O: Copy,
    {
        Vector(array::from_fn(|index| f(self.0[index], other.0[index])))
    }
    #[inline(always)]
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
    #[inline(always)]
    pub fn length_squared(&self) -> T
    where
        T: Add<Output = T> + Clone + Mul<Output = T>,
    {
        self.dot(*self)
    }
}
impl<T: Copy> Vector<3, T> {
    #[inline(always)]
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
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Vector(self.0.map(|e| -e))
    }
}
impl<const DIMENSIONS: usize, T> Default for Vector<DIMENSIONS, T>
where
    T: Copy + Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self([Default::default(); DIMENSIONS])
    }
}
macro_rules! impl_vec_float {
    ($($Type:ident),*) => {
        $(
            impl<const DIMENSIONS: usize> Vector<DIMENSIONS, $Type> {
                #[inline(always)]
                pub fn length(&self) -> $Type
                {
                    self.length_squared().sqrt()
                }
                #[inline(always)]
                pub fn normalize(self) -> NormalizedVector<DIMENSIONS, $Type>
                {
                    NormalizedVector(self.clone() / self.length())
                }
                #[inline(always)]
                pub fn is_normalized(&self) -> bool {
                    const TOLERANCE: $Type = 1e-5;
                    self.length() <= 1. + TOLERANCE && self.length() >= 1. - TOLERANCE
                }
                #[inline(always)]
                pub fn near_zero(&self) -> bool {
                    self.0.map(|e| e.abs() < $Type::EPSILON) == [true; _]
                }
                /// Element-wise min
                #[inline(always)]
                pub fn min(self, other: Self) -> Self {
                    self.combine(&other, $Type::min)
                }
                /// Element-wise max
                #[inline(always)]
                pub fn max(self, other: Self) -> Self {
                    self.combine(&other, $Type::max)
                }
                #[inline(always)]
                pub fn lerp(self, other: Self, t: $Type) -> Self {
                    self.combine(&other, |e1, e2| e1 * (1. - t) + e2 * t)
                }
                /// gamma 2 correction
                #[inline(always)]
                pub fn color_correct(self) -> Self {
                    Self(self.0.map($Type::sqrt))
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
                #[inline(always)]
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
                #[inline(always)]
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
                #[inline(always)]
                pub const fn $name(&self) -> T {
                    self.0[$index]
                }
            }
        )*
    };
}
access_vec!(x => 0, y => 1, z => 2, w => 3);
/// Converts natural -> float Colors (0..MAX -> 0.0..1.0).
pub trait ToFloatColor<F> {
    fn to_float_color(self) -> F;
}
/// Converts float -> natural Colors (0.0..1.0 -> 0..MAX).
pub trait ToNaturalColor<N> {
    fn to_natural_color(self) -> N;
}
macro_rules! float_natural_conversion {
    // base case
    ( -> $($natural:ident),*) => {};

    // recurse case
    ($float:ident $(, $float_tail:ident)* -> $($natural:ident),*) => {
        $(
            #[expect(clippy::allow_attributes)]
            #[allow(clippy::cast_possible_truncation)] // We check in debug mode
            #[allow(clippy::cast_sign_loss)]
            #[allow(clippy::cast_precision_loss)]
            #[allow(clippy::cast_lossless)]
            impl<const DIMENSIONS: usize> ToFloatColor<Vector<DIMENSIONS, $float>> for  Vector<DIMENSIONS, $natural> {
                #[inline(always)]
                fn to_float_color(self) -> Vector<DIMENSIONS, $float> {
                    Vector(self.0.map(|natural| natural as $float / $natural::MAX as $float))
                }
            }
            #[expect(clippy::cast_possible_truncation)] // We check in debug mode
            #[expect(clippy::cast_sign_loss)]
            #[expect(clippy::allow_attributes)]
            #[allow(clippy::cast_precision_loss)]
            #[allow(clippy::cast_lossless)]
            impl<const DIMENSIONS: usize> ToNaturalColor<Vector<DIMENSIONS, $natural>> for  Vector<DIMENSIONS, $float> {
                #[inline(always)]
                fn to_natural_color(self) -> Vector<DIMENSIONS, $natural> {
                    Vector(
                        self.0.map(|float| {
                            debug_assert!((0.0..=1.).contains(&float));

                            (float * $natural::MAX as $float) as $natural
                        })
                    )
                }
            }
        )*
        float_natural_conversion!($($float_tail),* -> $($natural),*);
    };
}
float_natural_conversion!(f16, f32, f64, f128 -> u8, u16, u32, u64, u128);

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
    #[inline(always)]
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
    #[inline(always)]
    fn neg(self) -> Self::Output {
        NormalizedVector(self.0.neg())
    }
}
impl<const DIMENSIONS: usize> Random for NormalizedVector<DIMENSIONS, f32> {
    #[inline(always)]
    fn random() -> Self {
        Vector(array::from_fn(|_| f32::random() - 0.5)).normalize()
    }
}
macro_rules! impl_normalized_vec_float {
    ($($Type:ident),*) => {
        $(
            impl<const DIMENSIONS: usize> NormalizedVector<DIMENSIONS, $Type> {
                #[inline(always)]
                pub fn new(vector: Vector<DIMENSIONS, $Type>) -> Self {
                    debug_assert!(vector.is_normalized(), "vector: {vector:?}, len: {:?}", vector.length());

                    Self(vector)
                }
                #[inline(always)]
                pub fn reflect(&self, normal: Self) -> Self {
                    Self(**self - *normal * 2. * self.dot(*normal))
                }
            }
        )*
    };
}
impl_normalized_vec_float!(f16, f32, f64, f128);

pub type NormalizedVec3 = NormalizedVector<3, f32>;
