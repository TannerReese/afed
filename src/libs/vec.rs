use std::mem::swap;
use std::vec::Vec;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error, Write};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::iter::zip;

use super::num::sqrt;
use super::mat::Matrix;

use crate::object::opers::{Unary, Binary};
use crate::object::{Operable, Object, NamedType, EvalError};
use crate::object::number::Number;
use crate::object::array::Array;
use crate::object::bltn_func::BltnFuncSingle;

macro_rules! check_dims {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims(), $b.dims());
        if adims != bdims { panic!(
            "Vector dimensions {} and {} do not match",
            adims, bdims,
        )}
    };
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vector(Vec<Object>);
impl NamedType for Vector { fn type_name() -> &'static str { "vector" }}

impl Operable for Vector {
    type Output = Object;
    fn unary(self, op: Unary) -> Option<Self::Output> { match op {
        Unary::Neg => Some((-self).into())
    }}
   
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add | Binary::Sub => other.is_a::<Vector>(),
        Binary::Mul => !other.is_a::<Matrix>(),
        Binary::Div | Binary::Mod | Binary::FlrDiv => {
            !rev && !other.is_a::<Matrix>()
        },
        _ => false,
    }}
    
    fn binary(self, rev: bool, op: Binary, other: Object) -> Self::Output {
        if other.is_a::<Vector>() {
            let (mut v1, mut v2) = (self, try_cast!(other => Vector));
            if v1.dims() != v2.dims() { return eval_err!(
                "Vector dimensions {} and {} do not match",
                v1.dims(), v2.dims(),
            )}
            if rev { swap(&mut v1, &mut v2); }
            
            match op {
                Binary::Add => (v1 + v2).into(),
                Binary::Sub => (v1 - v2).into(),
                Binary::Mul => v1 * v2,
                _ => panic!(),
            }
        } else if rev { match op {
            Binary::Mul => other * self,
            _ => panic!(),
        }.into()} else { match op {
            Binary::Mul => self * other,
            Binary::Div => self / other,
            Binary::Mod => self % other,
            Binary::FlrDiv => self.flrdiv(other),
            _ => panic!(),
        }.into()}
    }
    
    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        _ => None,
    }}

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Self::Output {
        if attr.is_some() { panic!() }
        if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
            if let Some(obj) = self.0.get(idx) { obj.clone() }
            else { eval_err!(
                "Index {} is larger or equal to dimension {}", idx, self.dims(),
            )}
        } else { eval_err!("Index could not be cast to correct integer") }
    }
}



impl Vector {
    pub fn check_errs(self) -> Result<Self, Object> {
        if self.0.iter().any(|c| c.is_err()) {
            Err(self.0.into_iter()
            .filter(|c| c.is_err())
            .next().unwrap())
        } else { Ok(self) }
    }
    
    pub fn dims(&self) -> usize { self.0.len() }
    
    pub fn mag2(self) -> Object
        { self.0.into_iter().map(|x| x.clone() * x).sum() }
    pub fn mag(self) -> Object {
        sqrt(try_cast!(self.mag2() => Number))
        .map_or(eval_err!(
            "Cannot take square root of negative"
        ), &Object::new)
    }
    
    
    pub fn flrdiv_assign(&mut self, rhs: Object)
        { self.0.iter_mut().for_each(|r| r.do_inside(|x| x.flrdiv(rhs.clone()))); }
    pub fn flrdiv(mut self, rhs: Object) -> Self { self.flrdiv_assign(rhs); self }
}


impl Neg for Vector {
    type Output = Self;
    fn neg(mut self) -> Self {
        self.0.iter_mut().for_each(|a| a.do_inside(|x| -x));
        self
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        check_dims!(self, rhs);
        zip(self.0.iter_mut(), rhs.0).for_each(|(a, b)| *a += b);
    }
}

impl Add for Vector {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self { self += rhs; self }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        check_dims!(self, rhs);
        zip(self.0.iter_mut(), rhs.0).for_each(|(a, b)| *a -= b);
    }
}

impl Sub for Vector {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self { self -= rhs; self }
}

impl Mul<Vector> for Vector {
    type Output = Object;
    fn mul(self, rhs: Vector) -> Object {
        check_dims!(self, rhs);
        zip(self.0, rhs.0).map(|(a, b)| a * b).sum()
    }
}

impl Mul<Vector> for Object {
    type Output = Vector;
    fn mul(self, mut rhs: Vector) -> Vector {
        rhs.0.iter_mut().for_each(|r| r.do_inside(|x| self.clone() * x));
        rhs
    }
}

impl MulAssign<Object> for Vector {
    fn mul_assign(&mut self, rhs: Object)
        { self.0.iter_mut().for_each(|x| *x *= rhs.clone()); }
}

impl Mul<Object> for Vector {
    type Output = Vector;
    fn mul(mut self, rhs: Object) -> Self { self *= rhs; self }
}

impl DivAssign<Object> for Vector {
    fn div_assign(&mut self, rhs: Object)
        { self.0.iter_mut().for_each(|x| *x /= rhs.clone()); }
}

impl Div<Object> for Vector {
    type Output = Vector;
    fn div(mut self, rhs: Object) -> Self { self /= rhs; self }
}

impl RemAssign<Object> for Vector {
    fn rem_assign(&mut self, rhs: Object)
        { self.0.iter_mut().for_each(|x| *x %= rhs.clone()); }
}

impl Rem<Object> for Vector {
    type Output = Vector;
    fn rem(mut self, rhs: Object) -> Self::Output { self %= rhs; self }
}


impl Display for Vector {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("V[")?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}

impl FromIterator<Object> for Vector {
    fn from_iter<T: IntoIterator<Item = Object>>(iter: T) -> Self
        { Vector(Vec::from_iter(iter)) }
}

impl From<Vector> for Object {
    fn from(v: Vector) -> Self {
        if v.0.iter().any(|x| x.is_err()) {
            v.0.into_iter()
            .filter(|x| x.is_err())
            .next().unwrap()
        } else { Object::new(v) }
    }
}



pub fn make_bltns() -> Object {
    let mut vec = HashMap::new();
    def_bltn!(vec.V(comps: Array) =
        if comps.0.len() > 0 { Vector(comps.0).into() } 
        else { eval_err!("Vector cannot be zero dimensional") }
    );
    def_bltn!(vec.dims(vec: Vector) = (vec.dims() as i64).into());
    def_bltn!(vec.mag2(vec: Vector) = vec.mag2());
    def_bltn!(vec.mag(vec: Vector) = vec.mag());
    vec.into()
}

