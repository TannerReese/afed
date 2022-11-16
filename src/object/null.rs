use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
name_type!{null: Null}

impl Operable for Null {
    unary_not_impl!{}
    binary_not_impl!{}

    call_not_impl!{}
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

impl From<Null> for Object {
    fn from(n: Null) -> Self { Object::new(n) }
}

