use std::collections::HashMap;

use crate::object::{Object, EvalError};
use crate::object::number::Number;
use crate::object::bltn_func::{BltnFuncSingle, BltnFuncDouble};

macro_rules! real_func {
    ($pkg:ident.$name:ident) => {
        def_bltn!($pkg.$name(n: Number) = n.to_real().$name().into())
    };
}

fn choose(n: Number, k: Number) -> Option<Number> {
    Some((0..k.as_index()?).map(|i| (i as i64).into())
    .fold(1.into(), |accum, i|
        accum * (n - i) / (i + 1.into())
    ))
}

fn factorial(n: Number) -> Option<Number> {
    Some(((1..n.as_index()? + 1).product::<usize>() as i64).into())
}


pub fn make_bltns() -> Object {
    let mut num = HashMap::new();
    def_bltn!(num.pi = Number::Real(std::f64::consts::PI));
    def_bltn!(num.e = Number::Real(std::f64::consts::E));
    def_bltn!(num.gold = Number::Real((1.0 + (5.0 as f64).sqrt()) / 2.0));
    
    def_bltn!(num.abs(n: Number) = n.abs().into());
    def_bltn!(num.signum(n: Number) = n.signum().into());
    def_bltn!(num.real(n: Number) = n.to_real().into());
    def_bltn!(num.floor(n: Number) = n.floor().into());
    def_bltn!(num.ceil(n: Number) = n.ceil().into());
    real_func!(num.round);
    
    def_bltn!(num.sqrt(n: Number) = n.sqrt().map_or(eval_err!(
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
    
    def_bltn!(num.gcd(a: Number, b: Number) = Number::gcd(a, b).map_or(eval_err!(
        "Cannot take GCD of reals"
    ), &Object::new));
    def_bltn!(num.lcm(a: Number, b: Number) = Number::lcm(a, b).map_or(eval_err!(
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

