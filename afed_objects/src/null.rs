use std::fmt::{Display, Error, Formatter};
use std::vec::Vec;

use super::Object;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
name_type! {null: Null}
impl_operable! {Null:
    //! Null value. Exists for compatability with JSON
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

impl From<Null> for Object {
    fn from(n: Null) -> Self {
        Object::new(n)
    }
}
