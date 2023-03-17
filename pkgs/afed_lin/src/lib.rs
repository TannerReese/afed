use vector::Vector;
use matrix::Matrix;

use afed_objects::{call, declare_pkg, array::Array, Object};

mod aug_matrix;
pub mod matrix;
pub mod vector;

declare_pkg! {lin:
    /// lin.V (comps: array) -> vector
    /// Construct a vector with the components 'comps'
    #[allow(non_snake_case)]
    #[global]
    fn V(comps: Array) -> Result<Vector, &'static str> {
        if !comps.0.is_empty() { Ok(Vector::new(comps.0)) }
        else { Err("Vector cannot be zero dimensional") }
    }

    /// lin.dims (x: any) -> any
    /// Call method 'dims' on 'x'
    fn dims(obj: Object) -> Object { call!(obj.dims) }
    /// lin.comps (x: any) -> any
    /// Call method 'comps' on 'x'
    fn comps(obj: Object) -> Object { call!(obj.dims) }
    /// lin.mag (x: any) -> any
    /// Call method 'mag' on 'x'
    fn mag(obj: Object) -> Object { call!(obj.dims) }
    /// lin.mag2 (x: any) -> any
    /// Call method 'mag2' on 'x'
    fn mag2(obj: Object) -> Object { call!(obj.dims) }



    /// lin.M (rows: array of arrays) -> matrix
    /// Construct a matrix from a array of rows
    #[allow(non_snake_case)]
    #[global]
    fn M(rows: Vec<Object>) -> Object {
        let mut comps = Vec::new();
        for row in rows.into_iter() {
            match row.cast() {
                Err(err) => return err,
                Ok(arr) => comps.push(arr),
            }
        }
        Matrix::new(comps).into()
    }

    /// lin.zero (rows: natural) (cols: natural) -> matrix
    /// A 'rows' by 'cols' dimensional zero matrix
    pub fn zero(rows: usize, cols: usize) -> Result<Matrix, &'static str> {
        Matrix::zero(rows, cols).ok_or("Matrix dimensions can't be zero")
    }

    /// lin.ident (dims: natural) -> matrix
    /// Identity matrix with dimension 'dims'
    pub fn ident(dims: usize) -> Result<Matrix, &'static str> {
        Matrix::identity(dims).ok_or("Dimension must be a positive integer")
    }

    /// lin.rows (x: any) -> any
    /// Call method 'rows' on 'x'
    fn rows(obj: Object) -> Object { call!(obj.rows) }
    /// lin.cols (x: any) -> any
    /// Call method 'cols' on 'x'
    fn cols(obj: Object) -> Object { call!(obj.cols) }
    /// lin.row_vecs (x: any) -> any
    /// Call method 'row_vecs' on 'x'
    fn row_vecs(obj: Object) -> Object { call!(obj.row_vecs) }
    /// lin.col_vecs (x: any) -> any
    /// Call method 'col_vecs' on 'x'
    fn col_vecs(obj: Object) -> Object { call!(obj.col_vecs) }

    /// lin.trsp (m: matrix) -> matrix
    /// Transpose of 'm', where the rows and columns are swapped
    pub fn trsp(m: Matrix) -> Matrix {
        let mut m = m;
        m.transpose(); m
    }

    /// lin.inv (m: matrix) -> matrix
    /// Multiplicative inverse of the matrix 'm'
    fn inv(m: Matrix) -> Object { m.inverse().0 }
    /// lin.deter (m: matrix) -> any
    /// Determinant of the matrix 'm'
    fn deter(m: Matrix) -> Object { m.into_determinant() }
}
