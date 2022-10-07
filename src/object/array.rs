use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};
use super::number::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
impl NamedType for Array { fn type_name() -> &'static str { "array" }}
impl Objectish for Array {}

impl Operable for Array {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    fn arity(&self) -> usize { 1 }
    fn call<'a>(&self, mut args: Vec<Object>) -> Self::Output {
        if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
            if let Some(obj) = self.0.get(idx) { obj.clone() }
            else { eval_err!("Index {} is out of bounds", idx) }
        } else { eval_err!("Index could not be cast to correct integer") }
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('[')?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}


impl<const N: usize> From<[Object; N]> for Object {
    fn from(arr: [Object; N]) -> Object {
        Array(arr.into()).into()
    }
}

