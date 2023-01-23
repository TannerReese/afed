use std::collections::HashMap;
use std::fmt::{Debug, Display, Error, Formatter};

use super::{Binary, NamedType, Object, Operable, Unary};

/* A tree of packages that can be converted to objects and added to the arena.
 * The bool in the entries of `Bltn::Map` represents whether that entry
 * should be treated as a global when the `Bltn` is added to an `ExprArena`.
 */
pub enum Bltn {
    Const(Object),
    Map(HashMap<String, (bool, Bltn)>),
}

// Wrapper for Rust functions so they can be used in Afed
#[derive(Clone, Copy)]
pub struct BltnFunc<const N: usize> {
    pub name: &'static str,
    pub help: &'static str,
    ptr: fn([Object; N]) -> Object,
}

impl<const N: usize> NamedType for BltnFunc<N> {
    fn type_name() -> &'static str {
        "builtin function"
    }
}

impl<const N: usize> BltnFunc<N> {
    pub fn create(
        name: &'static str,
        help: &'static str,
        ptr: fn([Object; N]) -> Object,
    ) -> Object {
        BltnFunc { name, help, ptr }.into()
    }
}

impl<const N: usize> Operable for BltnFunc<N> {
    fn unary(self, _: Unary) -> Option<Object> {
        None
    }
    fn binary(self, _: bool, _: Binary, other: Object) -> Result<Object, (Object, Object)> {
        Err((self.into(), other))
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        match attr {
            None => Some(N),
            Some("arity") => Some(0),
            _ => None,
        }
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        match attr {
            None => Some(self.help.to_owned()),
            Some("arity") => Some(
                concat!(
                    "arity -> usize\n",
                    "Number of arguments to builtin function"
                )
                .to_owned(),
            ),
            _ => None,
        }
    }

    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object {
        match attr {
            None => (self.ptr)(
                args.try_into()
                    .expect("Incorrect number of arguments given"),
            ),
            Some("arity") => N.into(),
            _ => panic!(),
        }
    }
}

impl<const N: usize> Display for BltnFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<const N: usize> Debug for BltnFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "BltnFunc {{ name: {}, arity: {}, ptr: {} }}",
            self.name, N, self.ptr as usize
        )
    }
}

impl<const N: usize> PartialEq for BltnFunc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<const N: usize> Eq for BltnFunc<N> {}

impl<const N: usize> From<BltnFunc<N>> for Object {
    fn from(x: BltnFunc<N>) -> Self {
        Object::new(x)
    }
}
