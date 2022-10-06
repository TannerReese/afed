use std::any::Any;
use std::vec::Vec;
use std::fmt::{Debug, Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError};


#[derive(Clone, Copy)]
pub struct BltnFuncSingle<A> {
    pub name: &'static str,
    ptr: fn(A) -> Object,
}

impl<A: 'static> NamedType for BltnFuncSingle<A> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A> Objectish for BltnFuncSingle<A> where A: NamedType + Objectish + Clone
{ impl_objectish!{} }

impl<A> BltnFuncSingle<A> where A: NamedType + Objectish + Clone {
    pub fn new(name: &'static str, ptr: fn(A) -> Object) -> Object {
        Object::new(BltnFuncSingle {name, ptr})
    }
}

impl<A> Operable<Object> for BltnFuncSingle<A> where A: NamedType + Objectish + Clone {
    type Output = Object;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        unary_not_impl!(op, self)
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        unary_not_impl!(op, self)
    }
    
    fn arity(&self) -> usize { 1 }
    fn apply_call(&self, mut args: Vec<Object>) -> Self::Output {
        (self.ptr)(try_expect!(args.remove(0)))
    }
}

impl<A> Display for BltnFuncSingle<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<A> Debug for BltnFuncSingle<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "BltnFuncSingle {{ name: {}, ptr: {} }}",
            self.name, self.ptr as usize
        )
    }
}

impl<A> PartialEq for BltnFuncSingle<A> {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl<A> Eq for BltnFuncSingle<A> {}





#[derive(Clone, Copy)]
pub struct BltnFuncDouble<A, B> {
    pub name: &'static str,
    ptr: fn(A, B) -> Object,
}

impl<A: 'static, B: 'static> NamedType for BltnFuncDouble<A, B> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A, B> Objectish for BltnFuncDouble<A, B>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
{ impl_objectish!{} }

impl<A, B> BltnFuncDouble<A, B>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
{
    pub fn new(name: &'static str, ptr: fn(A, B) -> Object) -> Object {
        Object::new(BltnFuncDouble {name, ptr})
    }
}

impl<A, B> Operable<Object> for BltnFuncDouble<A, B>
where
    A: NamedType + Objectish + Clone,
    B: NamedType + Objectish + Clone,
{
    type Output = Object;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        unary_not_impl!(op, self)
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        unary_not_impl!(op, self)
    }
    
    fn arity(&self) -> usize { 2 }
    fn apply_call(&self, mut args: Vec<Object>) -> Self::Output {
        let x = try_expect!(args.remove(0));
        let y = try_expect!(args.remove(0));
        (self.ptr)(x, y)
    }
}

impl<A, B> Display for BltnFuncDouble<A, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<A, B> Debug for BltnFuncDouble<A, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "BltnFuncDouble {{ name: {}, ptr: {} }}",
            self.name, self.ptr as usize
        )
    }
}

impl<A, B> PartialEq for BltnFuncDouble<A, B> {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl<A, B> Eq for BltnFuncDouble<A, B> {}

