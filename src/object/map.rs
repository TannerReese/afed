use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use std::hash::Hash;
use std::borrow::Borrow;

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, EvalError};
use super::string::Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub unnamed: Vec<Object>,
    pub named: HashMap<String, Object>,
}
impl NamedType for Map { fn type_name() -> &'static str { "map" } }

impl Map {
    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    { self.named.get(key) }
}

impl Operable for Map {
    type Output = Object;
    unary_not_impl!{}

    fn try_binary(&self, _: bool, op: Binary, other: &Object) -> bool { match op {
        Binary::Add => other.is_a::<Map>(),
         _ => false,
    }}

    fn binary(mut self, _: bool, op: Binary, other: Object) -> Object {
        if op != Binary::Add { panic!() }
        let Map {unnamed, named} = try_cast!(other);
        self.unnamed.extend(unnamed);
        self.named.extend(named);
        self.into()
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some(_) => Some(0),
    }}

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Object {
        let s;
        let key = if let Some(key) = attr { key }
        else { s = try_cast!(args.remove(0) => Str); s.0.as_str() };
        self.named.get(key).map(|obj| obj.clone()).unwrap_or(
            eval_err!("Key {} is not contained in map", key)
        )
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('{')?;
        let mut is_first = true;
        for obj in self.unnamed.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }

        for (key, obj) in self.named.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}: {}", key, obj)?;
        }
        f.write_char('}')
    }
}

impl From<Map> for Object {
    fn from(map: Map) -> Self {
        if map.unnamed.iter().any(|elm| elm.is_err()) {
            map.unnamed.into_iter()
            .filter(|elm| elm.is_err())
            .next().unwrap()
        } else if map.named.values().any(|elm| elm.is_err()) {
            map.named.into_values()
            .filter(|elm| elm.is_err())
            .next().unwrap()
        } else { Object::new(map) }
    }
}

impl From<HashMap<String, Object>> for Object {
    fn from(map: HashMap<String, Object>) -> Object {
        Map {unnamed: Vec::new(), named: map}.into()
    }
}

impl<const N: usize> From<[(&str, Object); N]> for Object {
    fn from(arr: [(&str, Object); N]) -> Object {
        Map {
            unnamed: Vec::new(),
            named: arr.map(|(key, obj)| (key.to_owned(), obj)).into(),
        }.into()
    }
}

