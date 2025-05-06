use crate::vec3::{Lerp, MinMax, Sqrt, Vector};
use std::{
    array,
    cmp::Ordering,
    ops::{Add, Div, Mul, Sub},
};

pub struct Aabb<const DIMENSIONS: usize, T: Copy> {
    min: Vector<DIMENSIONS, T>,
    max: Vector<DIMENSIONS, T>,
}
impl<const DIMENSIONS: usize, T: Copy> Aabb<DIMENSIONS, T> {
    pub fn new(min: Vector<DIMENSIONS, T>, max: Vector<DIMENSIONS, T>) -> Self
    where
        T: MinMax,
    {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }
    /// Region of intersection between self and other
    pub fn intersection(&self, other: &Self) -> Self
    where
        T: MinMax,
    {
        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }
    /// Whether self and other overlap
    pub fn overlaps(&self, other: &Self) -> bool
    where
        T: PartialOrd,
    {
        (0..DIMENSIONS).all(|index| {
            self.max.0[index] >= other.min.0[index] && self.min.0[index] <= other.max.0[index]
        })
    }
    pub fn contains(&self, point: Vector<DIMENSIONS, T>) -> bool
    where
        T: PartialOrd,
    {
        (0..DIMENSIONS)
            .all(|index| point.0[index] >= self.min.0[index] && point.0[index] <= self.max.0[index])
    }
    /// upper boundaries not included
    pub fn contains_exclusive(&self, point: Vector<DIMENSIONS, T>) -> bool
    where
        T: PartialOrd,
    {
        (0..DIMENSIONS)
            .all(|index| point.0[index] >= self.min.0[index] && point.0[index] < self.max.0[index])
    }
    pub fn distance_squared(&self, point: Vector<DIMENSIONS, T>) -> T
    where
        T: Add<Output = T> + MinMax + Sub<Output: Clone + From<u8> + MinMax + Mul<Output = T>>,
    {
        (0..DIMENSIONS)
            .map(|index| {
                (<T as Sub>::Output::from(0))
                    .max(self.min.0[index] - point.0[index])
                    .max(point.0[index] - self.max.0[index])
            })
            .map(|value| value.clone() * value)
            .reduce(|acc, e| acc + e)
            .unwrap()
    }
    pub fn distance<O>(&self, point: Vector<DIMENSIONS, T>) -> O
    where
        T: Add<Output = T>
            + MinMax
            + Sqrt<O>
            + Sub<Output: Clone + From<u8> + MinMax + Mul<Output = T>>,
    {
        self.distance_squared(point).sqrt()
    }
    /// Pad by delta in all dimensions
    pub fn expand(&mut self, delta: T)
    where
        T: Add<Output = T> + Sub<Output = T>,
    {
        let delta = Vector([delta; _]);
        self.min = self.min - delta;
        self.max = self.max + delta;
    }
    pub fn diagonal(&self) -> Vector<DIMENSIONS, <T as Sub>::Output>
    where
        T: Sub<Output: Copy>,
    {
        self.max - self.min
    }
    /// index of the dimensions with the biggest value
    pub fn max_dimension(&self) -> <T as Sub>::Output
    where
        T: Sub<Output: Copy + PartialOrd>,
    {
        let d = self.diagonal();
        d.0.into_iter()
            .max_by(|e1, e2| e1.partial_cmp(e2).unwrap_or(Ordering::Equal))
            .unwrap()
    }
    /// Interpolate between the corners by t dimension-wise
    pub fn lerp<X>(&self, t: Vector<DIMENSIONS, X>) -> Vector<DIMENSIONS, <T as Lerp<X>>::Output>
    where
        T: Lerp<X, Output: Copy>,
        X: Copy,
    {
        Vector(array::from_fn(|index| {
            self.min.0[index].lerp(self.max.0[index], t.0[index])
        }))
    }
    /// The position of `point` relative to the corners.
    /// min = 0, max = 1
    pub fn offset(&self, point: Vector<DIMENSIONS, T>) -> Vector<DIMENSIONS, T>
    where
        T: Sub<Output = T> + PartialOrd + Div<Output = T>,
    {
        let mut out = point - self.min;
        for index in 0..DIMENSIONS {
            if self.max.0[index] > self.min.0[index] {
                out.0[index] = out.0[index] / (self.max.0[index] - self.min.0[index]);
            }
        }
        out
    }
    pub fn is_empty(&self) -> bool
    where
        T: PartialOrd,
    {
        (0..DIMENSIONS).any(|index| self.min.0[index] >= self.max.0[index])
    }
}
impl<T: Copy> Aabb<2, T> {
    pub fn area(&self) -> <<T as Sub>::Output as Mul>::Output
    where
        T: Sub<Output: Copy + Mul>,
    {
        let d = self.diagonal();
        d.x() * d.y()
    }
}
impl<T: Copy> Aabb<3, T> {
    pub fn corner(&self, corner: usize) -> Vector<3, T> {
        let mut bit = 1;
        Vector([Vector::x, Vector::y, Vector::z].map(|f| {
            let v = f(if corner & bit == 0 {
                &self.min
            } else {
                &self.max
            });
            bit *= 2;
            v
        }))
    }
    pub fn surface_area(&self) -> T
    where
        T: Add<Output = T> + From<u8> + Mul<Output = T> + Sub<Output = T>,
    {
        let d = self.diagonal();
        T::from(2) * (d.x() * d.y() + d.x() * d.z() + d.y() * d.z())
    }
    pub fn volume(&self) -> <T as Sub>::Output
    where
        T: Sub<Output: Copy + Mul<Output = <T as Sub>::Output>>,
    {
        let d = self.diagonal();
        d.x() * d.y() * d.z()
    }
}

pub trait Union<T> {
    /// Grow the bounding box to include `value`
    fn union(&mut self, value: T);
}
impl<const DIMENSIONS: usize, T: Copy + MinMax> Union<Vector<DIMENSIONS, T>>
    for Aabb<DIMENSIONS, T>
{
    fn union(&mut self, value: Vector<DIMENSIONS, T>) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}
impl<const DIMENSIONS: usize, T: Copy + MinMax> Union<Self> for Aabb<DIMENSIONS, T> {
    fn union(&mut self, value: Self) {
        self.min = self.min.min(value.min);
        self.max = self.max.max(value.max);
    }
}
