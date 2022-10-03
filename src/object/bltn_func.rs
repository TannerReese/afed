use std::any::Any;
use std::vec::Vec;
use std::fmt::{Debug, Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};


#[derive(Clone, Copy)]
pub struct BltnFuncSingle<A, R> {
    pub name: &'static str,
    ptr: fn(A) -> Result<R, EvalError>,
}

impl<A: 'static, R: 'static> NamedType for BltnFuncSingle<A, R> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A, R> Objectish for BltnFuncSingle<A, R>
where
    A: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{ impl_objectish!{} }

impl<A, R> BltnFuncSingle<A, R>
where
    A: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{
    pub fn new(name: &'static str, ptr: fn(A) -> Result<R, EvalError>) -> Object {
        Object::new(BltnFuncSingle {name, ptr})
    }
}

impl<A, R> Operable<Object> for BltnFuncSingle<A, R>
where
    A: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn arity(&self) -> usize { 1 }
    fn apply_call(&self, mut args: Vec<Object>) -> Self::Output {
        let x = args.remove(0).downcast()?;
        Ok(Object::new((self.ptr)(x)?))
    }
}

impl<A, R> Display for BltnFuncSingle<A, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<A, R> Debug for BltnFuncSingle<A, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "BltnFuncSingle {{ name: {}, ptr: {} }}",
            self.name, self.ptr as usize
        )
    }
}

impl<A, R> PartialEq for BltnFuncSingle<A, R> {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl<A, R> Eq for BltnFuncSingle<A, R> {}





#[derive(Clone, Copy)]
pub struct BltnFuncDouble<A, B, R> {
    pub name: &'static str,
    ptr: fn(A, B) -> Result<R, EvalError>,
}

impl<A: 'static, B: 'static, R: 'static> NamedType for BltnFuncDouble<A, B, R> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A, B, R> Objectish for BltnFuncDouble<A, B, R>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{ impl_objectish!{} }

impl<A, B, R> BltnFuncDouble<A, B, R>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{
    pub fn new(name: &'static str, ptr: fn(A, B) -> Result<R, EvalError>) -> Object {
        Object::new(BltnFuncDouble {name, ptr})
    }
}

impl<A, B, R> Operable<Object> for BltnFuncDouble<A, B, R>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
    R: Objectish + Clone,
{
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn arity(&self) -> usize { 2 }
    fn apply_call(&self, mut args: Vec<Object>) -> Self::Output {
        let x = args.remove(0).downcast()?;
        let y = args.remove(0).downcast()?;
        Ok(Object::new((self.ptr)(x, y)?))
    }
}

impl<A, B, R> Display for BltnFuncDouble<A, B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<A: Debug, B, R> Debug for BltnFuncDouble<A, B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "BltnFuncDouble {{ name: {}, ptr: {} }}",
            self.name, self.ptr as usize
        )
    }
}

impl<A, B, R> PartialEq for BltnFuncDouble<A, B, R> {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl<A, B, R> Eq for BltnFuncDouble<A, B, R> {}

