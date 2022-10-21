use std::mem::swap;
use std::vec::{Vec, IntoIter};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error, Write};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::ops::{Index, IndexMut};
use std::iter::zip;

use super::vec::Vector;

use crate::object::opers::{Unary, Binary};
use crate::object::{Operable, Object, NamedType, EvalError};
use crate::object::number::Number;
use crate::object::array::Array;
use crate::object::bltn_func::BltnFuncSingle;

macro_rules! check_dims {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims, $b.dims);
        if adims != bdims { panic!(
            "Matrix dimensions {:?} and {:?} do not match",
            adims, bdims,
        )}
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    dims: (usize, usize),
    comps: Vec<Object>,
}
impl NamedType for Matrix { fn type_name() -> &'static str { "matrix" }}

pub struct IntoVectors {
    dims: usize,
    comps: IntoIter<Object>,
}


impl Operable for Matrix {
    type Output = Object;
    fn unary(self, op: Unary) -> Option<Self::Output> { match op {
        Unary::Neg => Some((-self).into()),
        _ => None,
    }}
    
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add | Binary::Sub => other.is_a::<Matrix>(),
        Binary::Mul => true,
        Binary::Div | Binary::Mod | Binary::FlrDiv => {
            !rev && !other.is_a::<Vector>() && !other.is_a::<Matrix>()
        },
        _ => false,
    }}
    
    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        if other.is_a::<Matrix>() {
            let (mut m1, mut m2) = (self, try_cast!(other => Matrix));
            if rev { swap(&mut m1, &mut m2); }
            
            match op {
                Binary::Add => if m1.dims == m2.dims {
                    (m1 + m2).into()
                } else { eval_err!(
                    "Matrix dimensions {:?} and {:?} do not match",
                    m1.dims, m2.dims,
                )},
                Binary::Sub => if m1.dims == m2.dims {
                    (m1 - m2).into()
                } else { eval_err!(
                    "Matrix dimensions {:?} and {:?} do not match",
                    m1.dims, m2.dims,
                )},
                Binary::Mul => if m1.columns() == m2.rows() {
                    (m1 * m2).into()
                } else { eval_err!(
                    "For matrix multiplication, {} and {} do not match",
                    m1.columns(), m2.rows(),
                )},
                _ => panic!(),
            }
        } else if other.is_a::<Vector>() {
            let (m, v) = (self, try_cast!(other => Vector));
            match op {
                Binary::Mul => if rev {
                    if v.dims() == m.rows() { (v * m).into() }
                    else { eval_err!(
                        "Vector dimension {} does not match row dimension {} in matrix",
                        v.dims(), m.rows(),
                    )}
                } else {
                    if m.columns() == v.dims() { (m * v).into() }
                    else { eval_err!(
                        "Vector dimension {} does not match column dimension {} in matrix",
                        v.dims(), m.columns(),
                    )}
                },
                _ => panic!(),
            }
        } else if rev { match op {
            Binary::Mul => (other * self).into(),
            _ => panic!(),
        }} else { match op {
            Binary::Mul => (self * other).into(),
            Binary::Div => (self / other).into(),
            Binary::Mod => (self % other).into(),
            Binary::FlrDiv => self.flrdiv(other).into(),
            _ => panic!(),
        }}
    }
    
    
    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some("rows") => Some(0),
        Some("cols") => Some(0),
        Some("row_vecs") => Some(0),
        Some("col_vecs") => Some(0),
        Some("trsp") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Self::Output { match attr {
        None => {
            if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
                if idx >= self.rows() { eval_err!(
                    "Index {} is larger or equal to {} number of rows",
                    idx, self.rows()
                )} else {
                    let cols = self.columns();
                    self.comps[idx * cols .. (idx + 1) * cols]
                    .iter().cloned().collect::<Vector>().into()
                }
            } else { eval_err!("Index could not be cast to correct integer") }
        },

        Some("rows") => self.rows().into(),
        Some("cols") => self.columns().into(),
        Some("row_vecs") =>
            self.clone().into_rows().map(|v| v.into()).collect(),
        Some("col_vecs") =>
            self.clone().into_columns().map(|v| v.into()).collect(),
        Some("trsp") => {
            let mut m = self.clone();
            m.transpose();
            m.into()
        },
        _ => panic!(),
    }}
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
        Matrix {dims: (row_dim, col_dim), comps}.into()
    }
    
    pub fn from_array(arr: Array) -> Object {
        let mut comps = Vec::new();
        for row in arr.0.into_iter() {
            comps.push(try_cast!(row => Array).0)
        }
        Matrix::new(comps)
    }
    
    pub fn build<F>((rows, cols): (usize, usize), mut gen: F) -> Self
    where F: FnMut(usize, usize) -> Object {
        let mut comps = Vec::new();
        for i in 0..rows { for j in 0..cols {
            comps.push(gen(i, j))
        }}
        Matrix {dims: (rows, cols), comps}
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
}

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




pub fn make_bltns() -> Object {
    let mut mat = HashMap::new();
    def_bltn!(mat.M(rows: Array) = Matrix::from_array(rows).into());
    mat.into()
}

