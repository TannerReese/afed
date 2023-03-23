use std::collections::HashMap;
use std::fmt::{Debug, Display, Error, Formatter};

use super::{Binary, NamedType, NamedTypeId, Object, Operable, Unary};

/* A tree of packages that can be converted to objects and added to the arena.
 * The bool in the entries of `Pkg::Map` represents whether that entry
 * should be treated as a global when the `Pkg` is added to an `ExprArena`.
 */
pub enum Pkg {
    Const(Object),
    Map(HashMap<String, (bool, Pkg)>),
}

// Wrapper for Rust functions so they can be used in Afed
#[derive(Clone, Copy)]
pub struct PkgFunc<const N: usize> {
    pub name: &'static str,
    pub help: &'static str,
    ptr: fn([Object; N]) -> Object,
}

impl<const N: usize> NamedType for PkgFunc<N> {
    fn type_name() -> &'static str {
        "builtin function"
    }

    fn type_id() -> NamedTypeId {
        NamedTypeId::_from_context(
            "PkgFunc",
            Self::type_name(),
            (line!() << 4) + N as u32,
            column!(),
        )
    }
}

impl<const N: usize> PkgFunc<N> {
    pub fn create(
        name: &'static str,
        help: &'static str,
        ptr: fn([Object; N]) -> Object,
    ) -> Object {
        PkgFunc { name, help, ptr }.into()
    }
}

impl<const N: usize> Operable for PkgFunc<N> {
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

impl<const N: usize> Display for PkgFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.name)
    }
}

impl<const N: usize> Debug for PkgFunc<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "PkgFunc {{ name: {}, arity: {}, ptr: {} }}",
            self.name, N, self.ptr as usize
        )
    }
}

impl<const N: usize> PartialEq for PkgFunc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<const N: usize> Eq for PkgFunc<N> {}

impl<const N: usize> From<PkgFunc<N>> for Object {
    fn from(x: PkgFunc<N>) -> Self {
        Object::new(x)
    }
}
