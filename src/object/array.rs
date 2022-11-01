use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::cmp::Ordering;
use std::iter::repeat;

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, CastObject,
    NamedType, EvalError,
};
use super::bool::Bool;
use super::number::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
impl NamedType for Array { fn type_name() -> &'static str { "array" } }

impl Operable for Array {
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
                let mut arr2 = try_cast!(other);
                if rev { std::mem::swap(&mut arr1, &mut arr2); }
                arr1.append(&mut arr2);
                arr1
            },
            Binary::Mul => {
                let idx: usize = try_cast!(other);
                repeat(arr1).take(idx).flatten().collect()
            },
            _ => panic!(),
        }.into()
    }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some("len") => Some(0),
        Some("fst") | Some("snd") => Some(0),
        Some("map") => Some(1),
        Some("filter") => Some(1),
        Some("all") | Some("any") => Some(1),
        Some("has") => Some(1),

        Some("sum") | Some("prod") => Some(0),
        _ => None,
    }}

    fn call<'a>(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Object { match attr {
        None => {
            let idx: usize = try_cast!(args.remove(0));
            if let Some(obj) = self.0.get(idx) { obj.clone() }
            else { eval_err!("Index {} is out of bounds", idx) }
        },

        Some("len") => self.0.len().into(),
        Some("fst") => self.0.get(0).cloned().unwrap_or(
            eval_err!("Array doesn't have a first element")
        ),
        Some("snd") => self.0.get(1).cloned().unwrap_or(
            eval_err!("Array doesn't have a second element")
        ),

        Some("map") => {
            let func = args.remove(0);
            self.0.iter().cloned().map(|elem|
                obj_call!(func(elem))
            ).collect()
        },
        Some("filter") => {
            let pred = args.remove(0);
            let mut new_arr = Vec::with_capacity(self.0.len());
            for elem in self.0.iter() {
                if obj_call!(pred(elem.clone()) => Bool).0 {
                    new_arr.push(elem.clone());
                }
            }
            Array(new_arr).into()
        },

        Some("all") => self.all(args.remove(0)),
        Some("any") => self.any(args.remove(0)),
        Some("has") => {
            let target = args.remove(0);
            self.0.contains(&target).into()
        },

        Some("sum") => self.0.iter().cloned().sum(),
        Some("prod") => self.0.iter().cloned().product(),
        _ => panic!(),
    }}
}

impl Array {
    fn all(&self, pred: Object) -> Object {
        let mut is_all = true;
        for elem in self.0.iter() {
            if !obj_call!(pred(elem.clone()) => Bool).0 {
                is_all = false;
                break;
            }
        }
        is_all.into()
    }

    fn any(&self, pred: Object) -> Object {
        let mut is_any = false;
        for elem in self.0.iter() {
            if obj_call!(pred(elem.clone()) => Bool).0 {
                is_any = true;
                break;
            }
        }
        is_any.into()
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

impl CastObject for Vec<Object> {
    fn cast(obj: Object) -> Result<Self, Object> { Ok(obj.cast::<Array>()?.0) }
}

impl<const N: usize> From<[Object; N]> for Object {
    fn from(arr: [Object; N]) -> Object {
        Array(arr.into()).into()
    }
}

