use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, Castable,
    NamedType, ErrObject, EvalError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
name_type!{string: Str}

impl_operable!{Str:
    #[binary(Add)]
    fn _(own: String, other: String) -> String
        { own + &other }

    #[binary(Mul, comm)]
    fn _(own: String, num: usize) -> String
        { own.repeat(num) }

    #[call]
    fn __call(&self, idx: usize) -> Object {
        if let Some(c) = self.0.chars().skip(idx).next() {
            c.to_string().into()
        } else { eval_err!("Index {} is out of bounds", idx) }
    }

    pub fn len(&self) -> usize { self.0.len() }
    pub fn lower(&self) -> String { self.0.to_lowercase() }
    pub fn upper(&self) -> String { self.0.to_uppercase() }
}


impl From<Str> for String {
    fn from(s: Str) -> Self { s.0 }
}

impl From<Str> for Object {
    fn from(s: Str) -> Self { Object::new(s) }
}

impl From<String> for Str {
    fn from(s: String) -> Self { Str(s) }
}

impl From<String> for Object {
    fn from(s: String) -> Self { Object::new(Str(s)) }
}

impl From<&str> for Str {
    fn from(s: &str) -> Self { Str(s.to_owned()) }
}

impl From<&str> for Object {
    fn from(s: &str) -> Self { Object::new(Str::from(s)) }
}

impl Castable for String {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>
        { Ok(Str::cast(obj)?.0) }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0.escape_default())
    }
}

