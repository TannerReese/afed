use std::mem::swap;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};
use super::number::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str(pub String);
impl NamedType for Str { fn type_name() -> &'static str { "string" } }

impl Operable for Str {
    type Output = Object;
    unary_not_impl!{}

    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add => other.is_a::<Str>(),
        Binary::Mul => other.is_a::<Number>(),
        _ => false,
    }}

    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        let Str(mut s1) = self;

        match op {
            Binary::Add => {
                let Str(mut s2) = try_cast!(other);
                if rev { swap(&mut s1, &mut s2); }
                s1.push_str(s2.as_str())
            },
            Binary::Mul => {
                if let Some(idx) = try_cast!(other => Number).as_index() {
                    s1 = s1.repeat(idx);
                } else { return eval_err!(
                    "Can only multiply string by positive integer"
                )}
            },
            _ => panic!(),
        }
        s1.into()
    }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some("len") => Some(0),
        Some("lower") => Some(0),
        Some("upper") => Some(0),
        _ => None,
    }}

    fn call<'a>(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Object { match attr {
        None => {
            if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
                if let Some(c) = self.0.chars().skip(idx).next()
                    { c.to_string().into() }
                else { eval_err!("Index {} is out of bounds", idx) }
            } else { eval_err!("Index could not be cast to correct integer") }
        },

        Some("len") => self.0.len().into(),
        Some("lower") => self.0.to_lowercase().into(),
        Some("upper") => self.0.to_uppercase().into(),
        _ => panic!(),
    }}
}

impl From<Str> for Object {
    fn from(s: Str) -> Self { Object::new(s) }
}

impl From<String> for Object {
    fn from(s: String) -> Self { Object::new(Str(s)) }
}

impl Display for Str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "\"{}\"", self.0.escape_default())
    }
}

