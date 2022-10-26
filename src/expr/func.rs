use std::fmt::{Debug, Display, Formatter, Error};
use std::iter::zip;

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::object::{
    Object, Unary, Binary, Operable, NamedType,
};
use super::{ExprId, ArgId, ExprArena};

#[derive(Debug, Clone)]
pub struct Func {
    name: Option<String>,
    id: usize,
    args: Vec<ArgId>,
    body: ExprId,
    arena: ExprArena,
}

impl NamedType for Func { fn type_name() -> &'static str{ "function" } }

static FUNC_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Func {
    pub fn new(
        name: Option<String>, args: Vec<ArgId>, body: ExprId, arena: ExprArena
    ) -> Object {
        let id = FUNC_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(Func {name, id, args, body, arena})
    }
}

impl Operable for Func {
    unary_not_impl!();
    binary_not_impl!();

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(self.args.len()),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, args: Vec<Object>
    ) -> Object { match attr {
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
        if let Some(name) = &self.name {
            write!(f, "Func<name='{}', id={}, arity={}>",
                name, self.id, self.args.len(),
            )
        } else { write!(f, "Lambda<id={}, arity={}>",
            self.id, self.args.len(),
        )}
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl Eq for Func {}

