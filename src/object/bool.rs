use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
impl NamedType for Bool { fn type_name() -> &'static str { "boolean" }}
impl Objectish for Bool {}

impl Bool {
    pub fn new(b: bool) -> Object { Object::new(Bool(b)) }
}

impl Operable for Bool {
    type Output = Object;
    fn apply_unary(self, op: Unary) -> Self::Output {
        Object::new(match op {
            Unary::Neg => Bool(!self.0),
        })
    }
    
    fn apply_binary(self, op: Binary, other: Object) -> Self::Output {
        let Bool(b1) = self;
        let Bool(b2) = try_expect!(other);
        
        Bool::new(match op {
            Binary::And => b1 && b2,
            Binary::Or => b1 || b2,
            Binary::Add | Binary::Sub => b1 ^ b2,
            Binary::Mul => b1 && b2,
            _ => return binary_not_impl!(op, Self),
        })
    }
    
    call_not_impl!{Self}
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ternary();
impl NamedType for Ternary { fn type_name() -> &'static str { "ternary" }}
impl Objectish for Ternary {}

impl Operable for Ternary {
    type Output = Object;
    fn apply_unary(self, op: Unary) -> Self::Output {
        unary_not_impl!(op, Self)
    }
    
    fn apply_binary(self, op: Binary, _: Object) -> Self::Output {
        binary_not_impl!(op, Self)
    }
    
    fn arity(&self) -> usize { 3 }
    fn apply_call(&self, mut args: Vec<Object>) -> Self::Output {
        let Bool(cond) = try_expect!(args.remove(0));
        args.remove(if cond { 0 } else { 1 })
    }
}

impl Display for Ternary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> { write!(f, "if") }
}

