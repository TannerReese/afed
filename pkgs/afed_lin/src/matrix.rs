// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::cell::Cell;
use std::convert::TryInto;
use std::fmt::{Debug, Display, Error, Formatter, Write};
use std::iter::zip;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};
use std::ops::{Index, IndexMut};
use std::vec::IntoIter;

use super::aug_matrix::AugMatrix;
use super::vector::Vector;

use afed_objects::{call, eval_err, impl_operable, name_type, Object};

macro_rules! check_dims {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims, $b.dims);
        if adims != bdims {
            panic!("Matrix dimensions {:?} and {:?} do not match", adims, bdims,)
        }
    };
}

pub struct Matrix {
    dims: (usize, usize),
    pub comps: Vec<Object>,
    deter: Cell<Option<Object>>,
}
name_type! {matrix: Matrix}

pub struct IntoVectors {
    dims: usize,
    comps: IntoIter<Object>,
}

impl_operable! {Matrix:
    //! Matrix with arbitrary size and heterogeneous components

    /// -matrix -> matrix
    /// Negation of matrix
    #[unary(Neg)] fn _(m: Self) -> Self { -m }

    /// matrix + matrix -> matrix
    /// Add matrices with the same number of rows and columns
    #[binary(Add)] fn _(m1: Self, m2: Self) -> Result<Self, String> {
        if m1.dims == m2.dims { Ok(m1 + m2) } else { Err(format!(
            "Matrix dimensions {:?} and {:?} do not match", m1.dims, m2.dims
        ))}
    }

    /// matrix - matrix -> matrix
    /// Subtract matrices with the same number of rows and columns
    #[binary(Sub)] fn _(m1: Self, m2: Self) -> Result<Self, String> {
        if m1.dims == m2.dims { Ok(m1 - m2) } else { Err(format!(
            "Matrix dimensions {:?} and {:?} do not match", m1.dims, m2.dims
        ))}
    }

    /// (a: matrix) * (b: matrix) -> matrix
    /// Multiply matrices as long as 'a.cols == b.rows'
    #[binary(Mul)] fn _(m1: Self, m2: Self) -> Result<Self, String> {
        if m1.dims.1 == m2.dims.0 { Ok(m1 * m2) } else { Err(format!(
            "For matrix multiplication, {} and {} do not match",
            m1.dims.1, m2.dims.0,
        ))}
    }

    /// matrix * vector -> vector
    /// vector * matrix -> vector
    /// Apply 'matrix' to a row or column vector of appropriate dimension
    #[binary(Mul)] fn _(m: Self, v: Vector) -> Result<Vector, String> {
        if m.dims.1 == v.dims() { Ok(m * v) } else { Err(format!(
            "Vector dimension {} does not match column dimension {} in matrix",
            v.dims(), m.dims.1,
        ))}
    }

    #[binary(Mul, rev)] fn _(m: Self, v: Vector) -> Result<Vector, String> {
        if m.dims.0 == v.dims() { Ok(v * m) } else { Err(format!(
            "Vector dimension {} does not match row dimension {} in matrix",
            v.dims(), m.dims.0,
        ))}
    }

    /// (scalar: any) * matrix -> matrix
    /// matrix * (scalar: any) -> matrix
    /// Multiply each component of 'matrix' by 'vector'
    #[binary(Mul)] fn _(m: Self, scalar: Object) -> Self { m * scalar }
    #[binary(Mul, rev)] fn _(m: Self, scalar: Object) -> Self { scalar * m }

    /// matrix / matrix -> matrix
    /// Divide matrices by multiplying by the inverse
    #[binary(Div)]
    #[exclude(Vector)]
    fn _(m1: Self, m2: Matrix) -> Object { Object::new(m1) * m2.inverse().0 }
    /// matrix / (scalar: any) -> matrix
    /// Divide each component of 'matrix' by 'scalar'
    #[binary(Div)]
    fn _(m: Self, scalar: Object) -> Self { m / scalar }

    /// matrix % (mod: any) -> matrix
    /// Reduce each component of 'matrix' modulo 'mod'
    #[binary(Mod)]
    #[exclude(Vector, Matrix)]
    fn _(m: Self, scalar: Object) -> Self { m % scalar }

    /// matrix // (divisor: any) -> matrix
    /// Floor divide each component of 'matrix' by 'divisor'
    #[binary(FlrDiv)]
    #[exclude(Vector, Matrix)]
    fn _(m: Self, scalar: Object) -> Self { m.flrdiv(scalar) }


    /// matrix (i: natural) -> vector
    /// Get the 'i'th row vector of 'matrix'
    #[call]
    fn __call(&self, idx: usize) -> Result<Vector, String> {
        if idx >= self.rows() { Err(format!(
            "Index {} is larger or equal to {} number of rows",
            idx, self.rows()
        ))} else {
            let cols = self.cols();
            Ok(self.comps[idx * cols .. (idx + 1) * cols]
                .iter().cloned().collect())
        }
    }

    /// matrix.rows -> natural
    /// Number of rows in 'matrix'
    pub fn rows(&self) -> usize { self.dims.0 }
    /// matrix.cols -> natural
    /// Number of columns in 'matrix'
    pub fn cols(&self) -> usize { self.dims.1 }
    /// matrix.row_vecs -> array of vectors
    /// Array of row vectors in 'matrix'
    pub fn row_vecs(self) -> Vec<Vector> { self.into_rows().collect() }
    /// matrix.col_vecs -> array of vectors
    /// Array of column vectors in 'matrix'
    pub fn col_vecs(self) -> Vec<Vector> { self.into_columns().collect() }

    /// matrix.trsp -> matrix
    /// Transpose of 'matrix' where the rows are swapped for columns
    pub fn trsp(self) -> Self {
        let mut m = self;
        m.transpose();  m
    }

    /// matrix.has_inv -> bool
    /// True if 'matrix' has a multiplicative inverse
    pub fn has_inv(&self) -> bool {
        call!((self.deter()).has_inv())
        .try_cast().unwrap_or(false)
    }

    /// matrix.inv -> matrix
    /// Multiplicative inverse of 'matrix'
    pub fn inv(&self) -> Object {
        let (inv, det) = self.clone().inverse();
        if let Some(det) = det { self.deter.set(Some(det)); }
        inv
    }

    /// matrix.deter -> any
    /// Determinant of 'matrix'
    pub fn deter(&self) -> Object {
        let det = if let Some(det) = self.deter.take() { det }
        else { self.clone().into_determinant() };
        self.deter.set(Some(det.clone()));
        det
    }
}

impl Matrix {
    pub fn new(rows: Vec<Vec<Object>>) -> Result<Matrix, &'static str> {
        let row_dim = rows.len();
        if row_dim == 0 {
            return Err("Matrix cannot be zero-dimensional");
        }

        let col_dim = rows[0].len();
        if rows.iter().any(|r| r.len() != col_dim) {
            return Err("Matrix cannot have jagged rows");
        }

        let comps = rows.into_iter().flatten().collect();
        Ok(Matrix {
            dims: (row_dim, col_dim),
            comps,
            deter: Cell::new(None),
        })
    }

    pub fn build<F, T>((rows, cols): (usize, usize), mut gen: F) -> Self
    where
        T: Into<Object>,
        F: FnMut(usize, usize) -> T,
    {
        let mut comps = Vec::new();
        for i in 0..rows {
            for j in 0..cols {
                comps.push(gen(i, j).into())
            }
        }
        Matrix {
            dims: (rows, cols),
            comps,
            deter: Cell::new(None),
        }
    }

    pub fn zero(rows: usize, cols: usize) -> Option<Matrix> {
        if rows == 0 || cols == 0 {
            return None;
        }
        let z = Matrix::build((rows, cols), |_, _| 0);
        z.deter.set(Some(0.into()));
        Some(z)
    }

    pub fn identity(dims: usize) -> Option<Self> {
        if dims == 0 {
            return None;
        }
        let id = Matrix::build((dims, dims), |r, c| if r == c { 1 } else { 0 });
        id.deter.set(Some(1.into()));
        Some(id)
    }

    pub fn transpose(&mut self) {
        let (rows, cols) = self.dims;
        let comps = &mut self.comps;
        let prev = |idx: usize| {
            let (j, i) = (idx / rows, idx % rows);
            j + i * cols
        };

        let mut visited = Vec::with_capacity(rows * cols);
        visited.resize(rows * cols, false);
        for i in 0..rows {
            for j in 0..cols {
                let start = j + i * cols;
                if visited[start] {
                    continue;
                }
                visited[start] = true;

                let mut loc = start;
                loop {
                    let prev_loc = prev(loc);
                    if prev_loc == start {
                        break;
                    }
                    comps.swap(loc, prev_loc);
                    loc = prev_loc;
                    visited[loc] = true;
                }
            }
        }
        self.dims = (self.dims.1, self.dims.0);
    }

    pub fn into_rows(self) -> IntoVectors {
        IntoVectors {
            dims: self.cols(),
            comps: self.comps.into_iter(),
        }
    }

    pub fn into_columns(mut self) -> IntoVectors {
        self.transpose();
        self.into_rows()
    }

    pub fn flrdiv_assign(&mut self, rhs: Object) {
        self.comps
            .iter_mut()
            .for_each(|r| r.do_inside(|x| x.flrdiv(rhs.clone())));
    }
    pub fn flrdiv(mut self, rhs: Object) -> Self {
        self.flrdiv_assign(rhs);
        self
    }

    pub fn inverse(self) -> (Object, Option<Object>) {
        if self.dims.0 != self.dims.1 {
            return (
                eval_err!(
                    concat!(
                        "Rows dimension {} and column dimension {} don't match.",
                        " Cannot take inverse"
                    ),
                    self.dims.0,
                    self.dims.1
                ),
                None,
            );
        }

        let rows = self.rows();
        let id = Matrix::identity(rows).unwrap();
        let mut augmat = AugMatrix::new(vec![self, id.clone()]);
        if let Err(err) = augmat.gauss_elim(0) {
            return (err, None);
        }

        if augmat.matrices[0] == id {
            let inv = augmat.matrices.remove(1);
            let det = call!((augmat.deter).inv());
            inv.deter.set(Some(augmat.deter));
            (inv.into(), Some(det))
        } else {
            (eval_err!("Matrix is singular"), None)
        }
    }

    pub fn into_determinant(self) -> Object {
        if let Some(deter) = self.deter.take() {
            return deter;
        }

        if self.dims.0 != self.dims.1 {
            return eval_err!(
                concat!(
                    "Rows dimension {} and column dimension {} don't match.",
                    " Cannot take inverse"
                ),
                self.dims.0,
                self.dims.1
            );
        }

        let rows = self.rows();
        let mut augmat = AugMatrix::new(vec![self]);
        if let Err(err) = augmat.gauss_elim(0) {
            return err;
        }

        if augmat.matrices[0] == Matrix::identity(rows).unwrap() {
            call!((augmat.deter).inv())
        } else {
            0.into()
        }
    }
}

impl Debug for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Matrix {{ dims: {:?}, comps: {:?} }}",
            self.dims, self.comps
        )
    }
}

impl Clone for Matrix {
    fn clone(&self) -> Self {
        let old_det = self.deter.take();
        let deter = Cell::new(old_det.clone());
        self.deter.set(old_det);
        Matrix {
            dims: self.dims,
            comps: self.comps.clone(),
            deter,
        }
    }
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        self.dims == other.dims && self.comps == other.comps
    }
}

impl Eq for Matrix {}

impl Index<(usize, usize)> for Matrix {
    type Output = Object;
    fn index(&self, (r, c): (usize, usize)) -> &Object {
        let cols = self.cols();
        &self.comps[c + r * cols]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut Object {
        let cols = self.cols();
        &mut self.comps[c + r * cols]
    }
}

impl Iterator for IntoVectors {
    type Item = Vector;
    fn next(&mut self) -> Option<Self::Item> {
        if !self.comps.as_slice().is_empty() {
            Some(self.comps.by_ref().take(self.dims).collect())
        } else {
            None
        }
    }
}

impl Neg for Matrix {
    type Output = Self;
    fn neg(mut self) -> Self {
        self.comps.iter_mut().for_each(|a| a.do_inside(|x| -x));
        self
    }
}

impl AddAssign for Matrix {
    fn add_assign(&mut self, rhs: Self) {
        check_dims!(self, rhs);
        zip(self.comps.iter_mut(), rhs.comps).for_each(|(a, b)| *a += b);
    }
}

impl Add for Matrix {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl SubAssign for Matrix {
    fn sub_assign(&mut self, rhs: Self) {
        check_dims!(self, rhs);
        zip(self.comps.iter_mut(), rhs.comps).for_each(|(a, b)| *a -= b);
    }
}

impl Sub for Matrix {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl MulAssign<Object> for Matrix {
    fn mul_assign(&mut self, rhs: Object) {
        self.comps.iter_mut().for_each(|a| *a *= rhs.clone());
    }
}

impl Mul<Object> for Matrix {
    type Output = Matrix;
    fn mul(mut self, rhs: Object) -> Matrix {
        self *= rhs;
        self
    }
}

impl Mul<Matrix> for Object {
    type Output = Matrix;
    fn mul(self, mut rhs: Matrix) -> Matrix {
        rhs.comps
            .iter_mut()
            .for_each(|a| a.do_inside(|x| self.clone() * x));
        rhs
    }
}

impl DivAssign<Object> for Matrix {
    fn div_assign(&mut self, rhs: Object) {
        self.comps.iter_mut().for_each(|a| *a /= rhs.clone());
    }
}

impl Div<Object> for Matrix {
    type Output = Matrix;
    fn div(mut self, rhs: Object) -> Matrix {
        self /= rhs;
        self
    }
}

impl RemAssign<Object> for Matrix {
    fn rem_assign(&mut self, rhs: Object) {
        self.comps.iter_mut().for_each(|a| *a %= rhs.clone())
    }
}

impl Rem<Object> for Matrix {
    type Output = Matrix;
    fn rem(mut self, rhs: Object) -> Matrix {
        self %= rhs;
        self
    }
}

impl Mul<Vector> for Matrix {
    type Output = Vector;
    fn mul(self, rhs: Vector) -> Vector {
        self.into_rows().map(|row| row * rhs.clone()).collect()
    }
}

impl Mul<Matrix> for Vector {
    type Output = Vector;
    fn mul(self, rhs: Matrix) -> Vector {
        rhs.into_columns().map(|col| self.clone() * col).collect()
    }
}

impl Mul<Matrix> for Matrix {
    type Output = Matrix;
    fn mul(self, rhs: Matrix) -> Self::Output {
        if self.cols() != rhs.rows() {
            panic!(
                "For matrix multiplication, {} and {} do not match",
                self.cols(),
                rhs.rows(),
            )
        }
        Matrix::build((self.rows(), rhs.cols()), |i, j| {
            (0..self.cols())
                .map(|k| self[(i, k)].clone() * rhs[(k, j)].clone())
                .sum::<Object>()
        })
    }
}

impl Display for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let (rows, cols) = self.dims;

        f.write_str("M[")?;
        let mut is_first = true;
        for i in 0..rows {
            if !is_first {
                f.write_str(", ")?;
            }
            is_first = false;

            let mut is_first_inner = true;
            f.write_char('[')?;
            for j in 0..cols {
                if !is_first_inner {
                    f.write_str(", ")?;
                }
                is_first_inner = false;
                write!(f, "{}", self[(i, j)])?;
            }
            f.write_char(']')?;
        }
        f.write_char(']')
    }
}

impl From<Matrix> for Object {
    fn from(m: Matrix) -> Self {
        if m.comps.iter().any(|x| x.is_err()) {
            m.comps.into_iter().find(|x| x.is_err()).unwrap()
        } else {
            Object::new(m)
        }
    }
}
