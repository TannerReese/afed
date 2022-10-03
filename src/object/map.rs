use std::any::Any;
use std::vec::Vec;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use std::hash::Hash;
use std::borrow::Borrow;

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};
use super::string::Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub unnamed: Vec<Object>,
    pub named: HashMap<String, Object>,
}
impl NamedType for Map { fn type_name() -> &'static str { "map" } }
impl Objectish for Map { impl_objectish!{} }

impl Map {
    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq + std::fmt::Debug,
        String: Borrow<B>,
    { self.named.get(key) }
}

impl Operable<Object> for Map {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(binary_not_impl!(op, self))
    }
    
    fn arity(&self) -> usize { 1 }
    fn apply_call(&self, args: Vec<Object>) -> Self::Output {
        let Str(key) = args[0].downcast_ref::<Str>()
            .ok_or(eval_err!("Key for map call is not a string"))?;
        self.named.get(key.as_str()).map(|obj| obj.clone()).ok_or(
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

