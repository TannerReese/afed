use std::vec::Vec;
use std::fmt::{Debug, Display, Formatter, Error};

use crate::object::{Operable, Object, Unary, Binary, NamedType};

#[derive(Clone, Copy)]
pub struct BltnFunc<const N: usize> {
    pub name: &'static str,
    ptr: fn([Object; N]) -> Object,
}

impl<const N: usize> NamedType for BltnFunc<N> {
    fn type_name() -> &'static str { "builtin function" }
}

impl<const N: usize> BltnFunc<N> {
    pub fn new(
        name: &'static str, ptr: fn([Object; N]) -> Object
    ) -> Object { BltnFunc {name, ptr}.into() }
}

impl<const N: usize> Operable for BltnFunc<N> {
    fn unary(self, _: Unary) -> Option<Object> { None }
    fn binary(self,
        _: bool, _: Binary, other: Object
    ) -> Result<Object, (Object, Object)> { Err((self.into(), other)) }

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(N),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, args: Vec<Object>
    ) -> Object { match attr {
        None => (self.ptr)(args.try_into().expect(
            "Incorrect number of arguments given"
        )),
        Some("arity") => N.into(),
        _ => panic!(),
    }}
}

impl<const N: usize> Display for BltnFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<const N: usize> Debug for BltnFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "BltnFunc {{ name: {}, arity: {}, ptr: {} }}",
            self.name, N, self.ptr as usize
        )
    }
}

impl<const N: usize> PartialEq for BltnFunc<N> {
    fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

impl<const N: usize> Eq for BltnFunc<N> {}

impl<const N: usize> From<BltnFunc<N>> for Object {
    fn from(x: BltnFunc<N>) -> Self { Object::new(x) }
}

