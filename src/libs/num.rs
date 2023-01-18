use super::bltn_func::BltnFunc;
use crate::expr::Bltn;
use crate::object::Object;

create_bltns! {num:
    fn pi() -> f64 { std::f64::consts::PI }
    fn e() -> f64 { std::f64::consts::E }
    fn gold() -> f64 { (1.0 + (5.0_f64).sqrt()) / 2.0 }

    /// num.signum (x: any) -> any
    /// Call method 'signum' on 'x'
    fn signum(obj: Object) -> Object { call!(obj.signum) }
    /// num.abs (x: any) -> any
    /// Call method 'abs' on 'x'
    fn abs(obj: Object) -> Object { call!(obj.abs) }
    /// num.real (x: any) -> any
    /// Call method 'real' on 'x'
    fn real(obj: Object) -> Object { call!(obj.real) }
    /// num.floor (x: any) -> any
    /// Call method 'floor' on 'x'
    fn floor(obj: Object) -> Object { call!(obj.floor) }
    /// num.ceil (x: any) -> any
    /// Call method 'ceil' on 'x'
    fn ceil(obj: Object) -> Object { call!(obj.ceil) }
    /// num.round (x: any) -> any
    /// Call method 'round' on 'x'
    fn round(obj: Object) -> Object { call!(obj.round) }


    /// num.has_inv (x: any) -> any
    /// Call method 'has_inv' on 'x'
    fn has_inv(obj: Object) -> Object { call!(obj.has_inv) }
    /// num.inv (x: any) -> any
    /// Call method 'inv' on 'x'
    fn inv(obj: Object) -> Object { call!(obj.inv) }
    /// num.sqrt (x: any) -> any
    /// Call method 'sqrt' on 'x'
    fn sqrt(obj: Object) -> Object { call!(obj.sqrt) }
    /// num.cbrt (x: any) -> any
    /// Call method 'cbrt' on 'x'
    fn cbrt(obj: Object) -> Object { call!(obj.cbrt) }

    /// num.sin (x: any) -> any
    /// Call method 'sin' on 'x'
    fn sin(obj: Object) -> Object { call!(obj.sin) }
    /// num.cos(x: any) -> any
    /// Call method 'cos' on 'x'
    fn cos(obj: Object) -> Object { call!(obj.cos) }
    /// num.tan (x: any) -> any
    /// Call method 'tan' on 'x'
    fn tan(obj: Object) -> Object { call!(obj.tan) }
    /// num.sinh (x: any) -> any
    /// Call method 'sinh' on 'x'
    fn sinh(obj: Object) -> Object { call!(obj.sinh) }
    /// num.cosh (x: any) -> any
    /// Call method 'cosh' on 'x'
    fn cosh(obj: Object) -> Object { call!(obj.cosh) }
    /// num.tanh (x: any) -> any
    /// Call method 'tanh' on 'x'
    fn tanh(obj: Object) -> Object { call!(obj.tanh) }
    /// num.asin (x: any) -> any
    /// Call method 'asin' on 'x'
    fn asin(obj: Object) -> Object { call!(obj.asin) }
    /// num.acos (x: any) -> any
    /// Call method 'acos' on 'x'
    fn acos(obj: Object) -> Object { call!(obj.acos) }
    /// num.atan (x: any) -> any
    /// Call method 'atan' on 'x'
    fn atan(obj: Object) -> Object { call!(obj.atan) }
    /// num.asinh (x: any) -> any
    /// Call method 'asinh' on 'x'
    fn asinh(obj: Object) -> Object { call!(obj.asinh) }
    /// num.acosh (x: any) -> any
    /// Call method 'acosh' on 'x'
    fn acosh(obj: Object) -> Object { call!(obj.acosh) }
    /// num.atanh (x: any) -> any
    /// Call method 'atanh' on 'x'
    fn atanh(obj: Object) -> Object { call!(obj.atanh) }
    /// num.atan2 (y: any) (x: any) -> any
    /// Call method 'atan2' on 'y' with argument 'x'
    fn atan2(y: Object, x: Object) -> Object { call!(y.atan2(x)) }

    /// num.exp (x: any) -> any
    /// Call method 'exp' on 'x'
    fn exp(obj: Object) -> Object { call!(obj.exp) }
    /// num.exp2 (x: any) -> any
    /// Call method 'exp2' on 'x'
    fn exp2(obj: Object) -> Object { call!(obj.exp2) }
    /// num.ln (x: any) -> any
    /// Call method 'ln' on 'x'
    fn ln(obj: Object) -> Object { call!(obj.ln) }
    /// num.log10 (x: any) -> any
    /// Call method 'log10' on 'x'
    fn log10(obj: Object) -> Object { call!(obj.log10) }
    /// num.log2 (x: any) -> any
    /// Call method 'log2' on 'x'
    fn log2(obj: Object) -> Object { call!(obj.log2) }
    /// num.log (base: any) (x: any) -> any
    /// Call method 'log' on 'base' with argument 'x'
    fn log(base: Object, x: Object) -> Object { call!(base.log(x)) }

    /// num.gcd (a: any) (b: any) -> any
    /// Call method 'gcd' on 'a' with argument 'b'
    fn gcd(a: Object, b: Object) -> Object { call!(a.gcd(b)) }
    /// num.lcm (a: any) (b: any) -> any
    /// Call method 'lcm' on 'a' with argument 'b'
    fn lcm(a: Object, b: Object) -> Object { call!(a.lcm(b)) }

    /// num.factorial (x: any) -> any
    /// Call method 'factorial' on 'x'
    fn factorial(obj: Object) -> Object { call!(obj.factorial) }
    /// num.choose (n: any) (k: any) -> any
    /// Call method 'choose' on 'n' with argument 'k'
    fn choose(n: Object, k: Object) -> Object { call!(n.choose(k)) }
}
