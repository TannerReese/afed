use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, CastObject,
    NamedType, ErrObject, EvalError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
name_type!{string: Str}

impl Operable for Str {
    def_unary!{}
    def_binary!{self,
        self + other : (Str => String) = { self.0 + &other },
        (self ~) * num : (Number => usize) = { self.0.repeat(num) }
    }
    def_methods!{Str(s),
        __call(idx: usize) = if let Some(c) = s.chars().skip(idx).next() {
            c.to_string().into()
        } else { eval_err!("Index {} is out of bounds", idx) },

        len() = s.len().into(),
        lower() = s.to_lowercase().into(),
        upper() = s.to_uppercase().into()
    }
}


impl From<Str> for Object {
    fn from(s: Str) -> Self { Object::new(s) }
}

impl From<String> for Object {
    fn from(s: String) -> Self { Object::new(Str(s)) }
}

impl CastObject for String {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>
        { Ok(Str::cast(obj)?.0) }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0.escape_default())
    }
}

