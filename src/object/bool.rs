use std::any::Any;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
impl NamedType for Bool { fn type_name() -> &'static str { "boolean" }}
impl Objectish for Bool { impl_objectish!{} }

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
        let Bool(b2) = other.downcast::<Bool>()?;
        
        match op {
            Binary::Add | Binary::Sub => b ^= b2,
            Binary::Mul => b &= b2,
            _ => return Err(binary_not_impl!(op, self)),
        }
        Ok(Object::new(Bool(b)))
    }
    
    call_not_impl!{Self}
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

