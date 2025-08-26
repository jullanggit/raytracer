pub struct Matrix<const N: usize, const M: usize, T>([[T; N]; M]);
pub type SquareMatrix<const N: usize, T> = Matrix<N, N, T>;

pub struct Transform {
    m: SquareMatrix<4, f32>,
    inv_m: SquareMatrix<4, f32>,
}
