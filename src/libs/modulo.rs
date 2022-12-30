use std::mem::swap;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};

use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{
    Operable, Object,
    Unary, Binary,
    NamedType, EvalError,
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

    reducer = bezout(x, reducer).0;
    while reducer != 1 {
        while x % reducer == 0 { x /= reducer; }
        reducer = bezout(x, reducer).0;
    }
    return x
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modulo {
    residue: i64,
    modulo: u64,
}
name_type!{modulo: Modulo}

impl_operable!{Modulo:
    //! Residue class of a modular ring or an integer.
    //! Stored as a 64-bit signed residue and a 64-bit unsigned modulo.
    //! All operations convert integers to the appropriate modulo.
    //! When operators have arguments with different modulos,
    //! the GCD of the modulos is used.

    /// -modulo -> modulo
    /// Negation in modular ring
    #[unary(Neg)] fn _(m: Self) -> Self { -m }

    /// modulo + modulo -> modulo
    /// Add residue classes
    #[binary(Add)] fn _(m1: Self, m2: Self) -> Self { m1 + m2 }
    /// modulo - modulo -> modulo
    /// Subtract residue classes
    #[binary(Sub)] fn _(m1: Self, m2: Self) -> Self { m1 - m2 }
    /// modulo * modulo -> modulo
    /// Multiply residue classes
    #[binary(Mul)] fn _(m1: Self, m2: Self) -> Self { m1 * m2 }
    /// modulo / modulo -> modulo
    /// Divide residue classes by multiplying by the inverse
    #[binary(Div)] fn _(m1: Self, m2: Self) -> Self { m1 / m2 }
    /// modulo % modulo -> modulo
    /// Reduce residue class further to smaller modulo
    #[binary(Mod)] fn _(m1: Self, m2: Self) -> Self { m1 % m2 }
    /// modulo ^ integer -> modulo
    /// Raise residue class to an integer power
    #[binary(Pow)] fn _(m: Self, k: i64) -> Self { m.pow(k) }

    #[binary(comm, Add)]
    fn _(m: Self, n: Number) -> Result<Self, String> { m + n }
    #[binary(Sub)]
    fn _(m: Self, n: Number) -> Result<Self, String> { m - n }
    #[binary(Sub, rev)]
    fn _(m: Self, n: Number) -> Result<Self, String> { n - m }
    #[binary(comm, Mul)]
    fn _(m: Self, n: Number) -> Result<Self, String> { m * n }
    #[binary(Div)]
    fn _(m: Self, n: Number) -> Result<Self, String> { m / n }
    #[binary(Div, rev)]
    fn _(m: Self, n: Number) -> Result<Self, String> { n / m }
    #[binary(Mod)]
    fn _(m: Self, n: Number) -> Result<Self, String> { m % n }
    #[binary(Mod, rev)]
    fn _(m: Self, n: Number) -> Result<Self, String> { n % m }

    /// modulo.resid -> integer
    /// Smallest positive integer representation of residue class
    /// or integer itself if modulo is an integer
    pub fn resid(self) -> i64 { self.residue }
    /// modulo.modulo -> natural
    /// Modulo for modular ring or zero if 'modulo' is an integer
    pub fn modulo(self) -> u64 { self.modulo }

    /// modulo.has_inv -> bool
    /// True when residue class has a multiplicative inverse
    pub fn has_inv(self) -> bool
        { bezout(self.residue.abs() as u64, self.modulo).0 == 1 }
    /// modulo.inv -> modulo
    /// Multiplicative inverse of residue class
    pub fn inv(self) -> Self {
        if self.residue == 1 || self.residue == -1 { self }
        else if self.modulo > 0 {
            let (sign, res) = (self.residue.signum(), self.residue.abs());
            let new_mod = largest_coprime(self.modulo, res as u64);
            let (inv, _) = bezout(res as u64, new_mod).1;
            Modulo::from(sign * inv, new_mod)
        } else { Modulo::from(0, 1) }
    }

    /// modulo.order -> natural
    /// Smallest positive integer 'k' such that 'modulo ^ k == 1'
    /// or zero if 'modulo' is not invertible
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



impl Modulo {
    fn from(mut residue: i64, modulo: u64) -> Self {
        if modulo > 0 {
            residue %= modulo as i64;
            if residue < 0 { residue += modulo as i64; }
        }
        Modulo { residue, modulo }
    }

    fn from_number(&self, num: Number) -> Result<Self, String> { match num {
        Number::Ratio(n, d) => Ok(
            Modulo::from(n, self.modulo) /
            Modulo::from(d as i64, self.modulo)
        ),
        Number::Real(_) => Err(
            "Cannot convert real number to Modulo".to_owned()
        ),
    }}


    pub fn pow(mut self, rhs: i64) -> Self {
        let mut exp;
        if rhs == 0 { return self }
        else if rhs < 0 {
            self = self.inv();
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

impl Add<Number> for Modulo {
    type Output = Result<Modulo, String>;
    fn add(self, rhs: Number) -> Self::Output
        { self.from_number(rhs).map(|n| self + n) }
}

impl Sub for Modulo {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Modulo::from(
        self.residue - rhs.residue, bezout(self.modulo, rhs.modulo).0,
    )}
}

impl Sub<Number> for Modulo {
    type Output = Result<Modulo, String>;
    fn sub(self, rhs: Number) -> Self::Output
        { self.from_number(rhs).map(|n| self - n) }
}

impl Sub<Modulo> for Number {
    type Output = Result<Modulo, String>;
    fn sub(self, rhs: Modulo) -> Self::Output
        { rhs.from_number(self).map(|n| n - rhs) }
}

impl Mul for Modulo {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { Modulo::from(
        self.residue * rhs.residue, bezout(self.modulo, rhs.modulo).0,
    )}
}

impl Mul<Number> for Modulo {
    type Output = Result<Modulo, String>;
    fn mul(self, rhs: Number) -> Self::Output
        { self.from_number(rhs).map(|n| self * n) }
}

impl Div for Modulo {
    type Output = Self;
    fn div(self, rhs: Self) -> Self { self * rhs.inv() }
}

impl Div<Number> for Modulo {
    type Output = Result<Modulo, String>;
    fn div(self, rhs: Number) -> Self::Output
        { self.from_number(rhs).map(|n| self / n) }
}

impl Div<Modulo> for Number {
    type Output = Result<Modulo, String>;
    fn div(self, rhs: Modulo) -> Self::Output
        { rhs.from_number(self).map(|n| n / rhs) }
}

impl Rem for Modulo {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self { Modulo::from(
        self.residue, bezout(self.modulo,
            bezout(rhs.residue.abs() as u64, rhs.modulo).0
        ).0,
    )}
}

impl Rem<Number> for Modulo {
    type Output = Result<Modulo, String>;
    fn rem(self, rhs: Number) -> Self::Output
        { self.from_number(rhs).map(|n| self % n) }
}

impl Rem<Modulo> for Number {
    type Output = Result<Modulo, String>;
    fn rem(self, rhs: Modulo) -> Self::Output
        { rhs.from_number(self).map(|n| n % rhs) }
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

