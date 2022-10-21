use std::fmt::{Debug, Display, Formatter, Error};
use std::iter::zip;

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::object::{
    Object, Operable, NamedType,
    opers::{Unary, Binary}
};
use super::{ExprId, ArgId, ExprArena};

#[derive(Debug, Clone)]
pub struct Func {
    name: String,
    id: usize,
    args: Vec<ArgId>,
    body: ExprId,
    arena: ExprArena,
}

impl NamedType for Func { fn type_name() -> &'static str{ "function" } }

static FUNC_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Func {
    pub fn new(name: String, args: Vec<ArgId>, body: ExprId, arena: ExprArena) -> Object {
        let id = FUNC_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(Func {name, id, args, body, arena})
    }
}

impl Operable for Func {
    type Output = Object;
    unary_not_impl!();
    binary_not_impl!();

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(self.args.len()),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, args: Vec<Object>
    ) -> Self::Output { match attr {
        None => {
            self.arena.clear_cache();
            for (&id, obj) in zip(self.args.iter(), args.into_iter()) {
                self.arena.set_arg(id, obj);
            }
            self.arena.eval(self.body)
        },

        Some("arity") => self.args.len().into(),
        _ => panic!(),
    }}
}

impl Display for Func {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Func<name='{}', id={}, arity={}>",
            self.name, self.id, self.args.len(),
        )
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl Eq for Func {}

