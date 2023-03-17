use afed_objects::{call, declare_pkg, Object};

declare_pkg! {math: #![bltn_pkg]
    fn pi() -> f64 { std::f64::consts::PI }
    fn e() -> f64 { std::f64::consts::E }
    fn gold() -> f64 { (1.0 + (5.0_f64).sqrt()) / 2.0 }

    /// math.signum (x: any) -> any
    /// Call method 'signum' on 'x'
    fn signum(obj: Object) -> Object { call!(obj.signum) }
    /// math.abs (x: any) -> any
    /// Call method 'abs' on 'x'
    fn abs(obj: Object) -> Object { call!(obj.abs) }
    /// math.real (x: any) -> any
    /// Call method 'real' on 'x'
    fn real(obj: Object) -> Object { call!(obj.real) }
    /// math.floor (x: any) -> any
    /// Call method 'floor' on 'x'
    fn floor(obj: Object) -> Object { call!(obj.floor) }
    /// math.ceil (x: any) -> any
    /// Call method 'ceil' on 'x'
    fn ceil(obj: Object) -> Object { call!(obj.ceil) }
    /// math.round (x: any) -> any
    /// Call method 'round' on 'x'
    fn round(obj: Object) -> Object { call!(obj.round) }


    /// math.has_inv (x: any) -> any
    /// Call method 'has_inv' on 'x'
    fn has_inv(obj: Object) -> Object { call!(obj.has_inv) }
    /// math.inv (x: any) -> any
    /// Call method 'inv' on 'x'
    fn inv(obj: Object) -> Object { call!(obj.inv) }
    /// math.sqrt (x: any) -> any
    /// Call method 'sqrt' on 'x'
    fn sqrt(obj: Object) -> Object { call!(obj.sqrt) }
    /// math.cbrt (x: any) -> any
    /// Call method 'cbrt' on 'x'
    fn cbrt(obj: Object) -> Object { call!(obj.cbrt) }

    /// math.sin (x: any) -> any
    /// Call method 'sin' on 'x'
    fn sin(obj: Object) -> Object { call!(obj.sin) }
    /// math.cos(x: any) -> any
    /// Call method 'cos' on 'x'
    fn cos(obj: Object) -> Object { call!(obj.cos) }
    /// math.tan (x: any) -> any
    /// Call method 'tan' on 'x'
    fn tan(obj: Object) -> Object { call!(obj.tan) }
    /// math.sinh (x: any) -> any
    /// Call method 'sinh' on 'x'
    fn sinh(obj: Object) -> Object { call!(obj.sinh) }
    /// math.cosh (x: any) -> any
    /// Call method 'cosh' on 'x'
    fn cosh(obj: Object) -> Object { call!(obj.cosh) }
    /// math.tanh (x: any) -> any
    /// Call method 'tanh' on 'x'
    fn tanh(obj: Object) -> Object { call!(obj.tanh) }
    /// math.asin (x: any) -> any
    /// Call method 'asin' on 'x'
    fn asin(obj: Object) -> Object { call!(obj.asin) }
    /// math.acos (x: any) -> any
    /// Call method 'acos' on 'x'
    fn acos(obj: Object) -> Object { call!(obj.acos) }
    /// math.atan (x: any) -> any
    /// Call method 'atan' on 'x'
    fn atan(obj: Object) -> Object { call!(obj.atan) }
    /// math.asinh (x: any) -> any
    /// Call method 'asinh' on 'x'
    fn asinh(obj: Object) -> Object { call!(obj.asinh) }
    /// math.acosh (x: any) -> any
    /// Call method 'acosh' on 'x'
    fn acosh(obj: Object) -> Object { call!(obj.acosh) }
    /// math.atanh (x: any) -> any
    /// Call method 'atanh' on 'x'
    fn atanh(obj: Object) -> Object { call!(obj.atanh) }
    /// math.atan2 (y: any) (x: any) -> any
    /// Call method 'atan2' on 'y' with argument 'x'
    fn atan2(y: Object, x: Object) -> Object { call!(y.atan2(x)) }

    /// math.exp (x: any) -> any
    /// Call method 'exp' on 'x'
    fn exp(obj: Object) -> Object { call!(obj.exp) }
    /// math.exp2 (x: any) -> any
    /// Call method 'exp2' on 'x'
    fn exp2(obj: Object) -> Object { call!(obj.exp2) }
    /// math.ln (x: any) -> any
    /// Call method 'ln' on 'x'
    fn ln(obj: Object) -> Object { call!(obj.ln) }
    /// math.log10 (x: any) -> any
    /// Call method 'log10' on 'x'
    fn log10(obj: Object) -> Object { call!(obj.log10) }
    /// math.log2 (x: any) -> any
    /// Call method 'log2' on 'x'
    fn log2(obj: Object) -> Object { call!(obj.log2) }
    /// math.log (base: any) (x: any) -> any
    /// Call method 'log' on 'base' with argument 'x'
    fn log(base: Object, x: Object) -> Object { call!(base.log(x)) }

    /// math.gcd (a: any) (b: any) -> any
    /// Call method 'gcd' on 'a' with argument 'b'
    fn gcd(a: Object, b: Object) -> Object { call!(a.gcd(b)) }
    /// math.lcm (a: any) (b: any) -> any
    /// Call method 'lcm' on 'a' with argument 'b'
    fn lcm(a: Object, b: Object) -> Object { call!(a.lcm(b)) }

    /// math.factorial (x: any) -> any
    /// Call method 'factorial' on 'x'
    fn factorial(obj: Object) -> Object { call!(obj.factorial) }
    /// math.choose (n: any) (k: any) -> any
    /// Call method 'choose' on 'n' with argument 'k'
    fn choose(n: Object, k: Object) -> Object { call!(n.choose(k)) }
}
