use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use std::hash::Hash;
use std::borrow::Borrow;

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, CastObject,
    NamedType, ErrObject, EvalError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map(pub HashMap<String, Object>);
name_type!{map: Map}

impl Map {
    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    { self.0.get(key) }
}

impl Operable for Map {
    def_unary!{}
    def_binary!{self,
        self + other : (Map) = {
            let mut s = self;
            s.0.extend(other.0);  s
        }
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some(_) => Some(0),
    }}

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Object {
        let s: String;
        let key = if let Some(key) = attr { key }
        else { s = cast!(args.remove(0)); s.as_str() };
        self.0.get(key).map(|obj| obj.clone()).unwrap_or(
            eval_err!("Key {} is not contained in map", key)
        )
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('{')?;
        let mut is_first = true;
        for (key, obj) in self.0.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;

            let mut chars = key.chars();
            if let Some(c) = chars.next() {
                if c.is_ascii_alphabetic()
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    write!(f, "{}: {}", key, obj)?;
                    continue;
                }
            }
            write!(f, "\"{}\": {}", key, obj)?;
        }
        f.write_char('}')
    }
}

impl From<Map> for Object {
    fn from(map: Map) -> Self {
        if map.0.values().any(|elm| elm.is_err()) {
            map.0.into_values()
            .filter(|elm| elm.is_err())
            .next().unwrap()
        } else { Object::new(map) }
    }
}

impl From<HashMap<String, Object>> for Object {
    fn from(map: HashMap<String, Object>) -> Object { Map(map).into() }
}

impl CastObject for HashMap<String, Object> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>
        { Ok(Map::cast(obj)?.0) }
}

impl<const N: usize> From<[(&str, Object); N]> for Object {
    fn from(arr: [(&str, Object); N]) -> Object {
        Map(arr.map(|(key, obj)| (key.to_owned(), obj)).into()).into()
    }
}

