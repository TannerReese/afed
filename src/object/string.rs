use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
impl NamedType for Str { fn type_name() -> &'static str { "string" } }
impl Objectish for Str {}

impl Operable for Str {
    type Output = Object;
    fn apply_unary(self, op: Unary) -> Self::Output {
        unary_not_impl!(op, Self)
    }
    
    fn apply_binary(self, op: Binary, other: Object) -> Self::Output {
        let Str(mut s1) = self;
        let Str(s2) = try_expect!(other);
        
        match op {
            Binary::Add => s1.push_str(s2.as_str()),
            _ => return binary_not_impl!(op, Self),
        }
        Object::new(Str(s1))
    }
    
    call_not_impl!{Self}
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0)
    }
}

