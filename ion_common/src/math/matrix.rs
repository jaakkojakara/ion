use std::ops::Mul;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4 {
    data: [[f32; 4]; 4],
}

impl Matrix4x4 {
    pub fn new(values: [[f32; 4]; 4]) -> Self {
        Self { data: values }
    }

    #[inline]
    pub fn raw(&self) -> [[f32; 4]; 4] {
        self.data
    }

    #[inline]
    pub fn zeros() -> Self {
        Matrix4x4::new([
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ])
    }

    #[inline]
    pub fn ones() -> Self {
        Matrix4x4::new([
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ])
    }

    #[inline]
    pub fn identity() -> Self {
        Matrix4x4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    #[inline]
    pub fn multiply(self, other: Matrix4x4) -> Matrix4x4 {
        Self {
            data: [
                [
                    (0..4).map(|k| other.data[0][k] * self.data[k][0]).sum(),
                    (0..4).map(|k| other.data[0][k] * self.data[k][1]).sum(),
                    (0..4).map(|k| other.data[0][k] * self.data[k][2]).sum(),
                    (0..4).map(|k| other.data[0][k] * self.data[k][3]).sum(),
                ],
                [
                    (0..4).map(|k| other.data[1][k] * self.data[k][0]).sum(),
                    (0..4).map(|k| other.data[1][k] * self.data[k][1]).sum(),
                    (0..4).map(|k| other.data[1][k] * self.data[k][2]).sum(),
                    (0..4).map(|k| other.data[1][k] * self.data[k][3]).sum(),
                ],
                [
                    (0..4).map(|k| other.data[2][k] * self.data[k][0]).sum(),
                    (0..4).map(|k| other.data[2][k] * self.data[k][1]).sum(),
                    (0..4).map(|k| other.data[2][k] * self.data[k][2]).sum(),
                    (0..4).map(|k| other.data[2][k] * self.data[k][3]).sum(),
                ],
                [
                    (0..4).map(|k| other.data[3][k] * self.data[k][0]).sum(),
                    (0..4).map(|k| other.data[3][k] * self.data[k][1]).sum(),
                    (0..4).map(|k| other.data[3][k] * self.data[k][2]).sum(),
                    (0..4).map(|k| other.data[3][k] * self.data[k][3]).sum(),
                ],
            ],
        }
    }

    #[inline]
    pub fn multiply_vec4(self, vec: [f32; 4]) -> [f32; 4] {
        [
            self.data[0][0] * vec[0]
                + self.data[1][0] * vec[1]
                + self.data[2][0] * vec[2]
                + self.data[3][0] * vec[3],
            self.data[0][1] * vec[0]
                + self.data[1][1] * vec[1]
                + self.data[2][1] * vec[2]
                + self.data[3][1] * vec[3],
            self.data[0][2] * vec[0]
                + self.data[1][2] * vec[1]
                + self.data[2][2] * vec[2]
                + self.data[3][2] * vec[3],
            self.data[0][3] * vec[0]
                + self.data[1][3] * vec[1]
                + self.data[2][3] * vec[2]
                + self.data[3][3] * vec[3],
        ]
    }

    #[inline]
    pub fn inverse(self) -> Option<Matrix4x4> {
        let adjugate = self.adjugate();
        let det = self.determinant();

        if det.abs() < f32::EPSILON {
            return None; // Singular matrix, no inverse
        }

        Some(Self {
            data: [
                [
                    adjugate.data[0][0] / det,
                    adjugate.data[1][0] / det,
                    adjugate.data[2][0] / det,
                    adjugate.data[3][0] / det,
                ],
                [
                    adjugate.data[0][1] / det,
                    adjugate.data[1][1] / det,
                    adjugate.data[2][1] / det,
                    adjugate.data[3][1] / det,
                ],
                [
                    adjugate.data[0][2] / det,
                    adjugate.data[1][2] / det,
                    adjugate.data[2][2] / det,
                    adjugate.data[3][2] / det,
                ],
                [
                    adjugate.data[0][3] / det,
                    adjugate.data[1][3] / det,
                    adjugate.data[2][3] / det,
                    adjugate.data[3][3] / det,
                ],
            ],
        })
    }

    #[inline]
    pub fn determinant(self) -> f32 {
        let m = &self.data;

        let sub_det_0 = m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]);

        let sub_det_1 = m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]);

        let sub_det_2 = m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        let sub_det_3 = m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
            + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        m[0][0] * sub_det_0 - m[0][1] * sub_det_1 + m[0][2] * sub_det_2 - m[0][3] * sub_det_3
    }

    #[inline]
    pub fn adjugate(self) -> Matrix4x4 {
        let m = &self.data;
        let mut adj = Matrix4x4 {
            data: [[0.0; 4]; 4],
        };

        adj.data[0][0] = m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]);

        adj.data[0][1] = -(m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]));

        adj.data[0][2] = m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        adj.data[0][3] = -(m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
            + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]));

        adj.data[1][0] = -(m[0][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[0][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[0][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]));

        adj.data[1][1] = m[0][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[0][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[0][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]);

        adj.data[1][2] = -(m[0][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            - m[0][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[0][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]));

        adj.data[1][3] = m[0][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
            - m[0][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
            + m[0][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        adj.data[2][0] = m[0][1] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
            - m[0][2] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
            + m[0][3] * (m[1][1] * m[3][2] - m[1][2] * m[3][1]);

        adj.data[2][1] = -(m[0][0] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
            - m[0][2] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
            + m[0][3] * (m[1][0] * m[3][2] - m[1][2] * m[3][0]));

        adj.data[2][2] = m[0][0] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
            - m[0][1] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
            + m[0][3] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]);

        adj.data[2][3] = -(m[0][0] * (m[1][1] * m[3][2] - m[1][2] * m[3][1])
            - m[0][1] * (m[1][0] * m[3][2] - m[1][2] * m[3][0])
            + m[0][2] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]));

        adj.data[3][0] = -(m[0][1] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
            - m[0][2] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
            + m[0][3] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]));

        adj.data[3][1] = m[0][0] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
            - m[0][2] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
            + m[0][3] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);

        adj.data[3][2] = -(m[0][0] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
            + m[0][3] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]));

        adj.data[3][3] = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

        adj
    }
}

impl Mul<Matrix4x4> for Matrix4x4 {
    type Output = Matrix4x4;

    #[inline]
    fn mul(self, rhs: Matrix4x4) -> Self::Output {
        self.multiply(rhs)
    }
}

impl Mul<[f32; 4]> for Matrix4x4 {
    type Output = [f32; 4];

    #[inline]
    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        self.multiply_vec4(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeros_matrix_works() {
        let zeros = Matrix4x4::zeros();

        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(zeros.data[i][j], 0.0);
            }
        }
    }

    #[test]
    fn ones_matrix_works() {
        let ones = Matrix4x4::ones();

        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(ones.data[i][j], 1.0);
            }
        }
    }

    #[test]
    fn identity_matrix_works() {
        let identity = Matrix4x4::identity();

        for i in 0..4 {
            for j in 0..4 {
                if i == j {
                    assert_eq!(identity.data[i][j], 1.0);
                } else {
                    assert_eq!(identity.data[i][j], 0.0);
                }
            }
        }

        // Multiplying any matrix by identity should return the original matrix
        let test_mat = Matrix4x4::new([
            [3.0, 2.0, 3.4, 1.5],
            [3.0, 1.0, 3.4, -1.5],
            [-2.0, 2.0, 3.4, 1.5],
            [6.4, 2.0, -3.4, 8.5],
        ]);

        assert_eq!(test_mat * identity, test_mat);
        assert_eq!(identity * test_mat, test_mat);
    }

    #[test]
    fn determinant_works() {
        let identity = Matrix4x4::identity();
        assert_eq!(identity.determinant(), 1.0);

        let zeros = Matrix4x4::zeros();
        assert_eq!(zeros.determinant(), 0.0);

        // Test with a matrix with known determinant
        let mat = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);

        assert_eq!(mat.determinant(), 0.0); // This matrix is singular
    }

    #[test]
    fn adjugate_works() {
        let identity = Matrix4x4::identity();
        let adj_identity = identity.adjugate();

        // Adjugate of identity is identity
        assert_eq!(adj_identity, identity);

        // Test with a more complex matrix
        let mat = Matrix4x4::new([
            [1.0, 2.0, 0.0, 0.0],
            [3.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 5.0, 6.0],
            [0.0, 0.0, 7.0, 8.0],
        ]);

        let expected = Matrix4x4::new([
            [-8.0, 6.0, 0.0, -0.0],
            [4.0, -2.0, -0.0, 0.0],
            [0.0, -0.0, -16.0, 14.0],
            [-0.0, 0.0, 12.0, -10.0],
        ]);

        assert_eq!(mat.adjugate(), expected);
    }

    #[test]
    fn matrix_multiplication_works() {
        let mat_1 = Matrix4x4::new([
            [3.0, 2.0, 3.4, 1.5],
            [3.0, 1.0, 3.4, -1.5],
            [-2.0, 2.0, 3.4, 1.5],
            [6.4, 2.0, -3.4, 8.5],
        ]);
        let mat_2 = Matrix4x4::new([
            [3.4, 2.5, 1.4, 1.5],
            [-3.0, 1.5, 3.4, -1.5],
            [-2.0, 4.0, 1.4, 2.5],
            [6.4, 2.5, -5.4, 1.5],
        ]);

        let res = Matrix4x4::new([
            [24.5, 15.1, 19.720001, 16.2],
            [-20.900002, -0.6999998, 11.56, -14.4],
            [19.2, 7.8, 3.0600004, 14.35],
            [47.1, 7.5, 6.799999, 10.5],
        ]);

        assert_eq!(mat_1 * mat_2, res);
    }

    #[test]
    fn matrix_inversion_works() {
        let mat_1 = Matrix4x4::new([
            [3.0, 2.0, 3.4, 1.5],
            [3.0, 1.0, 3.4, -1.5],
            [-2.0, 2.0, 3.4, 1.5],
            [6.4, 2.0, -3.4, 8.5],
        ]);

        let res = Matrix4x4::new([
            [0.2000001, 1.1219706e-7, -0.20000017, -2.8049264e-8],
            [-6.3200045, 5.0000033, 2.8200018, 1.500001],
            [2.7588253, -2.058825, -1.0676477, -0.66176516],
            [2.4400017, -2.0000014, -0.9400006, -0.50000036],
        ]);

        assert_eq!(mat_1.inverse().unwrap(), res)
    }

    #[test]
    fn matrix_vec_multiplication_works() {
        let mat = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);

        let vec = [2.0, 3.0, 4.0, 5.0];

        let expected = [118.0, 132.0, 146.0, 160.0];
        assert_eq!(mat * vec, expected);
    }

    #[test]
    fn inverse_fails_for_singular_matrix() {
        // Create a singular matrix (determinant = 0)
        let singular = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);

        assert_eq!(singular.inverse(), None);
    }

    #[test]
    fn inverse_identity_is_identity() {
        let identity = Matrix4x4::identity();
        let inverted = identity.inverse().unwrap();

        assert_eq!(inverted, identity);
    }

    #[test]
    fn matrix_multiplied_by_inverse_gives_identity() {
        let mat = Matrix4x4::new([
            [3.0, 2.0, 3.4, 1.5],
            [3.0, 1.0, 3.4, -1.5],
            [-2.0, 2.0, 3.4, 1.5],
            [6.4, 2.0, -3.4, 8.5],
        ]);

        let inv = mat.inverse().unwrap();
        let result = mat * inv;

        // Check if it's close to identity (due to floating point precision)
        for i in 0..4 {
            for j in 0..4 {
                if i == j {
                    assert!((result.data[i][j] - 1.0).abs() < 1e-5);
                } else {
                    assert!(result.data[i][j].abs() < 1e-5);
                }
            }
        }
    }
}
