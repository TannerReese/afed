use std::vec::Vec;
use std::fmt::{Debug, Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish};


#[derive(Clone, Copy)]
pub struct BltnFuncSingle<A> {
    pub name: &'static str,
    ptr: fn(A) -> Object,
}

impl<A: 'static> NamedType for BltnFuncSingle<A> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A> Objectish for BltnFuncSingle<A> where A: Objectish {}

impl<A> BltnFuncSingle<A> where A: Objectish {
    pub fn new(name: &'static str, ptr: fn(A) -> Object) -> Object {
        BltnFuncSingle {name, ptr}.into()
    }
}

impl<A> Operable for BltnFuncSingle<A> where A: Objectish {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    fn arity(&self) -> usize { 1 }
    fn call(&self, mut args: Vec<Object>) -> Self::Output {
        (self.ptr)(try_cast!(args.remove(0)))
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

impl<A, B> Objectish for BltnFuncDouble<A, B> where A: Objectish, B: Objectish {}

impl<A, B> BltnFuncDouble<A, B> where A: Objectish, B: Objectish {
    pub fn new(name: &'static str, ptr: fn(A, B) -> Object) -> Object {
        BltnFuncDouble {name, ptr}.into()
    }
}

impl<A, B> Operable for BltnFuncDouble<A, B> where A: Objectish, B: Objectish {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    fn arity(&self) -> usize { 2 }
    fn call(&self, mut args: Vec<Object>) -> Self::Output {
        let x = try_cast!(args.remove(0));
        let y = try_cast!(args.remove(0));
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

