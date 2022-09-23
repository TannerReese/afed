use std::any::Any;
use core::slice::Iter;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
impl_objectish!{Bool}

impl Operable<Object> for Bool {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        let Bool(mut b) = self;
        match op {
            Unary::Neg => b = !b,
        }
        Ok(Object::new(Bool(b)))
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        let Bool(mut b) = self;
        let Bool(b2) = other.downcast_ref::<Bool>()
            .ok_or(eval_err!("Boolean can only be combined with boolean"))?;
        
        match op {
            Binary::Add | Binary::Sub => b ^= b2,
            Binary::Mul => b &= b2,
            _ => return Err(binary_not_impl!(op, "boolean")),
        }
        Ok(Object::new(Bool(b)))
    }
   
    // Boolean does not support calling
    fn arity(&self) -> (usize, usize) { (0, 0) }
    fn apply_call<'a>(&self, _: Iter<'a, Object>) -> Self::Output {
        Err(eval_err!("Cannot call boolean"))
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

