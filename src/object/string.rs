use std::any::Any;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
impl NamedType for Str { fn type_name() -> &'static str { "string" } }
impl Objectish for Str { impl_objectish!{} }

impl Operable<Object> for Str {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        let mut s = std::mem::take(&mut self.0);
        let Str(s2) = other.downcast::<Str>()?;
        
        match op {
            Binary::Add => s.push_str(s2.as_str()),
            _ => return Err(binary_not_impl!(op, self)),
        }
        Ok(Object::new(Str(s)))
    }
    
    call_not_impl!{Self}
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0)
    }
}

