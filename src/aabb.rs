use std::ops::{Add, Mul, Sub};

use crate::vec3::{MinMax, Sqrt, Vector};

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
}

pub trait Union<T> {
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
