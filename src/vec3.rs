use std::{
    array,
    fmt::Debug,
    marker::PhantomData,
    num::FpCategory,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
    str::FromStr,
};

use crate::{convert::Convert, rng::Random};

// I know this is all way to generic, but its fun :D

macro_rules! ImplDelegate {
    ($trait:ident $(: $first_bound:ident $( + $bound:ident $(<$assoc:ident $(= $value:ident)?>)?)*)?, {$float:ident} $({$tail:ident})* [$(const $const:ident;)+ $(std const $std_const:ident;)* $(fn $fn:ident(self $(, $arg:ident: $type:ty )*) -> $return:ty);* $(;)?]) => {
        impl $trait for $float {
            $(const $const: $float = $float::$const;)+
            $(const $std_const: Self = std::$float::consts::$std_const;)*
            $(
                fn $fn(self $(, $arg: $type)*) -> $return {
                    self.$fn($($arg),*)
                }
            )*
        }

        ImplDelegate!($trait $(: $first_bound $(+ $bound $(<$assoc $(= $value)?>)?)*)?, $({$tail})* [$(const $const;)+ $(std const $std_const;)* $(fn $fn(self $(, $arg: $type )*) -> $return);*]);
    };
    ($trait:ident $(: $first_bound:ident $(+ $bound:ident $(<$assoc:ident $(= $value:ident)?>)?)*)?, [$(const $const:ident;)+ $(std const $std_const:ident;)* $(fn $fn:ident(self $(, $arg:ident: $type:ty )*) -> $return:ty);* $(;)?]) => {
        pub trait $trait $(: $first_bound $( + $bound $(<$assoc $(= $value)?>)?)*)? {
            $(const $const: Self;)+
            $(const $std_const: Self;)*
            $(fn $fn(self $(, $arg: $type)*) -> $return;)*
        }
    }
}
ImplDelegate!(Float: Copy + Add<Output = Self> + Mul<Output = Self>
        + Div<Output = Self> + PartialOrd + Sub<Output = Self> + From<bool>
        + Debug + Neg<Output = Self> + AddAssign, {f16} {f32} {f64} {f128} [
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

ImplDelegate!(Natural: Copy + Add<Output = Self> + Mul<Output = Self>
        + Div<Output = Self> + PartialOrd + Sub<Output = Self> + From<bool>
        + Debug + AddAssign, {u8} {u16} {u32} {u64} {u128} {usize} [
    const MAX;
]);

pub trait Sqrt<Output = Self> {
    fn sqrt(self) -> Output;
}
impl<Source: Convert<Output>, Output: Float> Sqrt<Output> for Source {
    fn sqrt(self) -> Output {
        self.convert().sqrt()
    }
}

/// Creates Trait & implementations that delegate the trait's methods to one of the two other given traits.
/// Uses something like a poor man's lattice Trait implementation, based on a #![feature(specialization)] hack. Throws a link-time error if neither of the traits are implemented.
macro_rules! DelegateTrait {
    (pub trait $trait:ident $(<$($generic:ident $(= $default:ty)?),+>)? {
        $(
            fn $fn:ident(self, $($arg:ident : $type:ty),*) $(-> $return:ty)?;
        )*
    }, $a:ident, $b:ident) => {
        // main/delegate trait
        pub trait $trait $(<$($generic $(= $default)?),+>)? {
            $(
                fn $fn(self, $($arg : $type),*) $(-> $return)?;
            )*
        }
        // fallback trait
        trait ${ concat($trait, Spec) } $(<$($generic $(= $default)?),+>)? {
            $(
                fn $fn(self, $($arg : $type),*) $(-> $return)?;
            )*
        }

        // main/delegate impl
        impl<T> $trait for T {
            $(
                default fn $fn(self, $($arg: $type),*) $(-> $return)? {
                    <Self as ${concat($trait, Spec)}>::$fn(self, $($arg),*)
                }
            )*
        }
        // implement $a on the main trait
        impl<T: $a> $trait for T {
            $(
                default fn $fn(self, $($arg: $type),*) $(-> $return)? {
                    self.$fn($($arg),*)
                }
            )*
        }
        // implement $b on the fallback trait
        impl<T: $b> ${concat($trait, Spec)} for T {
            $(
                fn $fn(self, $($arg: $type),*) $(-> $return)? {
                    self.$fn($($arg),*)
                }
            )*
        }

        // we could also add a distinct intersection impl for both $a and $b here, but I dont need it right now.

        // failing impl, if neither are implemented. Tries to link to a (hopefully) nonexistent symbol.
        impl<T> ${concat($trait, Spec)} for T {
            $(
                default fn $fn(self, $(_: $type),*) $(-> $return)? {
                    unsafe extern "C" {
                        fn ${concat(__neither, $a, Nor, $b, Implemented)}() -> !;
                    }
                    // SAFETY:
                    // yeah not really safe, fingers crossed this symbol is undefined and raises a link-time-error
                    unsafe {
                        ${concat(__neither, $a, Nor, $b, Implemented)}()
                    }
                }
            )*
        }
    };
}
DelegateTrait!(
    pub trait MinMax {
        fn min(self, other: Self) -> Self;
        fn max(self, other: Self) -> Self;
    },
    Float,
    Ord
);

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

macro_rules! VectorLabels {
    ($($label:ident),+) => {
        $(
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub struct ${ concat($label, Usage) };
            pub type $label<const DIMENSIONS: usize, T> = BaseVector<DIMENSIONS, T, ${concat($label, Usage)}>;
        )+
    };
}
VectorLabels!(Vector, Point, Normal, NormalizedVector, Color);

pub trait VectorOrColor {}
impl VectorOrColor for VectorUsage {}
impl VectorOrColor for ColorUsage {}

pub trait New<Input> {
    fn new(input: Input) -> Self;
}

#[repr(transparent)]
#[derive(PartialEq)]
pub struct BaseVector<const DIMENSIONS: usize, T, Usage>([T; DIMENSIONS], PhantomData<Usage>);
impl<const DIMENSIONS: usize, T, Usage> BaseVector<DIMENSIONS, T, Usage> {
    pub fn into_inner(self) -> [T; DIMENSIONS] {
        self.0
    }
    pub const fn inner(&self) -> &[T; DIMENSIONS] {
        &self.0
    }
    pub fn to_vector(self) -> Vector<DIMENSIONS, T> {
        Vector::new(self.into_inner())
    }
    #[inline(always)]
    pub fn length_squared(&self) -> T
    where
        T: Add<Output = T> + Mul<Output = T> + Clone,
    {
        self.clone().dot(self.clone())
    }
    #[inline(always)]
    pub fn dot<Usage2>(self, other: BaseVector<DIMENSIONS, T, Usage2>) -> T
    where
        T: Add<Output = T> + Mul<Output = T> + Clone,
    {
        (self.to_vector() * other.to_vector())
            .into_inner()
            .into_iter()
            .reduce(|acc, e| acc + e)
            .unwrap()
    }
    #[inline(always)]
    pub fn length<O>(&self) -> O
    where
        T: Add<Output = T> + Mul<Output = T> + Sqrt<O> + Clone,
    {
        self.length_squared().sqrt()
    }
}

impl<const DIMENSIONS: usize, T, Usage> New<[T; DIMENSIONS]> for BaseVector<DIMENSIONS, T, Usage> {
    #[inline(always)]
    default fn new(input: [T; DIMENSIONS]) -> Self {
        Self(input, PhantomData)
    }
}
impl<const DIMENSIONS: usize, T, Usage: VectorOrColor> BaseVector<DIMENSIONS, T, Usage> {
    // dont expost mutable references for Normals, NormalizedVectors etc.
    pub const fn inner_mut(&mut self) -> &mut [T; DIMENSIONS] {
        &mut self.0
    }
    /// Combine `self` with `other`, using `f`
    #[inline(always)]
    pub fn combine<F, O>(self, other: &Self, f: F) -> BaseVector<DIMENSIONS, O, Usage>
    where
        T: Clone,
        F: Fn(T, T) -> O,
        O: Clone,
    {
        BaseVector::new(array::from_fn(|index| {
            f(self.0[index].clone(), other.0[index].clone())
        }))
    }
}
impl<const DIMENSIONS: usize, T> Vector<DIMENSIONS, T> {
    #[inline(always)]
    pub fn normalize<O>(self) -> NormalizedVector<DIMENSIONS, O>
    where
        T: Add<Output = T> + Mul<Output = T> + Sqrt<O> + Clone + Convert<O>,
        O: Div<Output = O> + Clone,
    {
        NormalizedVector::new_unchecked((self.clone().convert() / self.length()).into_inner())
    }
    pub fn gram_schmidt(self, w: NormalizedVector<DIMENSIONS, T>) -> Self
    where
        T: Add<Output = T> + Mul<Output = T> + Sub<Output = T> + Clone,
    {
        let w = w.to_vector();
        self.clone() - w.clone() * self.dot(w)
    }
    /// Element-wise min
    #[inline(always)]
    pub fn min(self, other: &Self) -> Self
    where
        T: MinMax + Clone,
    {
        self.combine(other, MinMax::min)
    }
    /// Element-wise max
    #[inline(always)]
    pub fn max(self, other: &Self) -> Self
    where
        T: MinMax + Clone,
    {
        self.combine(other, MinMax::max)
    }
}
impl<T> Vector<3, T> {
    // TODO: maybe use difference_of_products (not yet implemented) to raise precision
    #[inline(always)]
    pub fn cross(self, other: Self) -> Self
    where
        T: Mul<Output: Sub<Output = T> + Clone> + Clone,
    {
        let yzx = |vector: Self| {
            let mut inner = vector.into_inner(); // xyz
            inner.swap(0, 2); // zyx
            inner.swap(0, 1); // yzx
            Self::new(inner)
        };
        let zxy = |vector: Self| {
            let mut inner = vector.into_inner(); // xyz
            inner.swap(0, 1); // yxz
            inner.swap(0, 2); // zxy
            Self::new(inner)
        };

        yzx(self.clone()) * zxy(other.clone()) - zxy(self) * yzx(other)
    }
}
impl<const DIMENSIONS: usize, T, Usage> Neg for BaseVector<DIMENSIONS, T, Usage>
where
    T: Neg,
{
    type Output = BaseVector<DIMENSIONS, T::Output, Usage>;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        BaseVector::new(self.0.map(Neg::neg))
    }
}
impl<const DIMENSIONS: usize, T> Default for Vector<DIMENSIONS, T>
where
    T: Copy + Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self::new([Default::default(); DIMENSIONS])
    }
}

impl<const DIMENSIONS: usize, T, Usage> Clone for BaseVector<DIMENSIONS, T, Usage>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<const DIMENSIONS: usize, T, Usage> Copy for BaseVector<DIMENSIONS, T, Usage> where T: Copy {}

impl<const DIMENSIONS: usize, T, Usage> Debug for BaseVector<DIMENSIONS, T, Usage>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector {:?}", self.0)
    }
}

impl<const DIMENSIONS: usize, Source> Vector<DIMENSIONS, Source> {
    #[inline(always)]
    fn convert<Target>(self) -> Vector<DIMENSIONS, Target>
    where
        Source: Convert<Target>,
    {
        Vector::new(self.0.map(Convert::convert))
    }
}
impl<const DIMENSIONS: usize, T: Float> Vector<DIMENSIONS, T> {
    #[inline(always)]
    pub fn is_normalized(&self) -> bool
    where
        f16: Convert<T>,
    {
        const TOLERANCE: f16 = 1e-5;
        self.length() <= (1. + TOLERANCE).convert() && self.length() >= (1. - TOLERANCE).convert()
    }
    #[inline(always)]
    pub fn near_zero(&self) -> bool {
        self.0.map(|e| e.abs() < T::EPSILON) == [true; _]
    }
    #[inline(always)]
    pub fn angle_between<O>(self, other: Self) -> T
    where
        u8: Convert<T>,
    {
        if self.dot(other) < 0.convert() {
            T::PI - 2.convert() * ((self + other).length::<T>() / 2.convert()).asin()
        } else {
            2.convert() * ((other - self).length::<T>() / 2.convert())
        }
    }
}
macro_rules! impl_vec_op {
    ($(($Trait:ident, $method:ident)),*) => {
        $(
            impl<const DIMENSIONS: usize, T, Usage: VectorOrColor> $Trait for BaseVector<DIMENSIONS, T, Usage>
            where
                T: $Trait + Clone,
                T::Output: Clone,
            {
                type Output = BaseVector<DIMENSIONS, T::Output, Usage>;
                #[inline(always)]
                fn $method(self, rhs: Self) -> Self::Output {
                    self.combine(&rhs, $Trait::$method)
                }
            }
            impl<const DIMENSIONS: usize, T, Usage: VectorOrColor> $Trait<T> for BaseVector<DIMENSIONS, T, Usage>
            where
                T: $Trait + Clone,
                T::Output: Clone,
            {
                type Output = BaseVector<DIMENSIONS, T::Output, Usage>;
                #[inline(always)]
                fn $method(self, rhs: T) -> Self::Output {
                    BaseVector::new(self.0.map(|e| e.$method(rhs.clone())))
                }
            }

            /// Normalized Vector
            impl<const DIMENSIONS: usize, T> $Trait for NormalizedVector<DIMENSIONS, T>
            where
                T: $Trait + Clone,
                T::Output: Clone,
            {
                type Output = Vector<DIMENSIONS, T::Output>;
                #[inline(always)]
                fn $method(self, rhs: Self) -> Self::Output {
                    self.to_vector().combine(&rhs.to_vector(), $Trait::$method)
                }
            }
            impl<const DIMENSIONS: usize, T, > $Trait<T> for NormalizedVector<DIMENSIONS, T>
            where
                T: $Trait + Clone,
                T::Output: Clone,
            {
                type Output = Vector<DIMENSIONS, T::Output>;
                #[inline(always)]
                fn $method(self, rhs: T) -> Self::Output {
                    BaseVector::new(self.0.map(|e| e.$method(rhs.clone())))
                }
            }
            impl<const DIMENSIONS: usize, T> $Trait<Vector<DIMENSIONS, T>> for NormalizedVector<DIMENSIONS, T>
            where
                T: $Trait + Clone,
                T::Output: Clone,
            {
                type Output = Vector<DIMENSIONS, T::Output>;
                #[inline(always)]
                fn $method(self, rhs: Vector<DIMENSIONS, T>) -> Self::Output {
                    self.to_vector().combine(&rhs, $Trait::$method)
                }
            }
        )*
    };
}
impl_vec_op!((Add, add), (Sub, sub), (Mul, mul), (Div, div));

macro_rules! access_vec {
    ($vector:ident, $($name:ident => $index:expr),*) => {
        $(
            impl<const DIMENSIONS: usize, T> $vector<DIMENSIONS, T>
            where
                // compile-time assertion on index (<)
                [(); DIMENSIONS - 1 - $index]:
            {
                #[inline(always)]
                pub const fn $name(&self) -> &T {
                    &self.0[$index]
                }
            }
        )*
    };
}
access_vec!(Vector, x => 0, y => 1, z => 2, w => 3);
access_vec!(NormalizedVector, x => 0, y => 1, z => 2, w => 3);
access_vec!(Color, r => 0, g => 1, b => 2, a => 3);

impl<const DIMENSIONS: usize, T> Color<DIMENSIONS, T> {
    /// gamma 2 correction
    #[inline(always)]
    pub fn color_correct(self) -> Self
    where
        T: Sqrt,
    {
        Self(self.0.map(Sqrt::sqrt), PhantomData)
    }
}
/// Converts natural -> float Colors (0..MAX -> 0.0..1.0).
impl<const DIMENSIONS: usize, N: Natural> Color<DIMENSIONS, N> {
    /// Converts natural -> float Colors (0..MAX -> 0.0..1.0).
    #[inline(always)]
    pub fn to_float_color<F: Float>(self) -> Color<DIMENSIONS, F>
    where
        N: Convert<F>,
    {
        Color::new(self.0.map(|natural| natural.convert() / N::MAX.convert()))
    }
}
impl<const DIMENSIONS: usize, F: Float> Color<DIMENSIONS, F>
where
    f16: Convert<F>,
{
    /// Converts float -> natural Colors ( 0.0..1.0 -> 0..MAX).
    #[inline(always)]
    pub fn to_natural_color<N: Convert<F> + Natural>(self) -> Color<DIMENSIONS, N>
    where
        F: Convert<N>,
    {
        Color::new(self.0.map(|float| {
            debug_assert!(float >= 0.0.convert());
            debug_assert!(float <= 1.0.convert());

            (float * N::MAX.convert()).convert()
        }))
    }
}

pub type Vec3 = Vector<3, f32>;

impl<Usage: VectorOrColor, T: FromStr<Err: Debug>> From<&str> for BaseVector<3, T, Usage> {
    fn from(value: &str) -> Self {
        let mut values = value.split(' ').map(|value| value.parse().unwrap());

        Self::new(array::from_fn(|_| values.next().unwrap()))
    }
}

impl<const DIMENSIONS: usize, T> Random for NormalizedVector<DIMENSIONS, T>
where
    T: Div<Output = T> + Copy + Sqrt<T> + Add<Output = T> + Mul<Output = T>,
    f32: Convert<T>,
{
    #[inline(always)]
    fn random() -> Self {
        Vector::new(array::from_fn(|_| (f32::random() - 0.5).convert())).normalize()
    }
}
impl<const DIMENSIONS: usize, T> NormalizedVector<DIMENSIONS, T> {
    pub const fn new_unchecked(vector: [T; DIMENSIONS]) -> Self {
        Self(vector, PhantomData)
    }
    #[inline(always)]
    pub fn reflect(self, normal: Self) -> Self
    where
        T: Mul<Output = T> + Clone + Add<Output = T> + Sub<Output = T>,
        u8: Convert<T>,
    {
        let this = self.to_vector();
        let normal = normal.to_vector();

        Self::new_unchecked(
            (this.clone() - normal.clone() * 2.convert() * this.dot(normal)).into_inner(),
        )
    }
}
impl<const DIMENSIONS: usize, T: Float> New<[T; DIMENSIONS]> for NormalizedVector<DIMENSIONS, T>
where
    f16: Convert<T>,
{
    #[inline(always)]
    fn new(input: [T; DIMENSIONS]) -> Self {
        debug_assert!(
            Vector::new(input).is_normalized(),
            "vector: {input:?}, len: {:?}",
            Vector::new(input).length::<T>()
        );

        Self(input, PhantomData)
    }
}
impl<const DIMENSIONS: usize, T: Float> New<Vector<DIMENSIONS, T>>
    for NormalizedVector<DIMENSIONS, T>
where
    f16: Convert<T>,
{
    #[inline(always)]
    fn new(input: Vector<DIMENSIONS, T>) -> Self {
        debug_assert!(
            input.is_normalized(),
            "vector: {input:?}, len: {:?}",
            input.length::<T>()
        );

        Self(input.into_inner(), PhantomData)
    }
}
impl<T: Float> NormalizedVector<3, T>
where
    i8: Convert<T>,
{
    #[inline(always)]
    pub fn coordinate_system(self) -> [Self; 2] {
        let sign = T::copysign(1.convert(), *self.z());
        let a = (-1).convert() / (sign + *self.z());
        let b = *self.x() * *self.y() * a;

        [
            Self::new_unchecked([
                1.convert() + sign * self.x().sqrt() * a,
                sign * b,
                -sign * *self.x(),
            ]),
            Self::new_unchecked([b, sign + self.y().sqrt() * a, -*self.y()]),
        ]
    }
    #[inline(always)]
    // A unit vector pointing in the given spherical direction
    pub fn spherical_direction(sin_theta: T, cos_theta: T, phi: T) -> Self {
        {
            let lower = (-1).convert();
            let upper = 1.convert();
            debug_assert!(
                lower < sin_theta && sin_theta <= upper && lower < cos_theta && cos_theta <= upper,
                "sin_theta: {sin_theta:?}, cos_theta: {cos_theta:?}"
            );
        }
        Self::new([sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta])
    }
}

pub type NormalizedVec3 = NormalizedVector<3, f32>;
