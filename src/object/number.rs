use std::mem::swap;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::cmp::Ordering;

use super::{Operable, Object, Unary, Binary, NamedType, EvalError};
use super::bool::Bool;

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Ratio(i64, u64),
    Real(f64),
}
impl NamedType for Number { fn type_name() -> &'static str { "number" } }

impl Operable for Number {
    fn unary(self, op: Unary) -> Option<Object> { match op {
        Unary::Neg => Some((-self).into()),
        _ => None,
    }}

    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Leq |
        Binary::Add | Binary::Sub |
        Binary::Mul | Binary::Div | Binary::Mod | Binary::FlrDiv |
        Binary::Pow => other.is_a::<Number>(),
        _ => false,
    }}

    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        let (mut num1, mut num2) = (self, try_cast!(other));
        if rev { swap(&mut num1, &mut num2); }

        match op {
            Binary::Leq => return Bool::new(num1 <= num2),
            Binary::Add => num1 + num2,
            Binary::Sub => num1 - num2,
            Binary::Mul => num1 * num2,
            Binary::Div => num1 / num2,
            Binary::Mod => num1 % num2,
            Binary::FlrDiv => num1.flrdiv(num2),
            Binary::Pow => num1.pow(num2),
            _ => panic!(),
        }.into()
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        Some("has_inv") | Some("inv") => Some(0),
        Some("abs") | Some("signum") => Some(0),
        Some("real") => Some(0),
        Some("floor") | Some("ceil") | Some("round") => Some(0),

        Some("sqrt") | Some("cbrt") => Some(0),

        Some("sin")   | Some("cos")   | Some("tan")   => Some(0),
        Some("asin")  | Some("acos")  | Some("atan")  => Some(0),
        Some("sinh")  | Some("cosh")  | Some("tanh")  => Some(0),
        Some("asinh") | Some("acosh") | Some("atanh") => Some(0),
        Some("atan2") => Some(1),

        Some("exp") | Some("exp2") => Some(0),
        Some("ln") | Some("log10") | Some("log2") => Some(0),
        Some("log") => Some(1),

        Some("gcd") => Some(1),
        Some("lcm") => Some(1),
        Some("factorial") => Some(0),
        Some("choose") => Some(1),
        _ => None,
    }}

    fn call<'a>(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Object { match attr {
        Some("has_inv") => (*self != Number::Ratio(0, 1)).into(),
        Some("inv") => (Number::Ratio(1, 1) / *self).into(),

        Some("abs") => self.abs().into(),
        Some("signum") => self.signum().into(),
        Some("real") => self.to_real().into(),
        Some("floor") => self.floor().into(),
        Some("ceil") => self.ceil().into(),
        Some("round") => self.round().into(),

        Some("sqrt") => {
            let r = self.to_real();
            if r < 0.0 { eval_err!("Cannot take square root of negative") }
            else { r.sqrt().into() }
        },
        Some("cbrt") => self.to_real().cbrt().into(),

        Some("sin") => self.to_real().sin().into(),
        Some("cos") => self.to_real().cos().into(),
        Some("tan") => self.to_real().tan().into(),
        Some("asin") => self.to_real().asin().into(),
        Some("acos") => self.to_real().acos().into(),
        Some("atan") => self.to_real().atan().into(),
        Some("atan2") => {
            let other = try_cast!(args.remove(0) => Number);
            self.to_real().atan2(other.to_real()).into()
        },

        Some("sinh") => self.to_real().sinh().into(),
        Some("cosh") => self.to_real().cosh().into(),
        Some("tanh") => self.to_real().tanh().into(),
        Some("asinh") => self.to_real().asinh().into(),
        Some("acosh") => self.to_real().acosh().into(),
        Some("atanh") => self.to_real().atanh().into(),

        Some("exp") => self.to_real().exp().into(),
        Some("exp2") => self.to_real().exp2().into(),
        Some("ln") => self.to_real().ln().into(),
        Some("log10") => self.to_real().log10().into(),
        Some("log2") => self.to_real().log2().into(),
        Some("log") => {
            let other = try_cast!(args.remove(0) => Number);
            other.to_real().log(self.to_real()).into()
        },

        Some("gcd") => {
            let other = try_cast!(args.remove(0) => Number);
            Number::gcd(*self, other).map_or(
                eval_err!("Cannot take GCD of reals"),
                &Object::new,
            )
        },
        Some("lcm") => {
            let other = try_cast!(args.remove(0) => Number);
            Number::lcm(*self, other).map_or(
                eval_err!("Cannot take LCM of reals"),
                &Object::new,
            )
        },
        Some("factorial") => self.factorial().map_or(
            eval_err!("Can only take factorial of positive integer"),
            &Object::new,
        ),
        Some("choose") => {
            let other = try_cast!(args.remove(0) => Number);
            Number::choose(*self, other).map_or(
                eval_err!("Second argument to choose must be a positive integer"),
                &Object::new,
            )
        },

        _ => panic!(),
    }}
}


pub fn gcd<T>(a: T, b: T) -> T where T: Eq + Copy + Ord + Default + RemAssign {
    let (mut a, mut b) = if a > b { (b, a) } else { (a, b) };
    let zero = T::default();

    while a != zero {
        b %= a;
        swap(&mut a, &mut b);
    }
    return b;
}

impl Number {
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
        (num1, num2) => num1.to_real().powf(num2.to_real()).into(),
    }}

    pub fn flrdiv(self, rhs: Self) -> Self { match (self, rhs) {
        (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
            if (n1 < 0) == (n2 < 0) || n1 == 0 || n2 == 0 {
                let n = (n1.abs() as u64 * d2) / (n2.abs() as u64 * d1);
                (n as i64).into()
            } else {
                let n = (n1.abs() as u64 * d2 - 1) / (n2.abs() as u64 * d1) + 1;
                (-(n as i64)).into()
            }
        },
        (num1, num2) => num1.to_real().div_euclid(num2.to_real()).into(),
    }}


    pub fn abs(self) -> Self { match self {
        Number::Ratio(n, d) => Number::Ratio(n.abs(), d),
        Number::Real(r) => Number::Real(r.abs()),
    }}

    pub fn signum(self) -> Self { Number::Ratio(match self {
        Number::Ratio(n, _) => n.signum(),
        Number::Real(r) => r.signum() as i64
    }, 1)}

    pub fn floor(self) -> Self { match self {
        Number::Ratio(n, d) => Number::Ratio(if n < 0 {
            (n + 1) / d as i64 - 1
        } else {
            n / d as i64
        }, 1),
        Number::Real(r) => Number::Ratio(r.floor() as i64, 1),
    }}

    pub fn ceil(self) -> Self { -(-self).floor() }
    pub fn round(self) -> Self { (self + Number::Ratio(1, 2)).floor() }
}



impl Number {
    pub fn gcd(a: Self, b: Self) -> Option<Self> { match (a, b) {
        (Number::Ratio(na, da), Number::Ratio(nb, db)) => Some({
            let g = gcd(na.abs() as u64 * db, nb.abs() as u64 * da);
            Number::Ratio(g as i64, da * db)
        }.simplify()),
        _ => None
    }}

    pub fn lcm(a: Self, b: Self) -> Option<Self> { match (a, b) {
        (Number::Ratio(na, da), Number::Ratio(nb, db)) => Some({
            let g = gcd(na.abs() as u64 * db, nb.abs() as u64 * da);
            Number::Ratio(na * nb, g)
        }.simplify()),
        _ => None,
    }}

    pub fn factorial(self) -> Option<Self> {
        Some(((1..self.as_index()? + 1).product::<usize>() as i64).into())
    }

    pub fn choose(n: Number, k: Number) -> Option<Self> {
        let one = Number::Ratio(1, 1);
        Some((0..k.as_index()?).map(|i| (i as i64).into())
        .fold(one, |accum, i|
            accum * (n - i) / (i + one)
        ))
    }
}



impl From<i64> for Number {
    fn from(n: i64) -> Self { Number::Ratio(n, 1) }
}

impl From<i32> for Number {
    fn from(n: i32) -> Self { Number::Ratio(n as i64, 1) }
}

impl From<usize> for Number {
    fn from(n: usize) -> Self { Number::Ratio(n as i64, 1) }
}

impl From<f64> for Number {
    fn from(r: f64) -> Self { Number::Real(r) }
}

impl From<Number> for Object {
    fn from(n: Number) -> Self { Object::new(n) }
}

impl From<i64> for Object {
    fn from(n: i64) -> Self { Number::Ratio(n, 1).into() }
}

impl From<i32> for Object {
    fn from(n: i32) -> Self { Number::Ratio(n as i64, 1).into() }
}

impl From<usize> for Object {
    fn from(n: usize) -> Self { Number::Ratio(n as i64, 1).into() }
}

impl From<f64> for Object {
    fn from(r: f64) -> Self { Number::Real(r).into() }
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

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { match (self, other) {
        (&Number::Ratio(n1, d1), &Number::Ratio(n2, d2)) => {
            Some((n1 * d2 as i64).cmp(&(n2 * d1 as i64)))
        },
        (num1, num2) => num1.to_real().partial_cmp(&num2.to_real()),
    }}
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        PartialOrd::partial_cmp(self, other).unwrap()
    }
}

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

impl AddAssign for Number {
    fn add_assign(&mut self, rhs: Self) { *self = *self + rhs }
}

impl SubAssign for Number {
    fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs }
}

impl MulAssign for Number {
    fn mul_assign(&mut self, rhs: Self) { *self = *self * rhs }
}

impl DivAssign for Number {
    fn div_assign(&mut self, rhs: Self) { *self = *self / rhs }
}

impl RemAssign for Number {
    fn rem_assign(&mut self, rhs: Self) { *self = *self % rhs }
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

