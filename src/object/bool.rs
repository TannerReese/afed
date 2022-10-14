use std::mem::swap;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
impl NamedType for Bool { fn type_name() -> &'static str { "boolean" }}

impl Bool {
    pub fn new(b: bool) -> Object { Bool(b).into() }
}

impl Operable for Bool {
    type Output = Object;
    fn unary(self, op: Unary) -> Option<Object> { match op {
        Unary::Neg => Some(Bool::new(!self.0)),
    }}
    
    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::And | Binary::Or | Binary::Add | Binary::Mul => other.is_a::<Bool>(),
        _ => false,
    }}
    
    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        let Bool(mut b1) = self;
        let Bool(mut b2) = try_cast!(other);
        if rev { swap(&mut b1, &mut b2); }
        
        Bool::new(match op {
            Binary::And => b1 && b2,
            Binary::Or => b1 || b2,
            Binary::Add | Binary::Sub => b1 ^ b2,
            Binary::Mul => b1 && b2,
            _ => panic!(),
        })
    }
    
    call_not_impl!{Self}
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl From<Bool> for Object {
    fn from(b: Bool) -> Object { Object::new(b) }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ternary();
impl NamedType for Ternary { fn type_name() -> &'static str { "ternary" }}

impl Operable for Ternary {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    fn arity(&self) -> usize { 3 }
    fn call(&self, mut args: Vec<Object>) -> Self::Output {
        let Bool(cond) = try_cast!(args.remove(0));
        args.remove(if cond { 0 } else { 1 })
    }
}

impl Display for Ternary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> { write!(f, "if") }
}

impl From<Ternary> for Object {
    fn from(t: Ternary) -> Self { Object::new(t) }
}

