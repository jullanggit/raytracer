use std::array;

use crate::vec3::{AsConvert, Float, literal_to_float};

pub struct SquareMatrix<const N: usize, T: Float>([[T; N]; N]);
impl<const N: usize, T: Float> SquareMatrix<N, T> {
    pub fn identity() -> Self {
        Self(array::from_fn(|i| array::from_fn(|j| (i == j).into())))
    }
    pub fn zero() -> Self
    where
        f128: AsConvert<T>,
    {
        let zero = literal_to_float(0.0);
        Self([[zero; N]; N])
    }
}
impl<const N: usize, T: Float> Default for SquareMatrix<N, T> {
    fn default() -> Self {
        Self::identity()
    }
}

pub struct Transform {
    m: SquareMatrix<4, f32>,
    inv_m: SquareMatrix<4, f32>,
}
