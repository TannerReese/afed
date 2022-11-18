use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
name_type!{null: Null}

impl Operable for Null {
    def_unary!{}
    def_binary!{}

    def_methods!{}
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

impl From<Null> for Object {
    fn from(n: Null) -> Self { Object::new(n) }
}

