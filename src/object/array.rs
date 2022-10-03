use std::any::Any;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};
use super::number::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
impl NamedType for Array { fn type_name() -> &'static str { "array" }}
impl Objectish for Array { impl_objectish!{} }

impl Operable<Object> for Array {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(binary_not_impl!(op, self))
    }
    
    fn arity(&self) -> usize { 1 }
    fn apply_call<'a>(&self, args: Vec<Object>) -> Self::Output {
        let idx = args[0]
            .downcast_ref::<Number>()
            .ok_or(eval_err!("Index for array call is not a number"))?
            .as_index()
            .ok_or(eval_err!("Index could not be cast to correct integer"))?;
        self.0.get(idx).map(|obj| obj.clone()).ok_or(eval_err!("Index {} is out of bounds", idx))
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('[')?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}

