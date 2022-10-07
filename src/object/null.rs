use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
impl NamedType for Null { fn type_name() -> &'static str { "null" }}
impl Objectish for Null {}

impl Operable for Null {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    call_not_impl!{Self}
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

