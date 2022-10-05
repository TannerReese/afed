
use crate::object::{Object, EvalError};
use crate::object::number::Number;
use crate::object::bltn_func::{BltnFuncSingle, BltnFuncDouble};

macro_rules! real_func {
    ($name:ident) => {
        (stringify!($name), BltnFuncSingle::new(
            concat!("num.", stringify!($name)),
            |n: Number| Ok(Number::Real(n.to_real().$name()))
        ))
    };
}

fn choose(n: Number, k: Number) -> Option<Number> {
    let k = k.as_index()?;
    let mut work = Number::Ratio(1, 1);
    for i in 0..k {
        let i = i as i64;
        work = work * (n - Number::Ratio(i, 1));
        work = work / Number::Ratio(i + 1, 1);
    }
    Some(work)
}

fn factorial(n: Number) -> Option<Number> {
    let n = n.as_index()?;
    Some(Number::Ratio((1..n).product::<usize>() as i64, 1))
}


pub fn make_bltns() -> Object {[
    ("pi", Number::real(std::f64::consts::PI)),
    ("e", Number::real(std::f64::consts::E)),
    ("gold", Number::real((1.0 + (5.0 as f64).sqrt()) / 2.0)),
    
    ("abs", BltnFuncSingle::new("num.abs", |n: Number| Ok(n.abs()))),
    ("signum", BltnFuncSingle::new("num.signum", |n: Number| Ok(n.signum()))),
    ("floor", BltnFuncSingle::new("num.floor", |n: Number| Ok(n.floor()))),
    ("ceil", BltnFuncSingle::new("num.ceil", |n: Number| Ok(n.ceil()))),
    real_func!(round),
    
    ("sqrt", BltnFuncSingle::new("num.sqrt", |n: Number|
        n.sqrt().ok_or(eval_err!("Cannot take square root of negative"))
    )),
    real_func!(cbrt),
    real_func!(sin), real_func!(cos), real_func!(tan),
    real_func!(sinh), real_func!(cosh), real_func!(tanh),
    real_func!(asin), real_func!(acos), real_func!(atan),
    real_func!(asinh), real_func!(acosh), real_func!(atanh),
    ("atan2", BltnFuncDouble::new("num.atan2", |y: Number, x: Number|
        Ok(Number::Real(y.to_real().atan2(x.to_real())))
    )),
    real_func!(exp), real_func!(exp2),
    real_func!(ln), real_func!(log10), real_func!(log2),
    ("log", BltnFuncDouble::new("num.log", |base: Number, x: Number|
        Ok(Number::Real(x.to_real().log(base.to_real())))
    )),
    
    ("gcd", BltnFuncDouble::new("num.gcd", |a: Number, b: Number|
        Number::gcd(a, b).ok_or(eval_err!("Cannot take GCD of reals"))
    )),
    ("lcm", BltnFuncDouble::new("num.lcm", |a: Number, b: Number|
        Number::lcm(a, b).ok_or(eval_err!("Cannot take LCM of reals"))
    )),
    ("factorial", BltnFuncSingle::new("num.factorial", |n: Number|
        factorial(n).ok_or(eval_err!("Can only take factorial of positive integer"))
    )),
    ("choose", BltnFuncDouble::new("num.choose", |n: Number, k: Number|
        choose(n, k).ok_or(eval_err!("Second argument to choose must be a positive integer"))
    )),
].into()}

