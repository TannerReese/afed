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
    //! String of characters that are substituted with quotes

    #[binary(Add)]
    /// string + string -> string
    /// Concatenate the strings
    fn _(own: String, other: String) -> String
        { own + &other }

    #[binary(Mul, comm)]
    /// string * (n: natural) -> string
    /// (n: natural) * string -> string
    /// Concatenate 'n' copies of a string
    fn _(own: String, num: usize) -> String
        { own.repeat(num) }

    #[call]
    /// string (i: natural) -> string
    /// Return character at index 'i' of string
    fn __call(&self, idx: usize) -> Object {
        if let Some(c) = self.0.chars().skip(idx).next() {
            c.to_string().into()
        } else { eval_err!("Index {} is out of bounds", idx) }
    }

    /// string.len -> natural
    /// Number of characters in string
    pub fn len(&self) -> usize { self.0.len() }
    /// string.lower -> string
    /// Convert all alphabetic characters to lowercase
    pub fn lower(&self) -> String { self.0.to_lowercase() }
    /// string.upper -> string
    /// Convert all alphabetic characters to uppercase
    pub fn upper(&self) -> String { self.0.to_uppercase() }
    /// string.print -> print-string
    /// Create print-string from string
    pub fn print(self) -> PrintStr { PrintStr(self.0) }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { write!(f, "\"{}\"", self.0.escape_default()) }
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintStr(pub String);
name_type!{"printer string": PrintStr}

impl_operable!{PrintStr:
    //! Series of characters that are substituted raw

    /// as_string -> string
    /// Convert print-string back into string
    fn as_string(self) -> String { self.0 }
}

impl From<PrintStr> for String {
    fn from(s: PrintStr) -> Self { s.0 }
}

impl From<PrintStr> for Object {
    fn from(s: PrintStr) -> Self { Object::new(s) }
}

impl From<String> for PrintStr {
    fn from(s: String) -> Self { PrintStr(s) }
}

impl From<&str> for PrintStr {
    fn from(s: &str) -> Self { PrintStr(s.to_owned()) }
}

impl Display for PrintStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { write!(f, "{}", self.0) }
}

