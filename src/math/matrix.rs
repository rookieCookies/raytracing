use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Sub};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Matrix<const ROW: usize, const COLUMN: usize, T> {
    rows: [[T; COLUMN]; ROW]
}


impl<const ROW: usize, const COLUMN: usize, T> Matrix<ROW, COLUMN, T> {
    pub const IDENTITY : Matrix<4, 4, f64> = Matrix {
        rows: [[1.0, 0.0, 0.0, 0.0],
               [0.0, 1.0, 0.0, 0.0],
               [0.0, 0.0, 1.0, 0.0],
               [0.0, 0.0, 0.0, 1.0]],
    };

    pub fn new(rows: [[T; COLUMN]; ROW]) -> Self {
        Self {
            rows,
        }
    }
}



impl<const ROW: usize, const COLUMN: usize, T: Copy> Matrix<ROW, COLUMN, T> {
    pub fn scale<V, A: Copy + Mul<T, Output = V>>(self, scale_factor: A) -> Matrix<ROW, COLUMN, V> {
        let arr = std::array::from_fn::<[V; COLUMN], ROW, _>(|i| {
            std::array::from_fn::<V, COLUMN, _>(|j| {
                scale_factor * self.rows[i][j] 
            })
        });

        Matrix::new(arr)
    }
}


impl<const ROW: usize, const COLUMN: usize, V, T: Add<Output=V> + Copy> Add for Matrix<ROW, COLUMN, T> {
    type Output = Matrix<ROW, COLUMN, V>;

    fn add(self, rhs: Self) -> Self::Output {
        let arr = std::array::from_fn::<[V; COLUMN], ROW, _>(|i| {
            std::array::from_fn::<V, COLUMN, _>(|j| {
                self.rows[i][j] + rhs.rows[i][j]
            })
        });

        Matrix::new(arr)
        
    }
}


impl<const ROW: usize, const COLUMN: usize, V, T: Sub<Output=V> + Copy> Sub for Matrix<ROW, COLUMN, T> {
    type Output = Matrix<ROW, COLUMN, V>;

    fn sub(self, rhs: Self) -> Self::Output {
        let arr = std::array::from_fn::<[V; COLUMN], ROW, _>(|i| {
            std::array::from_fn::<V, COLUMN, _>(|j| {
                self.rows[i][j] - rhs.rows[i][j]
            })
        });

        Matrix::new(arr)
        
    }
}


impl<const ROW: usize, const COLUMN: usize, const COLUMN_TWO: usize, V: AddAssign, T: Mul<Output=V> + Copy> Mul<Matrix<COLUMN, COLUMN_TWO, T>> for Matrix<ROW, COLUMN, T> {
    type Output = Matrix<COLUMN, COLUMN_TWO, V>;

    fn mul(self, rhs: Matrix<COLUMN, COLUMN_TWO, T>) -> Self::Output {
        let arr = std::array::from_fn::<[V; COLUMN_TWO], COLUMN, _>(|i| {
            std::array::from_fn::<V, COLUMN_TWO, _>(|j| {
                let mut res = None;
                for k in 0..COLUMN {
                    let r = self.rows[i][k] * rhs.rows[k][j];
                    if let Some(res) = &mut res {
                        *res += r;
                    } else {
                        res = Some(r);
                    }
                }
                res.unwrap()
            })
        });

        Matrix::new(arr)
        
    }
}


impl<const ROW: usize, const COLUMN: usize, T> Index<usize> for Matrix<ROW, COLUMN, T> {
    type Output = [T; COLUMN];

    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[index]
    }
}

impl<const ROW: usize, const COLUMN: usize, T> IndexMut<usize> for Matrix<ROW, COLUMN, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.rows[index]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_addition() {
        let m1 = Matrix::new([
            [1, 2],
            [3, 4],
        ]);

        let m2 = Matrix::new([
            [5, 6],
            [7, 8],
        ]);


        let m3 = Matrix::new([
            [6 , 8 ],
            [10, 12],
        ]);


        assert_eq!(m1 + m2, m3)
    }


    #[test]
    fn matrix_sub() {
        let m1 = Matrix::new([
            [4, 2],
            [1, 6],
        ]);

        let m2 = Matrix::new([
            [2, 4],
            [0, 1],
        ]);


        let m3 = Matrix::new([
            [2, -2],
            [1,  5],
        ]);


        assert_eq!(m1 - m2, m3)
    }


    #[test]
    fn matrix_scale() {
        let m1 = Matrix::new([
            [1, 2],
            [3, 4],
        ]);


        let m2 = Matrix::new([
            [2, 4],
            [6, 8],
        ]);


        assert_eq!(m1.scale(2), m2);
    }


    #[test]
    fn matrix_multiplication() {
        let m1 = Matrix::new([
            [1, 2],
            [3, 4],
        ]);

        let m2 = Matrix::new([
            [5, 6],
            [7, 8],
        ]);


        let m3 = Matrix::new([
            [19, 22],
            [43, 50],
        ]);


        assert_eq!(m1 * m2, m3);


        let m1 = Matrix::new([
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]);

        let m2 = Matrix::new([
            [5],
            [7],
            [3],
            [5],
        ]);

        assert_eq!(m1 * m2, m2)
    }


}
