use std::fmt::{Debug, Display, Formatter, Error};
use std::iter::zip;

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::object::{
    Object, Unary, Binary, Operable, NamedType,
};
use super::{ExprId, ArgId, ExprArena, Pattern};

#[derive(Debug, Clone)]
pub struct Func {
    name: Option<String>,
    id: usize,
    pats: Vec<Pattern<ArgId>>,
    body: ExprId,
    arena: ExprArena,
}

impl NamedType for Func { fn type_name() -> &'static str{ "function" } }

static FUNC_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Func {
    pub fn new(
        name: Option<String>, pats: Vec<Pattern<ArgId>>,
        body: ExprId, arena: ExprArena,
    ) -> Object {
        let id = FUNC_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(Func {name, id, pats, body, arena})
    }
}

impl Operable for Func {
    fn unary(self, _: Unary) -> Option<Object> { None }
    fn binary(self,
        _: bool, _: Binary, other: Object
    ) -> Result<Object, (Object, Object)> { Err((self.into(), other)) }

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(self.pats.len()),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn help(&self, attr: Option<&str>) -> Option<String> { match attr {
        None => Some(concat!("user-defined function:\n",
            "Lambda or Function defined by user",
            "\n\nMethods:\narity -> usize"
        ).to_owned()),
        Some("arity") => Some(concat!("arity -> usize\n",
            "Number of arguments to function or lambda"
        ).to_owned()),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, args: Vec<Object>
    ) -> Object { match attr {
        None => {
            self.arena.clear_cache();
            for (pat, obj) in zip(self.pats.iter(), args.into_iter()) {
                if let Err(err) = pat.recognize(&self.arena, obj) {
                    return err;
                }
            }
            self.arena.eval(self.body)
        },

        Some("arity") => self.pats.len().into(),
        _ => panic!(),
    }}
}

impl Display for Func {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(name) = &self.name {
            write!(f, "Func<name='{}', id={}, arity={}>",
                name, self.id, self.pats.len(),
            )
        } else { write!(f, "Lambda<id={}, arity={}>",
            self.id, self.pats.len(),
        )}
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl Eq for Func {}

impl From<Func> for Object {
    fn from(f: Func) -> Self { Object::new(f) }
}

