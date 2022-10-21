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

impl<A: Objectish> BltnFuncSingle<A> {
    pub fn new(name: &'static str, ptr: fn(A) -> Object) -> Object {
        BltnFuncSingle {name, ptr}.into()
    }
}

impl<A: Objectish> Operable for BltnFuncSingle<A> {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(1),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Self::Output { match attr {
        None => (self.ptr)(try_cast!(args.remove(0))),
        Some("arity") => (1 as i64).into(),
        _ => panic!(),
    }}
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

impl<A: Objectish> From<BltnFuncSingle<A>> for Object {
    fn from(x: BltnFuncSingle<A>) -> Self { Object::new(x) }
}





#[derive(Clone, Copy)]
pub struct BltnFuncDouble<A, B> {
    pub name: &'static str,
    ptr: fn(A, B) -> Object,
}

impl<A: 'static, B: 'static> NamedType for BltnFuncDouble<A, B> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<A: Objectish, B: Objectish> BltnFuncDouble<A, B> {
    pub fn new(name: &'static str, ptr: fn(A, B) -> Object) -> Object {
        BltnFuncDouble {name, ptr}.into()
    }
}

impl<A: Objectish, B: Objectish> Operable for BltnFuncDouble<A, B> {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(2),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, mut args: Vec<Object>
    ) -> Self::Output { match attr {
        None => {
            let x = try_cast!(args.remove(0));
            let y = try_cast!(args.remove(0));
            (self.ptr)(x, y)
        },

        Some("arity") => (2 as i64).into(),
        _ => panic!(),
    }}
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

impl<A: Objectish, B: Objectish> From<BltnFuncDouble<A, B>> for Object {
    fn from(x: BltnFuncDouble<A, B>) -> Self { Object::new(x) }
}

