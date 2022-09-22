use std::any::Any;
use core::slice::Iter;
use std::fmt::{Display, Formatter, Error};

use super::super::opers::{Unary, Binary};
use super::{Operable, Object, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
impl_objectish!{Null}

impl Operable<Object> for Null {
    type Output = EvalResult;
    fn apply_unary(&mut self, _: Unary) -> Self::Output {
        Err(eval_err!("Unary operators cannot be applied to null"))
    }
    
    fn apply_binary(&mut self, _: Binary, _: Object) -> Self::Output {
        Err(eval_err!("Binary operators cannot be applied to null"))
    }
   
    // Null does not support calling
    fn arity(&self) -> (usize, usize) { (0, 0) }
    fn apply_call<'a>(&self, _: Iter<'a, Object>) -> Self::Output {
        Err(eval_err!("Cannot call null"))
    }
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

