use std::any::Any;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
impl NamedType for Bool { fn type_name() -> &'static str { "boolean" }}
impl Objectish for Bool { impl_objectish!{} }

impl Bool {
    pub fn new(b: bool) -> Object { Object::new(Bool(b)) }
}

impl Operable<Object> for Bool {
    type Output = Object;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        let Bool(mut b) = self;
        match op {
            Unary::Neg => b = !b,
        }
        Object::new(Bool(b))
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        let &mut Bool(b1) = self;
        let Bool(b2) = try_expect!(other);
        
        Bool::new(match op {
            Binary::And => b1 && b2,
            Binary::Or => b1 || b2,
            Binary::Add | Binary::Sub => b1 ^ b2,
            Binary::Mul => b1 && b2,
            _ => return binary_not_impl!(op, self),
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
impl Objectish for Ternary { impl_objectish!{} }

impl Operable<Object> for Ternary {
    type Output = Object;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        unary_not_impl!(op, self)
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        binary_not_impl!(op, self)
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

