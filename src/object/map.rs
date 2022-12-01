use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use std::hash::Hash;
use std::borrow::Borrow;

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, Castable,
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
    fn unary(self, _: Unary) -> Option<Object> { None }
    fn binary(mut self,
        _: bool, op: Binary, other: Object
    ) -> Result<Object, (Object, Object)> {
        if Binary::Add == op { match other.try_cast::<Map>() {
            Ok(other) => { self.0.extend(other.0); Ok(self.into()) },
            Err(other) => Err((self.into(), other)),
        }} else { Err((self.into(), other)) }
    }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some(_) => Some(0),
    }}

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Object {
        let s: String;
        let key = if let Some(key) = attr { key } else {
            match args.remove(0).cast() {
                Ok(arg) => { s = arg; s.as_str() },
                Err(err) => return err,
            }
        };

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

impl Castable for HashMap<String, Object> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>
        { Ok(Map::cast(obj)?.0) }
}

