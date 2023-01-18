use std::cmp::Ordering;
use std::fmt::{Display, Error, Formatter};
use std::mem::swap;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};
use std::vec::Vec;

use super::{Binary, Castable, ErrObject, EvalError, NamedType, Object, Operable, Unary};

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Ratio(i64, u64),
    Real(f64),
}
name_type! {number: Number}

impl_operable! {Number:
    //! Real or rational number. A real is stored in 64-bit floating point.
    //! A rational is a 64-bit signed numerator with a 64-bit unsigned denominator.
    //! All operations convert rationals to reals when operating with reals.

    /// -number -> number
    /// Negation of 'number'
    #[unary(Neg)] fn _(num: Number) -> Number { -num }

    /// number <= number -> bool
    /// Implements standard ordering of reals
    #[binary(Leq)] fn _(n1: Self, n2: Self) -> bool { n1 <= n2 }
    /// number + number -> number
    /// Add numbers
    #[binary(Add)] fn _(n1: Self, n2: Self) -> Self { n1 + n2 }
    /// number - number -> number
    /// Subtract numbers
    #[binary(Sub)] fn _(n1: Self, n2: Self) -> Self { n1 - n2 }
    /// number * number -> number
    /// Multiply numbers
    #[binary(Mul)] fn _(n1: Self, n2: Self) -> Self { n1 * n2 }
    /// number / number -> number
    /// Divide numbers
    #[binary(Div)] fn _(n1: Self, n2: Self) -> Self { n1 / n2 }
    /// number % number -> number
    /// Get remainder after dividing
    #[binary(Mod)] fn _(n1: Self, n2: Self) -> Self { n1 % n2 }
    /// number // number -> number
    /// Get greatest integer less than or equal to the quotient
    #[binary(FlrDiv)] fn _(n1: Self, n2: Self) -> Self { n1.flrdiv(n2) }
    #[binary(Pow)] fn _(n1: Self, n2: Self) -> Self { n1.pow(n2) }

    /// rational.numer -> number
    /// Numerator of 'rational'
    pub fn numer(self) -> Result<i64, &'static str> { match self {
        Number::Ratio(n, _) => Ok(n),
        Number::Real(_) => Err("Real number has no numerator"),
    }}

    /// rational.denom -> number
    /// Denominator of 'rational'
    pub fn denom(self) -> Result<u64, &'static str> { match self {
        Number::Ratio(_, d) => Ok(d),
        Number::Real(_) => Err("Real number has no denominator"),
    }}

    /// integer.digits (b: natural) -> array of integers
    /// Digits of an integer in base 'b'
    pub fn digits(self, base: u64) -> Result<Vec<u64>, &'static str> {
        let mut num: u64 = self.abs().try_into()
            .map_err(|_| "Digits of a non-integer are ambiguous")?;
        let mut digs = Vec::new();
        while num > 0 {
            digs.push(num % base);
            num /= base;
        }
        Ok(digs)
    }

    /// number.has_inv -> bool
    /// True when 'number' is not zero
    pub fn has_inv(self) -> bool { self != Number::Ratio(0, 1) }
    /// number.inv -> number
    /// Multiplicative inverse of 'number'
    pub fn inv(self) -> Self { Number::Ratio(1, 1) / self }
    /// number.str -> string
    /// Convert 'number' to string
    pub fn str(self) -> String { format!("{}", self) }

    /// number.abs -> number
    /// Absolute value of 'number'
    pub fn abs(self) -> Self { match self {
        Number::Ratio(n, d) => Number::Ratio(n.abs(), d),
        Number::Real(r) => Number::Real(r.abs()),
    }}

    /// number.signum -> number
    /// 1 if 'number' is positive, -1 if negative, and zero when zero
    pub fn signum(self) -> i8 { match self {
        Number::Ratio(n, _) => n.signum() as i8,
        Number::Real(r) => r.signum() as i8
    }}

    /// number.real -> real
    /// Convert 'number' to a real number
    pub fn real(self) -> f64 { f64::from(self) }

    /// number.floor -> integer
    /// Greatest integer less than or equal to 'number'
    pub fn floor(self) -> i64 { match self {
        Number::Ratio(n, d) => if n < 0 {
            (n + 1) / d as i64 - 1
        } else {
            n / d as i64
        },
        Number::Real(r) => r.floor() as i64,
    }}

    /// number.ceil -> integer
    /// Least integer greater than or equal to 'number'
    pub fn ceil(self) -> i64 { -(-self).floor() }
    /// number.round -> integer
    /// Closest ingeger to 'number'
    pub fn round(self) -> i64 { (self + Number::Ratio(1, 2)).floor() }

    /// number.sqrt -> real
    /// Square root of number
    pub fn sqrt(self) -> Result<f64, &'static str> {
        let r = self.real();
        if r < 0.0 { Err("Cannot take square root of negative") }
        else { Ok(r.sqrt()) }
    }

    /// number.cbrt -> real
    /// Cube root of 'number'
    pub fn cbrt(self) -> f64 { self.real().cbrt() }
    /// number.sin -> real
    /// Sine of 'number' in radians
    pub fn sin(self) -> f64 { self.real().sin() }
    /// number.cos -> real
    /// Cosine of 'number' in radians
    pub fn cos(self) -> f64 { self.real().cos() }
    /// number.tan -> real
    /// Tangent of 'number' in radians
    pub fn tan(self) -> f64 { self.real().tan() }
    /// number.asin -> real
    /// Inverse sine of 'number' in radians
    pub fn asin(self) -> f64 { self.real().asin() }
    /// number.acos -> real
    /// Inverse cosine of 'number' in radians
    pub fn acos(self) -> f64 { self.real().acos() }
    /// number.atan -> real
    /// Inverse tangent of 'number' in radians
    pub fn atan(self) -> f64 { self.real().atan() }
    /// y.atan2 (x: number) -> real
    /// Angle of point ('x', 'y') counter-clockwise from x-axis
    pub fn atan2(self, x: f64) -> f64
        { self.real().atan2(x) }

    /// number.sinh -> real
    /// Hyperbolic sine of 'number'
    pub fn sinh(self) -> f64 { self.real().sinh() }
    /// number.cosh -> real
    /// Hyperbolic cosine of 'number'
    pub fn cosh(self) -> f64 { self.real().cosh() }
    /// number.tanh -> real
    /// Hyperbolic tangent of 'number'
    pub fn tanh(self) -> f64 { self.real().tanh() }
    /// number.asinh -> real
    /// Inverse hyperbolic sine of 'number'
    pub fn asinh(self) -> f64 { self.real().asinh() }
    /// number.acosh -> real
    /// Inverse hyperbolic cosine of 'number'
    pub fn acosh(self) -> f64 { self.real().acosh() }
    /// number.atanh -> real
    /// Inverse hyperbolic tangent of 'number'
    pub fn atanh(self) -> f64 { self.real().atanh() }

    /// number.exp -> real
    /// Return e to the power of the 'number'
    pub fn exp(self) -> f64 { self.real().exp() }
    /// number.exp2 -> real
    /// Return 2 to the power of the 'number'
    pub fn exp2(self) -> f64 { self.real().exp2() }
    /// number.ln -> real
    /// Natural logarithm of 'number'
    pub fn ln(self) -> f64 { self.real().ln() }
    /// number.log10 -> real
    /// Decimal logarithm of 'number'
    pub fn log10(self) -> f64 { self.real().log10() }
    /// number.log2 -> real
    /// Logarithm of 'number' with base two
    pub fn log2(self) -> f64 { self.real().log2() }
    /// b.log (x: number) -> number
    /// Logarithm of 'x' with base 'b'
    pub fn log(self, other: f64) -> f64 { other.log(self.real()) }


    /// a.gcd (b: number) -> rational
    /// Greatest commmon divisor of 'a' and 'b'
    pub fn gcd(self, other: Self) -> Result<Self, &'static str> {
        match (self, other) {
            (Number::Ratio(na, da), Number::Ratio(nb, db)) => Ok({
                let g = gcd(na.unsigned_abs() as u64 * db, nb.unsigned_abs() as u64 * da);
                Number::Ratio(g as i64, da * db)
            }.simplify()),
            _ => Err("Cannot take GCD of reals"),
        }
    }

    /// a.lcm (b: number) -> rational
    /// Least common multiple of 'a' and 'b'
    pub fn lcm(self, other: Self) -> Result<Self, &'static str> {
        match (self, other) {
            (Number::Ratio(na, da), Number::Ratio(nb, db)) => Ok({
                let g = gcd(na.unsigned_abs() as u64 * db, nb.unsigned_abs() as u64 * da);
                Number::Ratio(na * nb, g)
            }.simplify()),
            _ => Err("Cannot take LCM of reals"),
        }
    }

    /// natural.factorial -> natural
    /// Product of all positive integers less than 'natural'
    pub fn factorial(self) -> Result<Self, &'static str> {
        match (self).try_into() {
            Ok(n) => Ok((1..=n).product::<u64>().into()),
            Err(_) => Err("Cannot take factorial of non-integer"),
        }
    }

    /// number.choose (k: natural) -> number
    /// Falling factorial with length 'k' of 'number' divided by 'k!'
    pub fn choose(self, k: usize) -> Self {
        let one = Number::Ratio(1, 1);
        (0..k).map(|i| (i as i64).into())
        .fold(one, |accum, i: Number|
            accum * (self - i) / (i + one)
        )
    }
}

pub fn gcd<T>(a: T, b: T) -> T
where
    T: Eq + Copy + Ord + Default + RemAssign,
{
    let (mut a, mut b) = if a > b { (b, a) } else { (a, b) };
    let zero = T::default();

    while a != zero {
        b %= a;
        swap(&mut a, &mut b);
    }
    b
}

impl Number {
    pub fn simplify(&self) -> Self {
        match self {
            &Number::Ratio(n, d) => {
                let g = gcd(n.unsigned_abs() as u64, d);
                Number::Ratio(n / g as i64, d / g)
            }
            num => *num,
        }
    }

    pub fn pow(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, 1)) => {
                let (n1, d1, n2) = if n2 < 0 {
                    if n1 < 0 {
                        (-(d1 as i64), -n1 as u64, -n2 as u32)
                    } else {
                        (d1 as i64, n1 as u64, -n2 as u32)
                    }
                } else {
                    (n1, d1, n2 as u32)
                };
                Number::Ratio(n1.pow(n2), d1.pow(n2))
            }
            (num1, num2) => num1.real().powf(num2.real()).into(),
        }
    }

    pub fn flrdiv(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                if (n1 < 0) == (n2 < 0) || n1 == 0 || n2 == 0 {
                    let n = (n1.unsigned_abs() as u64 * d2) / (n2.unsigned_abs() as u64 * d1);
                    (n as i64).into()
                } else {
                    let n =
                        (n1.unsigned_abs() as u64 * d2 - 1) / (n2.unsigned_abs() as u64 * d1) + 1;
                    (-(n as i64)).into()
                }
            }
            (num1, num2) => num1.real().div_euclid(num2.real()).into(),
        }
    }
}

// Create conversion trait implementations between integral types and `Object`
macro_rules! convert_integral {
    ($tp:ty) => {
        impl From<$tp> for Number {
            fn from(n: $tp) -> Self {
                Number::Ratio(n as i64, 1)
            }
        }

        impl From<$tp> for Object {
            fn from(n: $tp) -> Self {
                Number::from(n).into()
            }
        }

        impl TryFrom<Number> for $tp {
            type Error = Number;
            fn try_from(num: Number) -> Result<$tp, Self::Error> {
                if let Number::Ratio(n, 1) = num {
                    if let Ok(n) = n.try_into() {
                        return Ok(n);
                    }
                }
                Err(num)
            }
        }

        impl Castable for $tp {
            fn cast(obj: Object) -> Result<$tp, (Object, ErrObject)> {
                match Number::cast(obj)?.try_into() {
                    Ok(val) => Ok(val),
                    Err(num) => Err((
                        Object::new(num),
                        eval_err!("Cannot cast number to integer type"),
                    )),
                }
            }
        }
    };
}

convert_integral! {i8}
convert_integral! {u8}
convert_integral! {i16}
convert_integral! {u16}
convert_integral! {i32}
convert_integral! {u32}
convert_integral! {i64}
convert_integral! {u64}
convert_integral! {usize}

impl From<f64> for Number {
    fn from(r: f64) -> Self {
        Number::Real(r)
    }
}

impl From<f64> for Object {
    fn from(r: f64) -> Self {
        Number::Real(r).into()
    }
}

impl From<Number> for f64 {
    fn from(num: Number) -> f64 {
        match num {
            Number::Ratio(n, d) => (n as f64) / (d as f64),
            Number::Real(r) => r,
        }
    }
}

impl Castable for f64 {
    fn cast(obj: Object) -> Result<f64, (Object, ErrObject)> {
        Ok(Number::cast(obj)?.into())
    }
}

impl From<Number> for Object {
    fn from(n: Number) -> Self {
        Object::new(n)
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&Number::Ratio(n1, d1), &Number::Ratio(n2, d2)) => n1 * d2 as i64 == n2 * d1 as i64,
            (num1, num2) => {
                let (r1, r2) = (num1.real(), num2.real());
                if r1.is_infinite() && r2.is_infinite() {
                    true
                } else {
                    (r1 - r2).abs() < 1e-10
                }
            }
        }
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (&Number::Ratio(n1, d1), &Number::Ratio(n2, d2)) => {
                Some((n1 * d2 as i64).cmp(&(n2 * d1 as i64)))
            }
            (num1, num2) => num1.real().partial_cmp(&num2.real()),
        }
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        PartialOrd::partial_cmp(self, other).unwrap()
    }
}

impl Neg for Number {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Number::Ratio(n, d) => Number::Ratio(-n, d),
            Number::Real(r) => Number::Real(-r),
        }
    }
}

impl Add for Number {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                Number::Ratio(n1 * d2 as i64 + n2 * d1 as i64, d1 * d2).simplify()
            }
            (num1, num2) => (f64::from(num1) + f64::from(num2)).into(),
        }
    }
}

impl Sub for Number {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                Number::Ratio(n1 * d2 as i64 - n2 * d1 as i64, d1 * d2).simplify()
            }
            (num1, num2) => (f64::from(num1) - f64::from(num2)).into(),
        }
    }
}

impl Mul for Number {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                Number::Ratio(n1 * n2, d1 * d2).simplify()
            }
            (num1, num2) => (f64::from(num1) * f64::from(num2)).into(),
        }
    }
}

impl Div for Number {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                let (n2, d2) = if n2 < 0 {
                    (-(d2 as i64), -n2 as u64)
                } else {
                    (d2 as i64, n2 as u64)
                };
                Number::Ratio(n1 * n2, d1 * d2).simplify()
            }
            (num1, num2) => (f64::from(num1) / f64::from(num2)).into(),
        }
    }
}

impl Rem for Number {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Number::Ratio(n1, d1), Number::Ratio(n2, d2)) => {
                let mut rem = (n1 * d2 as i64) % (n2 * d1 as i64);
                if rem < 0 {
                    rem += n2 * d1 as i64;
                }
                Number::Ratio(rem, d1 * d2).simplify()
            }
            (num1, num2) => f64::from(num1).rem_euclid(f64::from(num2)).into(),
        }
    }
}

impl AddAssign for Number {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl SubAssign for Number {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}

impl MulAssign for Number {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}

impl DivAssign for Number {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs
    }
}

impl RemAssign for Number {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs
    }
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
