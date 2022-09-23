use std::any::Any;
use core::slice::Iter;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
impl_objectish!{Str}

impl Operable<Object> for Str {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, "string"))
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        let mut s = std::mem::take(&mut self.0);
        let Str(s2) = other.downcast_ref::<Str>()
            .ok_or(eval_err!("String can only be combined with string"))?;
        
        match op {
            Binary::Add => s.push_str(s2.as_str()),
            _ => return Err(binary_not_impl!(op, "string")),
        }
        Ok(Object::new(Str(s)))
    }
   
    // String does not support calling
    fn arity(&self) -> (usize, usize) { (0, 0) }
    fn apply_call<'a>(&self, _: Iter<'a, Object>) -> Self::Output {
        Err(eval_err!("Cannot call string"))
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0)
    }
}

