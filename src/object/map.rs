use std::any::Any;
use core::slice::Iter;
use std::fmt::{Display, Formatter, Error, Write};
use std::collections::HashMap;

use super::opers::{Unary, Binary};
use super::{Operable, Object, Objectish, EvalError, EvalResult};
use super::string::Str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub unnamed: Vec<Object>,
    pub named: HashMap<String, Object>,
}
impl_objectish!{Map}

impl Map {
    pub fn from_map(named: HashMap<String, Object>) -> Object {
        Object::new(Map {unnamed: Vec::new(), named})
    }
}

impl Operable<Object> for Map {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, "map"))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(binary_not_impl!(op, "map"))
    }
   
    fn arity(&self) -> (usize, usize) { (1, 1) }
    fn apply_call<'a>(&self, mut args: Iter<'a, Object>) -> Self::Output {
        let key = args.next().ok_or(eval_err!("No key provided for map call"))?;
        let Str(key) = key.downcast_ref::<Str>()
            .ok_or(eval_err!("Key for map call is not a string"))?;
        
        if let Some(obj) = self.named.get(key.as_str()) {
            obj.apply_call(args)
        } else { Err(eval_err!("Key {} is not contained in map", &key)) }
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('{')?;
        let mut is_first = true;
        for obj in self.unnamed.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", *obj)?;
        }
        
        for (key, obj) in self.named.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}: {}", key, *obj)?;
        }
        f.write_char('}')
    }
}

