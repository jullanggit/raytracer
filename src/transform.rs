use std::{
    array,
    fmt::Debug,
    ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{
    convert::Convert,
    vec3::{New as _, Point},
};

#[derive(Debug, PartialEq)]
pub struct SquareMatrix<const N: usize, T>([[T; N]; N]);
impl<const N: usize, T> SquareMatrix<N, T> {
    pub fn identity() -> Self
    where
        T: From<bool>,
    {
        Self(array::from_fn(|i| array::from_fn(|j| (i == j).into())))
    }
    pub fn zero() -> Self
    where
        T: Copy,
        u8: Convert<T>,
    {
        Self([[0.convert(); N]; N])
    }
    pub fn determinant(mut self) -> T
    where
        T: PartialEq + Clone + Div<Output = T> + Mul<Output = T> + SubAssign + MulAssign,
        u8: Convert<T>,
    {
        let zero = 0.convert();
        let mut sign = true;

        for i in 0..N {
            // find pivot row
            let mut pivot_row = i;
            while i < N && self.0[pivot_row][i] == zero {
                pivot_row += 1;
            }
            // entire row is zeroes
            if pivot_row == N {
                return zero;
            }

            // swap rows
            if pivot_row != i {
                self.0.swap(i, pivot_row);
                // swap sign
                sign = !sign;
            }

            let pivot = self[i][i].clone();

            // eliminate rows below
            for j in (i + 1)..N {
                let factor = self[i][j].clone() / pivot.clone();
                for k in i..N {
                    let diff = factor.clone() * self[i][k].clone();
                    self[j][k] -= diff;
                }
            }
        }
        let mut determinant = if sign { 1.convert() } else { zero };
        for i in 0..N {
            determinant *= self[i][i].clone();
        }
        determinant
    }
    pub fn transpose(&self) -> Self
    where
        T: Copy,
        u8: Convert<T>,
    {
        let mut out = Self::zero();
        for i in 0..N {
            for j in 0..N {
                out[j][i] = self[i][i];
            }
        }
        out
    }
    /// If inversion does not work, returns self in the error case.
    pub fn inverse(mut self) -> Option<Self>
    where
        T: PartialEq + DivAssign + Mul<Output = T> + SubAssign + From<bool> + Clone,
        u8: Convert<T>,
    {
        let mut inv = Self::identity();

        for i in 0..N {
            // Find pivot
            let mut pivot_row = i;
            while pivot_row < N && self[pivot_row][i] == 0.convert() {
                pivot_row += 1;
            }
            if pivot_row == N {
                return None; // Singular matrix
            }

            // Swap rows in both mat and inv
            if pivot_row != i {
                self.swap(i, pivot_row);
                inv.swap(i, pivot_row);
            }

            // Scale pivot row to 1
            let pivot = self[i][i].clone();
            for j in 0..N {
                self[i][j] /= pivot.clone();
                inv[i][j] /= pivot.clone();
            }

            // Eliminate other rows
            for j in 0..N {
                if j != i {
                    let factor = self[j][i].clone();
                    for k in 0..N {
                        let self_diff = factor.clone() * self[i][k].clone();
                        let inv_diff = factor.clone() * inv[i][k].clone();
                        self[j][k] -= self_diff;
                        inv[j][k] -= inv_diff;
                    }
                }
            }
        }

        Some(inv)
    }
}
impl<const N: usize, T> Default for SquareMatrix<N, T>
where
    T: From<bool>,
{
    fn default() -> Self {
        Self::identity()
    }
}
macro_rules! implMatrixScalarOps {
    ($($Trait:ident, $method:ident),+) => {
        $(
            impl<const N: usize, T> $Trait<T> for SquareMatrix<N, T>
            where
                T: $Trait<Output: Copy> + Copy,
                u8: Convert<T::Output>,
            {
                type Output = SquareMatrix<N, T::Output>;

                fn $method(self, rhs: T) -> Self::Output {
                    let mut out = Self::Output::zero();
                    for i in 0..N {
                        for j in 0..N {
                            out[i][j] = self[i][j].$method(rhs);
                        }
                    }
                    out
                }
            }
        )+
    };
}
implMatrixScalarOps!(Add, add, Sub, sub, Mul, mul, Div, div);

impl<const N: usize, T> Deref for SquareMatrix<N, T> {
    type Target = [[T; N]; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<const N: usize, T> DerefMut for SquareMatrix<N, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<const N: usize, T> Mul<Point<N, T>> for SquareMatrix<N, T>
where
    T: AddAssign + Clone + Mul<Output = T>,
    u8: Convert<T>,
{
    type Output = Point<N, T>;

    fn mul(self, rhs: Point<N, T>) -> Self::Output {
        let mut out: [T; _] = array::from_fn(|_| 0.convert());
        for i in 0..N {
            for j in 0..N {
                out[i] += self[i][j].clone() * rhs.inner()[j].clone();
            }
        }
        Point::new(out)
    }
}
impl<const N: usize, T> Clone for SquareMatrix<N, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Transform<const N: usize, T> {
    m: SquareMatrix<N, T>,
    inv_m: SquareMatrix<N, T>,
}
impl<const N: usize, T> Transform<N, T> {
    /// Construct a Transform given a matrix and its inverse. Correct inverse will be checked in debug mode.
    pub fn new(m: SquareMatrix<N, T>, inv_m: SquareMatrix<N, T>) -> Self
    where
        T: Clone + Debug + PartialEq + DivAssign + Mul<Output = T> + SubAssign + From<bool>,
        u8: Convert<T>,
    {
        debug_assert_eq!(m.clone().inverse(), Some(inv_m.clone()));

        Self { m, inv_m }
    }
    /// Construct a Transform given a matrix and its inverse, without checking it.
    pub const fn new_unchecked(m: SquareMatrix<N, T>, inv_m: SquareMatrix<N, T>) -> Self {
        Self { m, inv_m }
    }
    pub fn invert(self) -> Self {
        Self {
            m: self.inv_m,
            inv_m: self.m,
        }
    }
    pub fn transpose(self) -> Self
    where
        T: Copy,
        u8: Convert<T>,
    {
        Self {
            m: self.m.transpose(),
            inv_m: self.inv_m.transpose(),
        }
    }
}
impl<const N: usize, T> Transform<N, T> {
    pub fn translate(delta: Point<{ N - 1 }, T>) -> Self
    where
        T: From<bool> + Copy + Neg<Output = T>,
    {
        let mut out = SquareMatrix::default();
        let mut out_inv = SquareMatrix::default();
        for i in 0..N - 1 {
            out[i][N - 1] = delta.inner()[i];
            out_inv[i][N - 1] = -delta.inner()[i];
        }

        Self {
            m: out,
            inv_m: out_inv,
        }
    }
    pub fn scale(scale: Point<{ N - 1 }, T>) -> Self
    where
        T: From<bool> + Copy + Div<Output = T>,
        u8: Convert<T>,
    {
        let mut out = SquareMatrix::default();
        let mut out_inv = SquareMatrix::default();
        for i in 0..N - 1 {
            out[i][i] = scale.inner()[i];
            out_inv[i][i] = 1.convert() / scale.inner()[i];
        }

        Self {
            m: out,
            inv_m: out_inv,
        }
    }
}
impl<const N: usize, T> Default for Transform<N, T>
where
    T: From<bool>,
{
    /// Identity
    fn default() -> Self {
        Self {
            m: SquareMatrix::default(),
            inv_m: SquareMatrix::default(),
        }
    }
}
impl<const N: usize, T> TryFrom<SquareMatrix<N, T>> for Transform<N, T>
where
    T: Copy + PartialEq + DivAssign + Mul<Output = T> + SubAssign + From<bool>,
    u8: Convert<T>,
{
    type Error = &'static str;
    fn try_from(value: SquareMatrix<N, T>) -> Result<Self, Self::Error> {
        Ok(Self {
            m: value.clone(),
            inv_m: value.inverse().ok_or("Failed to invert matrix")?,
        })
    }
}
