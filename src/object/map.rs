use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use std::hash::Hash;
use std::borrow::Borrow;

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};
use super::string::Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub unnamed: Vec<Object>,
    pub named: HashMap<String, Object>,
}
impl NamedType for Map { fn type_name() -> &'static str { "map" } }
impl Objectish for Map {}

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
    binary_not_impl!{}
    
    fn arity(&self) -> usize { 1 }
    fn call(&self, mut args: Vec<Object>) -> Self::Output {
        let Str(key) = try_cast!(args.remove(0));
        self.named.get(key.as_str()).map(|obj| obj.clone()).unwrap_or(
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

