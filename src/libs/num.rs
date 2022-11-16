use std::collections::HashMap;
use super::bltn_func::BltnFunc;
use crate::expr::Bltn;
use crate::object::Object;

pub fn make_bltns() -> Bltn {
    let mut num = HashMap::new();
    def_bltn!(num.pi = std::f64::consts::PI);
    def_bltn!(num.e = std::f64::consts::E);
    def_bltn!(num.gold = (1.0 + (5.0 as f64).sqrt()) / 2.0);

    def_getter!(num.signum);  def_getter!(num.abs);
    def_getter!(num.real);
    def_getter!(num.floor);   def_getter!(num.ceil);   def_getter!(num.round);

    def_getter!(num.inv);     def_getter!(num.has_inv);
    def_getter!(num.sqrt);    def_getter!(num.cbrt);
    def_getter!(num.sin);     def_getter!(num.cos);    def_getter!(num.tan);
    def_getter!(num.sinh);    def_getter!(num.cosh);   def_getter!(num.tanh);
    def_getter!(num.asin);    def_getter!(num.acos);   def_getter!(num.atan);
    def_getter!(num.asinh);   def_getter!(num.acosh);  def_getter!(num.atanh);
    def_bltn!(num.atan2(y: Object, x: Object) = call!(y.atan2(x)));

    def_getter!(num.exp);     def_getter!(num.exp2);
    def_getter!(num.ln);      def_getter!(num.log10);  def_getter!(num.log2);
    def_bltn!(num.log(base: Object, x: Object) = call!(base.log(x)));

    def_bltn!(num.gcd(a: Object, b: Object) = call!(a.gcd(b)));
    def_bltn!(num.lcm(a: Object, b: Object) = call!(a.lcm(b)));

    def_getter!(num.factorial);
    def_bltn!(num.choose(n: Object, k: Object) = call!(n.choose(k)));

    Bltn::Map(num)
}

