use std::collections::HashMap;

use std::mem::swap;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem, Index};
use std::iter::{zip, Sum};

use crate::object::opers::{Unary, Binary};
use crate::object::{Operable, Object, Objectish, NamedType, EvalError};
use crate::object::number::Number;
use crate::object::array::Array;
use crate::object::bltn_func::BltnFuncSingle;

macro_rules! check_dims {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims, $b.dims);
        if adims != bdims { panic!(
            "Vector dimensions {} and {} do not match",
            adims, bdims,
        )}
    };
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vector<T = Object> {
    pub dims: usize,
    comps: Vec<T>,
}

impl<T> Vector<T> {
    pub fn new(comps: Vec<T>) -> Vector<T>
        { Vector {dims: comps.len(), comps}}
    
    pub fn mag2(self) -> T where T: Clone + Mul<Output=T> + Sum {
        self.comps.into_iter().map(|x| x.clone() * x).sum()
    }
}

impl<A> FromIterator<A> for Vector<A> {
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item = A>
        { Vector::new(Vec::from_iter(iter)) }
}


impl<A> Neg for Vector<A> where A: Neg {
    type Output = Vector<A::Output>;
    fn neg(self) -> Self::Output {
        self.comps.into_iter().map(|a| -a).collect()
    }
}

impl<A, B> Add<Vector<B>> for Vector<A> where A: Add<B> {
    type Output = Vector<A::Output>;
    fn add(self, rhs: Vector<B>) -> Self::Output {
        check_dims!(self, rhs);
        zip(self.comps, rhs.comps).map(|(a, b)| a + b).collect()
    }
}

impl<A, B> Sub<Vector<B>> for Vector<A> where A: Sub<B> {
    type Output = Vector<A::Output>;
    fn sub(self, rhs: Vector<B>) -> Self::Output {
        check_dims!(self, rhs);
        zip(self.comps, rhs.comps).map(|(a, b)| a - b).collect()
    }
}

impl<A> Mul<Vector<A>> for Vector<A> where A: Mul, A::Output: Sum {
    type Output = A::Output;
    fn mul(self, rhs: Vector<A>) -> Self::Output {
        check_dims!(self, rhs);
        zip(self.comps, rhs.comps).map(|(a, b)| a * b).sum()
    }
}

impl<A> Mul<A> for Vector<A> where A: Clone + Mul {
    type Output = Vector<A::Output>;
    fn mul(self, rhs: A) -> Self::Output
        { self.comps.into_iter().map(|a| a * rhs.clone()).collect() }
}

impl<A, B> Div<B> for Vector<A> where A: Div<B>, B: Clone {
    type Output = Vector<A::Output>;
    fn div(self, rhs: B) -> Self::Output
        { self.comps.into_iter().map(|a| a / rhs.clone()).collect() }
}

impl<A, B> Rem<B> for Vector<A> where A: Rem<B>, B: Clone {
    type Output = Vector<A::Output>;
    fn rem(self, rhs: B) -> Self::Output
        { self.comps.into_iter().map(|a| a % rhs.clone()).collect() }
}

impl<A> Index<usize> for Vector<A> {
    type Output = A;
    fn index(&self, idx: usize) -> &Self::Output { &self.comps[idx] }
}




impl<T: 'static> NamedType for Vector<T> { fn type_name() -> &'static str { "vector" }}
impl Objectish for Vector {}

impl Operable for Vector {
    type Output = Object;
    fn try_unary(&self, _: Unary) -> bool { true }
    fn unary(self, op: Unary) -> Self::Output { match op {
        Unary::Neg => (-self).into()
    }}
   
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add | Binary::Sub => other.is_a::<Vector>(),
        Binary::Mul => true,
        Binary::Div | Binary::Mod | Binary::FlrDiv => !rev,
        _ => false,
    }}
    
    fn binary(self, rev: bool, op: Binary, other: Object) -> Self::Output {
        if other.is_a::<Vector>() {
            let (mut v1, mut v2) = (self, try_cast!(other => Vector));
            if v1.dims != v2.dims { return eval_err!(
                "Vector dimensions {} and {} do not match",
                v1.dims, v2.dims
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
    
    fn arity(&self) -> usize { 1 }
    fn call<'a>(&self, mut args: Vec<Object>) -> Self::Output {
        if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
            if let Some(obj) = self.comps.get(idx) { obj.clone() }
            else { eval_err!("Index {} is larger than dimension", idx) }
        } else { eval_err!("Index could not be cast to correct integer") }
    }
}

impl<T> Display for Vector<T> where T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("V [")?;
        let mut is_first = true;
        for obj in self.comps.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}

impl Mul<Vector> for Object {
    type Output = Vector;
    fn mul(self, rhs: Self::Output) -> Self::Output
        { rhs.comps.into_iter().map(|b| self.clone() * b).collect() }
}

impl Vector {
    pub fn mag(self) -> Object {
        let m2 = try_cast!(self.mag2() => Number).to_real();
        m2.sqrt().into()
    }
    
    pub fn flrdiv(self, other: Object) -> Self
        { self.comps.into_iter().map(|x| x.flrdiv(other.clone())).collect() }
}


pub fn make_bltns() -> Object {
    let mut vec = HashMap::new();
    def_bltn!(vec.V(comps: Array) =
        if comps.0.len() > 0 { Vector::new(comps.0).into() }
        else { eval_err!("Vector cannot be zero dimensional") }
    );
    def_bltn!(vec.dims(vec: Vector) = (vec.dims as i64).into());
    def_bltn!(vec.mag(vec: Vector) = vec.mag());
    vec.into()
}

