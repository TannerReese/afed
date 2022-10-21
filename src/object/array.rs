use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};
use super::bool::Bool;
use super::number::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
impl NamedType for Array { fn type_name() -> &'static str { "array" } }

impl Operable for Array {
    type Output = Object;
    unary_not_impl!{}

    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add => other.is_a::<Array>(),
        Binary::Mul => other.is_a::<Number>(),
        _ => false,
    }}

    fn binary(self, rev: bool, op: Binary, other: Object) -> Object {
        let Array(mut arr1) = self;

        match op {
            Binary::Add => {
                let Array(mut arr2) = try_cast!(other);
                if rev { std::mem::swap(&mut arr1, &mut arr2); }
                arr1.append(&mut arr2);
            },
            Binary::Mul => {
                if let Some(idx) = try_cast!(other => Number).as_index() {
                    arr1 = std::iter::repeat(arr1)
                    .take(idx).flatten().collect();
                } else { return eval_err!(
                    "Can only multiply array by positive integer"
                )}
            },
            _ => panic!(),
        }
        arr1.into()
    }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some("len") => Some(0),
        Some("map") => Some(1),
        Some("filter") => Some(1),
        Some("sum") => Some(0),
        Some("prod") => Some(0),
        _ => None,
    }}

    fn call<'a>(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Object { match attr {
        None => {
            if let Some(idx) = try_cast!(args.remove(0) => Number).as_index() {
                if let Some(obj) = self.0.get(idx) { obj.clone() }
                else { eval_err!("Index {} is out of bounds", idx) }
            } else { eval_err!("Index could not be cast to correct integer") }
        },

        Some("len") => self.0.len().into(),
        Some("map") => {
            let func = args.remove(0);
            self.0.iter().cloned().map(|elem|
                func.call(None, vec![elem])
            ).collect()
        },
        Some("filter") => {
            let pred = args.remove(0);
            let mut new_arr = Vec::with_capacity(self.0.len());
            for elem in self.0.iter() {
                let res = pred.call(None, vec![elem.clone()]);
                if try_cast!(res => Bool).0 { new_arr.push(elem.clone()); }
            }
            Array(new_arr).into()
        },

        Some("sum") => self.0.iter().cloned().sum(),
        Some("prod") => self.0.iter().cloned().product(),
        _ => panic!(),
    }}
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


impl FromIterator<Object> for Array {
    fn from_iter<T: IntoIterator<Item=Object>>(iter: T) -> Self
        { Array(Vec::from_iter(iter)) }
}

impl FromIterator<Object> for Object {
    fn from_iter<T: IntoIterator<Item=Object>>(iter: T) -> Self
        { Array(Vec::from_iter(iter)).into() }
}

impl From<Vec<Object>> for Object {
    fn from(objs: Vec<Object>) -> Self {
        if objs.iter().any(|elm| elm.is_err()) {
            objs.into_iter()
            .filter(|elm| elm.is_err())
            .next().unwrap()
        } else { Object::new(Array(objs)) }
    }
}

impl From<Array> for Object {
    fn from(arr: Array) -> Self { arr.0.into() }
}

impl<const N: usize> From<[Object; N]> for Object {
    fn from(arr: [Object; N]) -> Object {
        Array(arr.into()).into()
    }
}

