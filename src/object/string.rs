use std::mem::swap;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
impl NamedType for Str { fn type_name() -> &'static str { "string" } }

impl Operable for Str {
    type Output = Object;
    unary_not_impl!{}
    
    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add => other.is_a::<Str>(),
        _ => false,
    }}
    
    fn binary(self, rev: bool, op: Binary, other: Object) -> Self::Output {
        let Str(mut s1) = self;
        let Str(mut s2) = try_cast!(other);
        if rev { swap(&mut s1, &mut s2); }
        
        match op {
            Binary::Add => s1.push_str(s2.as_str()),
            _ => panic!(),
        }
        Str(s1).into()
    }
    
    call_not_impl!{Self}
}

impl From<Str> for Object {
    fn from(s: Str) -> Self { Object::new(s) }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0)
    }
}

