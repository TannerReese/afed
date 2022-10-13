use std::collections::HashMap;

use crate::object::{Object, EvalError};
use crate::object::number::Number;
use crate::object::bltn_func::{BltnFuncSingle, BltnFuncDouble};

macro_rules! real_func {
    ($pkg:ident.$name:ident) => {
        def_bltn!($pkg.$name(n: Number) = n.to_real().$name().into())
    };
}


pub fn abs(num: Number) -> Number { match num {
    Number::Ratio(n, d) => Number::Ratio(n.abs(), d),
    Number::Real(r) => Number::Real(r.abs()),
}}

pub fn signum(num: Number) -> Number { Number::Ratio(match num {
    Number::Ratio(n, _) => n.signum(),
    Number::Real(r) => r.signum() as i64
}, 1)}

pub fn floor(num: Number) -> Number { match num {
    Number::Ratio(n, d) => Number::Ratio(if n < 0 {
        (n + 1) / d as i64 - 1
    } else {
        n / d as i64
    }, 1),
    Number::Real(r) => Number::Ratio(r.floor() as i64, 1),
}}

pub fn ceil(num: Number) -> Number { match num {
    Number::Ratio(n, d) => Number::Ratio(if n > 0 {
        (n - 1) / d as i64 + 1
    } else {
        n / d as i64
    }, 1),
    Number::Real(r) => Number::Ratio(r.ceil() as i64, 1),
}}

pub fn sqrt(num: Number) -> Option<Number> {
    let r = num.to_real();
    if r < 0.0 { None }
    else { Some(Number::Real(r.sqrt())) }
}

pub fn gcd(a: Number, b: Number) -> Option<Number> { match (a, b) {
    (Number::Ratio(na, da), Number::Ratio(nb, db)) => Some({
        use crate::object::number::gcd;
        let g = gcd(na.abs() as u64 * db, nb.abs() as u64 * da);
        Number::Ratio(g as i64, da * db)
    }.simplify()),
    _ => None
}}

pub fn lcm(a: Number, b: Number) -> Option<Number> { match (a, b) {
    (Number::Ratio(na, da), Number::Ratio(nb, db)) => Some({
        use crate::object::number::gcd;
        let g = gcd(na.abs() as u64 * db, nb.abs() as u64 * da);
        Number::Ratio(na * nb, g)
    }.simplify()),
    _ => None,
}}

fn factorial(n: Number) -> Option<Number> {
    Some(((1..n.as_index()? + 1).product::<usize>() as i64).into())
}

fn choose(n: Number, k: Number) -> Option<Number> {
    Some((0..k.as_index()?).map(|i| (i as i64).into())
    .fold(1.into(), |accum, i|
        accum * (n - i) / (i + 1.into())
    ))
}


pub fn make_bltns() -> Object {
    let mut num = HashMap::new();
    def_bltn!(num.pi = Number::Real(std::f64::consts::PI));
    def_bltn!(num.e = Number::Real(std::f64::consts::E));
    def_bltn!(num.gold = Number::Real((1.0 + (5.0 as f64).sqrt()) / 2.0));
    
    def_bltn!(num.abs(n: Number) = abs(n).into());
    def_bltn!(num.signum(n: Number) = signum(n).into());
    def_bltn!(num.real(n: Number) = n.to_real().into());
    def_bltn!(num.floor(n: Number) = floor(n).into());
    def_bltn!(num.ceil(n: Number) = ceil(n).into());
    real_func!(num.round);
    
    def_bltn!(num.sqrt(n: Number) = sqrt(n).map_or(eval_err!(
        "Cannot take square root of negative"
    ), &Object::new));
    
    real_func!(num.cbrt);
    real_func!(num.sin); real_func!(num.cos); real_func!(num.tan);
    real_func!(num.sinh); real_func!(num.cosh); real_func!(num.tanh);
    real_func!(num.asin); real_func!(num.acos); real_func!(num.atan);
    real_func!(num.asinh); real_func!(num.acosh); real_func!(num.atanh);
    def_bltn!(num.atan2(y: Number, x: Number) =
        y.to_real().atan2(x.to_real()).into()
    );
    
    real_func!(num.exp); real_func!(num.exp2);
    real_func!(num.ln); real_func!(num.log10); real_func!(num.log2);
    def_bltn!(num.log(base: Number, x: Number) =
        x.to_real().log(base.to_real()).into()
    );
    
    def_bltn!(num.gcd(a: Number, b: Number) = gcd(a, b).map_or(eval_err!(
        "Cannot take GCD of reals"
    ), &Object::new));
    def_bltn!(num.lcm(a: Number, b: Number) = lcm(a, b).map_or(eval_err!(
        "Cannot take LCM of reals"
    ), &Object::new));
    
    def_bltn!(num.factorial(n: Number) = factorial(n).map_or(eval_err!(
        "Can only take factorial of positive integer"
    ), &Object::new));
    def_bltn!(num.choose(n: Number, k: Number) = choose(n, k).map_or(eval_err!(
        "Second argument to choose must be a positive integer"
    ), &Object::new));
    
    num.into()
}

