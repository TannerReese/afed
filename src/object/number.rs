use std::any::Any;
use std::vec::Vec;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::RemAssign;

use super::opers::{Unary, Binary};
use super::bltn_func::{BltnFuncSingle, BltnFuncDouble};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Ratio(i64, u64),
    Real(f64),
}
impl NamedType for Number { fn type_name() -> &'static str { "number" } }
impl Objectish for Number { impl_objectish!{} }

fn gcd<T>(a: T, b: T) -> T where T: Eq + Copy + Ord + Default + RemAssign {
    let (mut a, mut b) = if a > b { (b, a) } else { (a, b) };
    let zero = T::default();
    
    while a != zero {
        b %= a;
        std::mem::swap(&mut a, &mut b);
    }
    return b;
}

impl Number {
    pub fn real(r: f64) -> Object { Object::new(Number::Real(r)) }
    
    pub fn simplify(&self) -> Self { match self {
        &Number::Ratio(n, d) => {
            let g = gcd(n.abs() as u64, d);
            Number::Ratio(n / g as i64, d / g)
        },
        &num => num,
    }}
    
    pub fn to_real(&self) -> f64 { match self {
        &Number::Ratio(n, d) => n as f64 / d as f64,
        &Number::Real(r) => r,
    }}
    
    pub fn as_index(&self) -> Option<usize> { match self {
        &Number::Ratio(n, 1) => usize::try_from(n).ok(),
        _ => None,
    }}
    
    pub fn pow(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, 1)) => {
            let (n1, d1, n2) = if n2 < 0 {
                if n1 < 0 { (-(d1 as i64), -n1 as u64, -n2 as u32) }
                else { (d1 as i64, n1 as u64, -n2 as u32) }
            } else { (n1, d1, n2 as u32) };
            Number::Ratio(n1.pow(n2), d1.pow(n2))
        },
        (num1, num2) => Number::Real(num1.to_real().powf(num2.to_real())),
    }}
    
    pub fn sqrt(self) -> Option<Self> {
        let r = self.to_real();
        if r < 0.0 { None }
        else { Some(Number::Real(r.sqrt())) }
    }
    
    pub fn gcd(a: Self, b: Self) -> Option<Self> { match (a, b) {
        (Number::Ratio(na, da), Number::Ratio(nb, db)) => Some({
            Number::Ratio(gcd(na * db as i64, nb * da as i64), da * db)
        }.simplify()),
        _ => None
    }}
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&Number::Ratio(n1, d1), &Number::Ratio(n2, d2)) => {
                n1 * d2 as i64 == n2 * d1 as i64
            },
            (num1, num2) => {
                (num1.to_real() - num2.to_real()).abs() < 1e-10
            },
        }
    }
}

impl Eq for Number {}

impl Neg for Number {
    type Output = Self;
    fn neg(self) -> Self { match self {
        Number::Ratio(n, d) => Number::Ratio(-n, d),
        Number::Real(r) => Number::Real(-r),
    }}
}

impl Add for Number {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => Number::Ratio(
            n1 * d2 as i64 + n2 * d1 as i64, d1 * d2
        ).simplify(),
        (num1, num2) => Number::Real(num1.to_real() + num2.to_real()),
    }}
}

impl Sub for Number {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => Number::Ratio(
            n1 * d2 as i64 - n2 * d1 as i64, d1 * d2
        ).simplify(),
        (num1, num2) => Number::Real(num1.to_real() - num2.to_real()),
    }}
}


impl Mul for Number {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => Number::Ratio(
            n1 * n2, d1 * d2
        ).simplify(),
        (num1, num2) => Number::Real(num1.to_real() * num2.to_real()),
    }}
}

impl Div for Number {
    type Output = Self;
    fn div(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
            let (n2, d2) = if n2 < 0 { (-(d2 as i64), -n2 as u64) }
                else { (d2 as i64, n2 as u64) };
            Number::Ratio(n1 * n2, d1 * d2).simplify()
        },
        (num1, num2) => Number::Real(num1.to_real() / num2.to_real()),
    }}
}

impl Rem for Number {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
            let mut rem = (n1 * d2 as i64) % (n2 * d1 as i64);
            if rem < 0 { rem += n2 * d1 as i64; }
            Number::Ratio(rem, d1 * d2).simplify()
        },
        (num1, num2) => Number::Real(num1.to_real().rem_euclid(num2.to_real())),
    }}
}



impl Operable<Object> for Number {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Ok(Object::new(match op {
            Unary::Neg => -*self,
        }))
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        let num1 = *self;
        let num2 = other.downcast::<Number>()?;
        
        Ok(Object::new(match op {
            Binary::Add => num1 + num2,
            Binary::Sub => num1 - num2,
            Binary::Mul => num1 * num2,
            Binary::Div => num1 / num2,
            Binary::Mod => num1 % num2,
            Binary::Pow => num1.pow(num2),
        }))
    }
    
    call_not_impl!{Self}
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            Number::Ratio(n, 1) => write!(f, "{}", n),
            Number::Ratio(n, d) => write!(f, "{} / {}", n, d),
            Number::Real(r) => write!(f, "{}", r),
        }
    }
}



pub fn make_bltns() -> HashMap<String, Object> {
    HashMap::from_iter([
        ("pi", Number::real(std::f64::consts::PI)),
        ("e", Number::real(std::f64::consts::E)),
        ("sqrt", BltnFuncSingle::new("num.sqrt", |n: Number|
            n.sqrt().ok_or(eval_err!("Cannot take square root of negative"))
        )),
        ("gcd", BltnFuncDouble::new("num.gcd", |a: Number, b: Number|
            Number::gcd(a, b).ok_or(eval_err!("Cannot take GCD of reals"))
        )),
    ].into_iter().map(|(key, val)| (key.to_owned(), val)))
}

