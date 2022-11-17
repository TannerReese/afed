use std::mem::swap;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};

use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{
    Operable, Object,
    Unary, Binary,
    NamedType, ErrObject, EvalError,
};
use crate::object::number::Number;

fn bezout(a: u64, b: u64) -> (u64, (i64, i64)) {
    let (mut r, mut s) = ((a, 1, 0), (b, 0, 1));
    if r.0 < s.0 { swap(&mut r, &mut s); }

    while s.0 > 0 {
        let div = r.0 / s.0;
        r = (r.0 - div * s.0,
            r.1 - (div as i64) * s.1, r.2 - (div as i64) * s.2
        );
        swap(&mut r, &mut s);
    }
    return (r.0, (r.1, r.2))
}

fn largest_coprime(mut x: u64, mut reducer: u64) -> u64 {
    if reducer == 0 { return 1 }

    while reducer != 1 {
        let div = x / reducer;
        let res = x - div * reducer;
        if res == 0 { x = div; }
        else { reducer = res; }
    }
    return x
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modulo {
    residue: i64,
    modulo: u64,
}
name_type!{modulo: Modulo}

impl Operable for Modulo {
    fn unary(self, op: Unary) -> Option<Object> { match op {
        Unary::Neg => Some((-self).into()),
        _ => None,
    }}

    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add | Binary::Sub |
        Binary::Mul | Binary::Div | Binary::Mod => {
            other.is_a::<Modulo>() || other.is_a::<Number>()
        },
        Binary::Pow => !rev && other.is_a::<Number>(),
        _ => false,
    }}

    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        if op == Binary::Pow { return self.pow(cast!(other)).into() }

        let mut m1 = self;
        let mut m2 = match_cast!(other,
            m: Modulo => m,
            num: Number => match num {
                Number::Ratio(n, d) => Modulo::from_ratio((n, d), self.modulo),
                Number::Real(_) => return eval_err!(
                    "Can't convert real number to modular"
                ),
            }
        ).unwrap();
        if rev { swap(&mut m1, &mut m2); }

        match op {
            Binary::Add => m1 + m2,
            Binary::Sub => m1 - m2,
            Binary::Mul => m1 * m2,
            Binary::Div => m1 / m2,
            Binary::Mod => m1 % m2,
            _ => panic!(),
        }.into()
    }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        Some("resid") | Some("modulo") => Some(0),
        Some("has_inv") | Some("inv") => Some(0),
        Some("order") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, _: Vec<Object>
    ) -> Object { match attr {
        Some("resid") => self.residue.into(),
        Some("modulo") => (self.modulo as i64).into(),

        Some("has_inv") => (bezout(
            self.residue.abs() as u64, self.modulo
        ).0 == 1).into(),
        Some("inv") => self.inverse().into(),

        Some("order") => self.order().into(),
        _ => panic!(),
    }}
}

impl Modulo {
    fn from(mut residue: i64, modulo: u64) -> Self {
        if modulo > 0 {
            residue %= modulo as i64;
            if residue < 0 { residue += modulo as i64; }
        }
        Modulo { residue, modulo }
    }

    fn from_ratio((numer, denom): (i64, u64), modulo: u64) -> Self {
        Modulo::from(numer, modulo) / Modulo::from(denom as i64, modulo)
    }

    pub fn inverse(self) -> Self {
        if self.residue == 1 || self.residue == -1 { self }
        else if self.modulo > 0 {
            let (sign, res) = (self.residue.signum(), self.residue.abs());
            let new_mod = largest_coprime(self.modulo, res as u64);
            let (inv, _) = bezout(res as u64, new_mod).1;
            Modulo::from(sign * inv, new_mod)
        } else { Modulo::from(0, 1) }
    }

    pub fn pow(mut self, rhs: i64) -> Self {
        let mut exp;
        if rhs == 0 { return self }
        else if rhs < 0 {
            self = self.inverse();
            exp = (-rhs) as u64;
        } else { exp = rhs as u64; }

        let (mut accum, mut power) = (Modulo::from(1, 0), self);
        while exp > 0 {
            if exp & 1 == 1 { accum = accum * power; }
            power = power * power;
            exp >>= 1;
        }
        accum
    }

    pub fn order(self) -> u64 {
        if bezout(self.residue.abs() as u64, self.modulo).0 > 1 {
            return 0;
        }

        use super::prs::{prime_factors, euler_totient};
        let max_order = euler_totient(self.modulo);
        let mut ord = max_order;
        for (p, _) in prime_factors(max_order) {
            while ord % p == 0
            && self.pow((ord / p) as i64).residue == 1 {
                ord /= p;
            }
        }
        ord
    }
}

impl Neg for Modulo {
    type Output = Self;
    fn neg(self) -> Self { Modulo::from(-self.residue, self.modulo) }
}

impl Add for Modulo {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Modulo::from(
        self.residue + rhs.residue, bezout(self.modulo, rhs.modulo).0,
    )}
}

impl Sub for Modulo {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Modulo::from(
        self.residue - rhs.residue, bezout(self.modulo, rhs.modulo).0,
    )}
}

impl Mul for Modulo {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { Modulo::from(
        self.residue * rhs.residue, bezout(self.modulo, rhs.modulo).0,
    )}
}

impl Div for Modulo {
    type Output = Self;
    fn div(self, rhs: Self) -> Self { self * rhs.inverse() }
}

impl Rem for Modulo {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self { Modulo::from(
        self.residue, bezout(self.modulo,
            bezout(rhs.residue.abs() as u64, rhs.modulo).0
        ).0,
    )}
}


impl Display for Modulo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{} (mod {})", self.residue, self.modulo)
    }
}

impl From<Modulo> for Object {
    fn from(m: Modulo) -> Self { Object::new(m) }
}



pub fn make_bltns() -> Bltn {
    let mut modulo = HashMap::new();
    def_bltn!(static modulo("mod").Mod(m: Number) = match m {
        Number::Ratio(0, 1) => eval_err!("Modulo can't be zero"),
        Number::Ratio(m, 1) => Modulo::from(1, m.abs() as u64).into(),
        _ => eval_err!("Modulo must be a non-zero integer"),
    });
    Bltn::Map(modulo)
}

