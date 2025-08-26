use std::{
    array,
    fmt::Debug,
    num::FpCategory,
    ops::{Add, Deref, Div, Mul, Neg, Sub},
};

use crate::rng::Random;

// I know this is all way to generic, but its fun :D

macro_rules! ImplFloat {
    ({$float:ident} $({$tail:ident})* [$(const $const:ident;)+ $(std const $std_const:ident;)+ $(fn $fn:ident(self $(, $arg:ident: $type:ty )*) -> $return:ty);+ $(;)?]) => {
        impl Float for $float {
            $(const $const: $float = $float::$const;)+
            $(const $std_const: Self = std::$float::consts::$std_const;)+
            $(
                fn $fn(self $(, $arg: $type)*) -> $return {
                    self.$fn($($arg),*)
                }
            )+
        }

        ImplFloat!($({$tail})* [$(const $const;)+ $(std const $std_const;)+ $(fn $fn(self $(, $arg: $type )*) -> $return);+]);
    };
    ([$(const $const:ident;)+ $(std const $std_const:ident;)+ $(fn $fn:ident(self $(, $arg:ident: $type:ty )*) -> $return:ty);+ $(;)?]) => {
        pub trait Float: Copy + Add<Output = Self> + Mul<Output = Self>
        + Div<Output = Self> + PartialOrd + Sub<Output = Self> + From<bool>
        + Debug + Neg<Output = Self> {
            $(const $const: Self;)+
            $(const $std_const: Self;)+
            $(fn $fn(self $(, $arg: $type)*) -> $return;)+
        }
    }
}
ImplFloat!({f16} {f32} {f64} {f128} [
    const EPSILON;
    std const PI;
    fn abs(self) -> Self;
    fn acos(self) -> Self;
    fn acosh(self) -> Self;
    fn asin(self) -> Self;
    fn asinh(self) -> Self;
    fn atan(self) -> Self;
    fn atan2(self, other: Self) -> Self;
    fn atanh(self) -> Self;
    fn cbrt(self) -> Self;
    fn ceil(self) -> Self;
    fn classify(self) -> FpCategory;
    fn is_sign_positive(self) -> bool;
    fn is_sign_negative(self) -> bool;
    fn next_up(self) -> Self;
    fn next_down(self) -> Self;
    fn recip(self) -> Self;
    fn to_degrees(self) -> Self;
    fn to_radians(self) -> Self;
    fn max(self, other: Self) -> Self;
    fn min(self, other: Self) -> Self;
    fn midpoint(self, other: Self) -> Self;
    fn clamp(self, min: Self, max: Self) -> Self;
    fn copysign(self, sign: Self) -> Self;
    fn sqrt(self) -> Self;
    fn mul_add(self, a: Self, b: Self) -> Self;
    fn powf(self, n: Self) -> Self;
    fn exp(self) -> Self;
    fn exp2(self) -> Self;
    fn ln(self) -> Self;
    fn log(self, base: Self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
]);
pub fn literal_to_float<T>(literal: f128) -> T
where
    f128: AsConvert<T>,
{
    literal.as_convert()
}

impl<T: Float> Sqrt for T {
    fn sqrt(self) -> Self {
        self.sqrt()
    }
}

pub trait Sqrt<Output = Self> {
    fn sqrt(self) -> Output;
}
// implement sqrt for primitive number types, naturals get converted to floats
macro_rules! impl_sqrt {
    // base case
    ((), $($Type:ident),*) => {};
    // recursive case
    (($float:ident $(, $float_tail:ident)*), $($Type:ident),*) => {
        $(
            impl Sqrt<$float> for $Type {
                #[expect(clippy::allow_attributes)]
                #[allow(clippy::cast_lossless)]
                #[allow(clippy::cast_precision_loss)]
                #[allow(clippy::cast_possible_truncation)]
                #[inline(always)]
                fn sqrt(self) -> $float {
                    (self as $float).sqrt()
                }
            }
        )*
        impl_sqrt!(($($float_tail),*), $($Type),*);
    };
}
impl_sqrt!(
    (f16, f32, f64, f128),
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize
);

pub trait MinMax<Other = Self> {
    fn min(self, other: Other) -> Self;
    fn max(self, other: Other) -> Self;
}
macro_rules! impl_min_max {
    ($Trait:ident -> $($Type:ident),*) => {
        $(
            impl MinMax for $Type {
                fn min(self, other: Self) -> Self {
                    $Trait::min(self, other)
                }
                fn max(self, other: Self) -> Self {
                    $Trait::max(self, other)
                }
            }
        )*
    };
}
impl_min_max!(Self -> f16, f32, f64, f128);
impl_min_max!(Ord -> u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

pub trait Lerp<X> {
    type Output;
    fn lerp(&self, other: Self, x: X) -> Self::Output;
}
impl<T, X> Lerp<X> for T
where
    T: Clone + Mul<X> + Mul<<X as Sub>::Output, Output: Add<<T as Mul<X>>::Output>>,
    X: From<u8> + Sub + Clone,
{
    type Output = <<T as Mul<<X as Sub>::Output>>::Output as Add<<T as Mul<X>>::Output>>::Output;
    fn lerp(&self, other: Self, x: X) -> Self::Output {
        self.clone() * (X::from(1) - x.clone()) + other * x
    }
}

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
        T: Add<Output = T> + Mul<Output = T>,
    {
        self.dot(*self)
    }
    #[inline(always)]
    pub fn length<O>(&self) -> O
    where
        T: Add<Output = T> + Mul<Output = T> + Sqrt<O>,
    {
        self.length_squared().sqrt()
    }
    #[inline(always)]
    pub fn normalize<O>(self) -> NormalizedVector<DIMENSIONS, O>
    where
        T: Add<Output = T> + Mul<Output = T> + Sqrt<O>,
        O: Copy + Div<Output = O>,
        Vector<DIMENSIONS, O>: From<Self>,
    {
        NormalizedVector(Vector::from(self) / self.length())
    }
    /// gamma 2 correction
    #[inline(always)]
    pub fn color_correct(self) -> Self
    where
        T: Sqrt,
    {
        Self(self.0.map(Sqrt::sqrt))
    }
    pub fn gram_schmidt(self, w: NormalizedVector<DIMENSIONS, T>) -> Self
    where
        T: Add<Output = T> + Mul<Output = T> + Sub<Output = T>,
    {
        self - *w * self.dot(*w)
    }
    /// Element-wise min
    #[inline(always)]
    pub fn min(self, other: Self) -> Self
    where
        T: MinMax,
    {
        self.combine(&other, MinMax::min)
    }
    /// Element-wise max
    #[inline(always)]
    pub fn max(self, other: Self) -> Self
    where
        T: MinMax,
    {
        self.combine(&other, MinMax::max)
    }
}
impl<T: Copy> Vector<3, T> {
    // TODO: maybe use difference_of_products (not yet implemented) to raise precision
    #[inline(always)]
    pub fn cross(self, other: Self) -> Self
    where
        T: Mul<Output: Sub<Output = T> + Copy>,
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
        Vector(self.0.map(Neg::neg))
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
/// Convert between types using `as`
pub trait AsConvert<T> {
    fn as_convert(&self) -> T;
}
macro_rules! impl_as_convert {
    // base case
    () => {};
    ($From:ident $(, $Into:ident)*) => {
        impl AsConvert<$From> for $From {
            fn as_convert(&self) -> $From {
                *self
            }
        }

        $(
            impl AsConvert<$Into> for $From {
                #[expect(clippy::allow_attributes)]
                #[allow(clippy::cast_lossless)]
                #[allow(clippy::cast_precision_loss)]
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_sign_loss)]
                #[allow(clippy::cast_possible_wrap)]
                #[inline(always)]
                fn as_convert(&self) -> $Into {
                    *self as $Into
                }
            }
            impl AsConvert<$From> for $Into {
                #[expect(clippy::allow_attributes)]
                #[allow(clippy::cast_lossless)]
                #[allow(clippy::cast_precision_loss)]
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_sign_loss)]
                #[allow(clippy::cast_possible_wrap)]
                #[inline(always)]
                fn as_convert(&self) -> $From {
                    *self as $From
                }
            }
        )*
        impl_as_convert!($($Into),*);
    }
}
impl_as_convert!(
    f16, f32, f64, f128, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
impl<const DIMENSIONS: usize, Source, Target> AsConvert<Vector<DIMENSIONS, Target>>
    for Vector<DIMENSIONS, Source>
where
    Source: AsConvert<Target> + Copy,
    Target: Copy,
{
    #[inline(always)]
    fn as_convert(&self) -> Vector<DIMENSIONS, Target> {
        Vector(self.0.map(|e| e.as_convert()))
    }
}
impl<const DIMENSIONS: usize, T: Float> Vector<DIMENSIONS, T>
where
    f128: AsConvert<T>,
{
    #[inline(always)]
    pub fn is_normalized(&self) -> bool {
        const TOLERANCE: f128 = 1e-5;
        self.length::<T>() <= (1. + TOLERANCE).as_convert()
            && self.length::<T>() >= (1. - TOLERANCE).as_convert()
    }
    #[inline(always)]
    pub fn near_zero(&self) -> bool {
        self.0.map(|e| e.abs() < T::EPSILON) == [true; _]
    }
    #[inline(always)]
    pub fn angle_between<O>(self, other: Self) -> T {
        if self.dot(other) < (0.).as_convert() {
            T::PI
                - literal_to_float(2.)
                    * ((self + other).length::<T>() / literal_to_float(2.)).asin()
        } else {
            literal_to_float(2.) * ((other - self).length::<T>() / literal_to_float(2.))
        }
    }
}
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
                    Vector(self.0.map(|e| e.$method(rhs)))
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
float_natural_conversion!(f16, f32, f64, f128 -> u8, u16, u32, u64, u128, usize);

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
// not generic because of the -0.5
impl<const DIMENSIONS: usize> Random for NormalizedVector<DIMENSIONS, f32> {
    #[inline(always)]
    fn random() -> Self {
        Vector(array::from_fn(|_| f32::random() - 0.5)).normalize()
    }
}
impl<const DIMENSIONS: usize, T: Float> NormalizedVector<DIMENSIONS, T>
where
    f128: AsConvert<T>,
{
    #[inline(always)]
    pub fn new(vector: Vector<DIMENSIONS, T>) -> Self {
        debug_assert!(
            vector.is_normalized(),
            "vector: {vector:?}, len: {:?}",
            vector.length::<T>()
        );

        Self(vector)
    }
    #[inline(always)]
    pub fn reflect(&self, normal: Self) -> Self {
        Self(**self - *normal * literal_to_float(2.) * self.dot(*normal))
    }
}
impl<T: Float> NormalizedVector<3, T>
where
    f128: AsConvert<T>,
{
    #[inline(always)]
    pub fn coordinate_system(self) -> [Self; 2] {
        let sign = T::copysign(literal_to_float(1.), self.z());
        let a = literal_to_float(-1.) / (sign + self.z());
        let b = self.x() * self.y() * a;

        [
            Self(Vector([
                literal_to_float(1.) + sign * self.x().sqrt() * a,
                sign * b,
                -sign * self.x(),
            ])),
            Self(Vector([b, sign + self.y().sqrt() * a, -self.y()])),
        ]
    }
    #[inline(always)]
    // A unit vector pointing in the given spherical direction
    pub fn spherical_direction(sin_theta: T, cos_theta: T, phi: T) -> Self {
        {
            let lower = literal_to_float(-1.);
            let upper = literal_to_float(1.);
            debug_assert!(
                lower < sin_theta && sin_theta <= upper && lower < cos_theta && cos_theta <= upper,
                "sin_theta: {sin_theta:?}, cos_theta: {cos_theta:?}"
            );
        }
        Self(Vector([
            sin_theta * phi.cos(),
            sin_theta * phi.sin(),
            cos_theta,
        ]))
    }
}

pub type NormalizedVec3 = NormalizedVector<3, f32>;
