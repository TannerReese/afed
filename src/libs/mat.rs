use std::vec::IntoIter;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Error, Write};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::ops::{Index, IndexMut};
use std::iter::zip;

use super::vec::Vector;
use super::augmat::AugMatrix;
use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{
    Operable, Object, Castable,
    Unary, Binary,
    NamedType, EvalError,
};
use crate::object::array::Array;

macro_rules! check_dims {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims, $b.dims);
        if adims != bdims { panic!(
            "Matrix dimensions {:?} and {:?} do not match",
            adims, bdims,
        )}
    };
}

pub struct Matrix {
    dims: (usize, usize),
    pub comps: Vec<Object>,
    deter: Cell<Option<Object>>,
}
name_type!{matrix: Matrix}

pub struct IntoVectors {
    dims: usize,
    comps: IntoIter<Object>,
}


impl Operable for Matrix {
    def_unary!{self, -self = -self}
    def_binary!{self,
        self + m : (Matrix) = { guard!(self + m, self.dims == m.dims,
            "Matrix dimensions {:?} and {:?} do not match", self.dims, m.dims
        )},
        self - m : (Matrix) = { guard!(self - m, self.dims == m.dims,
            "Matrix dimensions {:?} and {:?} do not match", self.dims, m.dims
        )},

        self * m : (Matrix) = { guard!(self * m, self.dims.1 == m.dims.0,
            "For matrix multiplication, {} and {} do not match",
            self.dims.1, m.dims.0,
        )},
        self * v : (Vector) = { guard!(self * v, self.dims.1 == v.dims(),
            "Vector dimension {} does not match column dimension {} in matrix",
            v.dims(), self.dims.1,
        )},
        v * self : (Vector) = { guard!(v * self, self.dims.0 == v.dims(),
            "Vector dimension {} does not match row dimension {} in matrix",
            v.dims(), self.dims.0,
        )},
        self * other = { self * other },
        other * self = { other * self },

        self / _v : (Vector) = {},
        self / m : (Matrix) = { Object::new(self) * m.inverse().0 },
        self / other = { self / other },

        self % _v : (Vector) = {},
        self % _m : (Matrix) = {},
        self % other = { self % other },

        self "//" _v : (Vector) = {},
        self "//" _m : (Matrix) = {},
        self "//" other = { self.flrdiv(other) }
    }

    def_methods!{mat,
       __call(idx: usize) = if idx >= mat.rows() { eval_err!(
            "Index {} is larger or equal to {} number of rows",
            idx, mat.rows()
        )} else {
            let cols = mat.columns();
            mat.comps[idx * cols .. (idx + 1) * cols]
            .iter().cloned().collect::<Vector>().into()
        },

        rows() = mat.rows().into(),
        cols() = mat.columns().into(),
        row_vecs() = mat.clone().into_rows().collect(),
        col_vecs() = mat.clone().into_columns().collect(),
        trsp() = {
            let mut m = mat.clone();
            m.transpose();
            m.into()
        },

        has_inv() = call!((mat.determinant()).has_inv()),
        inv() = {
            let (inv, det) = mat.clone().inverse();
            if let Some(det) = det { mat.deter.set(Some(det)); }
            inv
        },
        deter() = mat.determinant()
    }
}



impl Matrix {
    pub fn new(rows: Vec<Vec<Object>>) -> Object {
        let row_dim = rows.len();
        if row_dim == 0 {
            return eval_err!("Matrix cannot be zero-dimensional");
        }

        let col_dim = rows[0].len();
        if rows.iter().any(|r| r.len() != col_dim) {
            return eval_err!("Matrix cannot have jagged rows");
        }

        let comps = rows.into_iter().flatten().collect();
        Matrix {
            dims: (row_dim, col_dim), comps,
            deter: Cell::new(None),
        }.into()
    }

    pub fn from_array(arr: Array) -> Object {
        let mut comps = Vec::new();
        for row in arr.0.into_iter() {
            comps.push(cast!(row))
        }
        Matrix::new(comps)
    }

    pub fn build<F>((rows, cols): (usize, usize), mut gen: F) -> Self
    where F: FnMut(usize, usize) -> Object {
        let mut comps = Vec::new();
        for i in 0..rows { for j in 0..cols {
            comps.push(gen(i, j))
        }}
        Matrix {dims: (rows, cols), comps, deter: Cell::new(None)}
    }

    pub fn identity(dims: usize) -> Self {
        let ident = Self::build((dims, dims), |r, c|
            if r == c { 1 } else { 0 }.into()
        );
        ident.deter.set(Some(1.into()));
        ident
    }

    pub fn rows(&self) -> usize { self.dims.0 }
    pub fn columns(&self) -> usize { self.dims.1 }

    pub fn transpose(&mut self){
        let (rows, cols) = self.dims;
        let comps = &mut self.comps;
        let prev = |idx: usize| {
            let (j, i) = (idx / rows, idx % rows);
            j + i * cols
        };

        let mut visited = Vec::with_capacity(rows * cols);
        visited.resize(rows * cols, false);
        for i in 0..rows { for j in 0..cols {
            let start = j + i * cols;
            if visited[start] { continue }
            visited[start] = true;

            let mut loc = start;
            loop {
                let prev_loc = prev(loc);
                if prev_loc == start { break }
                comps.swap(loc, prev_loc);
                loc = prev_loc;
                visited[loc] = true;
            }
        }}
        self.dims = (self.dims.1, self.dims.0);
    }

    pub fn into_rows(self) -> IntoVectors {
        IntoVectors {dims: self.columns(), comps: self.comps.into_iter()}
    }

    pub fn into_columns(mut self) -> IntoVectors {
        self.transpose();
        self.into_rows()
    }


    pub fn flrdiv_assign(&mut self, rhs: Object)
        { self.comps.iter_mut().for_each(|r| r.do_inside(|x| x.flrdiv(rhs.clone()))); }
    pub fn flrdiv(mut self, rhs: Object) -> Self { self.flrdiv_assign(rhs); self }


    pub fn inverse(self) -> (Object, Option<Object>) {
        if self.dims.0 != self.dims.1 {
            return (eval_err!(concat!(
                "Rows dimension {} and column dimension {} don't match.",
                " Cannot take inverse"
            ), self.dims.0, self.dims.1), None);
        }

        let rows = self.rows();
        let ident = Self::identity(rows);
        let mut augmat = AugMatrix::new(vec![self, ident]);
        if let Err(err) = augmat.gauss_elim(0) { return (err, None); }

        if augmat.matrices[0] == Self::identity(rows) {
            let inv = augmat.matrices.remove(1);
            let det = call!((augmat.deter).inv());
            inv.deter.set(Some(augmat.deter));
            (inv.into(), Some(det))
        } else { (eval_err!("Matrix is singular"), None) }
    }

    pub fn determinant(&self) -> Object {
        let det = if let Some(det) = self.deter.take() { det }
        else { self.clone().determinant() };
        self.deter.set(Some(det.clone()));
        det
    }

    pub fn into_determinant(self) -> Object {
        if let Some(deter) = self.deter.take() { return deter }

        if self.dims.0 != self.dims.1 {
            return eval_err!(concat!(
                "Rows dimension {} and column dimension {} don't match.",
                " Cannot take inverse"
            ), self.dims.0, self.dims.1);
        }

        let rows = self.rows();
        let mut augmat = AugMatrix::new(vec![self]);
        if let Err(err) = augmat.gauss_elim(0) { return err; }

        if augmat.matrices[0] == Self::identity(rows) {
            call!((augmat.deter).inv())
        } else { 0.into() }
    }
}

impl Debug for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Matrix {{ dims: {:?}, comps: {:?} }}", self.dims, self.comps)
    }
}

impl Clone for Matrix {
    fn clone(&self) -> Self {
        let old_det = self.deter.take();
        let deter = Cell::new(old_det.clone());
        self.deter.set(old_det);
        Matrix { dims: self.dims, comps: self.comps.clone(), deter }
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
    fn index(&self, (r, c): (usize, usize)) -> &Object
        { let cols = self.columns(); &self.comps[c + r * cols] }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut Object
        { let cols = self.columns(); &mut self.comps[c + r * cols] }
}

impl Iterator for IntoVectors {
    type Item = Vector;
    fn next(&mut self) -> Option<Self::Item> {
        if self.comps.as_slice().len() > 0 {
            Some(self.comps.by_ref().take(self.dims).collect())
        } else { None }
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
    fn add(mut self, rhs: Self) -> Self { self += rhs; self }
}

impl SubAssign for Matrix {
    fn sub_assign(&mut self, rhs: Self) {
        check_dims!(self, rhs);
        zip(self.comps.iter_mut(), rhs.comps).for_each(|(a, b)| *a -= b);
    }
}

impl Sub for Matrix {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self { self -= rhs; self }
}



impl MulAssign<Object> for Matrix {
    fn mul_assign(&mut self, rhs: Object)
        { self.comps.iter_mut().for_each(|a| *a *= rhs.clone()); }
}

impl Mul<Object> for Matrix {
    type Output = Matrix;
    fn mul(mut self, rhs: Object) -> Matrix { self *= rhs; self }
}

impl Mul<Matrix> for Object {
    type Output = Matrix;
    fn mul(self, mut rhs: Matrix) -> Matrix {
        rhs.comps.iter_mut().for_each(|a| a.do_inside(|x| self.clone() * x));
        rhs
    }
}

impl DivAssign<Object> for Matrix {
    fn div_assign(&mut self, rhs: Object)
        { self.comps.iter_mut().for_each(|a| *a /= rhs.clone()); }
}

impl Div<Object> for Matrix {
    type Output = Matrix;
    fn div(mut self, rhs: Object) -> Matrix { self /= rhs; self }
}

impl RemAssign<Object> for Matrix {
    fn rem_assign(&mut self, rhs: Object)
        { self.comps.iter_mut().for_each(|a| *a %= rhs.clone()) }
}

impl Rem<Object> for Matrix {
    type Output = Matrix;
    fn rem(mut self, rhs: Object) -> Matrix { self %= rhs; self }
}



impl Mul<Vector> for Matrix {
    type Output = Vector;
    fn mul(self, rhs: Vector) -> Vector
        { self.into_rows().map(|row| row * rhs.clone()).collect() }
}

impl Mul<Matrix> for Vector {
    type Output = Vector;
    fn mul(self, rhs: Matrix) -> Vector
        { rhs.into_columns().map(|col| self.clone() * col).collect() }
}

impl Mul<Matrix> for Matrix {
    type Output = Matrix;
    fn mul(self, rhs: Matrix) -> Self::Output {
        if self.columns() != rhs.rows() { panic!(
            "For matrix multiplication, {} and {} do not match",
            self.columns(), rhs.rows(),
        )}
        Matrix::build((self.rows(), rhs.columns()), |i, j|
            (0..self.columns()).map(|k|
                self[(i, k)].clone() * rhs[(k, j)].clone()
            ).sum()
        )
    }
}


impl Display for Matrix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let (rows, cols) = self.dims;
        
        f.write_str("M[")?;
        let mut is_first = true;
        for i in 0..rows {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            
            let mut is_first_inner = true;
            f.write_char('[')?;
            for j in 0..cols {
                if !is_first_inner { f.write_str(", ")?; }
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
            m.comps.into_iter()
            .filter(|x| x.is_err())
            .next().unwrap()
        } else { Object::new(m) }
    }
}




pub fn make_bltns() -> Bltn {
    let mut mat = HashMap::new();
    def_bltn!(static mat.M(rows: Array) = Matrix::from_array(rows).into());
    def_bltn!(mat.zero(rows: usize, cols: usize) = if rows > 0 && cols > 0 {
        Matrix::build((rows, cols), |_, _| 0.into()).into()
    } else { eval_err!("Matrix dimensions can't be zero") });
    def_bltn!(mat.ident(dims: usize) =
        if dims == 0 { eval_err!("Dimension must be a positive integer") }
        else { Matrix::identity(dims).into() }
    );

    def_getter!(mat.rows);
    def_getter!(mat.cols);
    def_getter!(mat.row_vecs);
    def_getter!(mat.col_vecs);

    def_bltn!(mat.trsp(m: Matrix) = {
        let mut m = m;
        m.transpose(); m.into()
    });
    def_bltn!(mat.inv(m: Matrix) = m.inverse().0);
    def_bltn!(mat.deter(m: Matrix) = m.into_determinant());
    Bltn::Map(mat)
}

