use std::collections::HashMap;
use std::iter::zip;

use super::{ArgId, ExprArena};

use crate::object::{Object, EvalError};
use crate::object::array::Array;
use crate::object::map::Map;

#[derive(Debug, Clone)]
pub enum Pattern<T> {
    Arg(T),
    Array(Vec<Pattern<T>>),
    Map(bool, HashMap<String, Pattern<T>>),
}

impl<A> Pattern<A> {
    pub fn map<B, F>(&self, mut f: F) -> Pattern<B>
    where F: FnMut(&A) -> B { self.map_raw(&mut f) }

    fn map_raw<B, F>(&self, f: &mut F) -> Pattern<B>
    where F: FnMut(&A) -> B { match self {
        Pattern::Arg(x) => Pattern::Arg(f(x)),
        Pattern::Array(pats) => Pattern::Array(
            pats.iter().map(|p| p.map_raw(f)).collect()
        ),
        Pattern::Map(is_fuzzy, pats) => Pattern::Map(*is_fuzzy,
            pats.iter().map(|(key, p)|
                (key.clone(), p.map_raw(f))
            ).collect()
        ),
    }}

    pub fn into_map<B, F>(self, mut f: F) -> Pattern<B>
    where F: FnMut(A) -> B { self.into_map_raw(&mut f) }

    fn into_map_raw<B, F>(self, f: &mut F) -> Pattern<B>
    where F: FnMut(A) -> B { match self {
        Pattern::Arg(x) => Pattern::Arg(f(x)),
        Pattern::Array(pats) => Pattern::Array(
            pats.into_iter().map(|p| p.into_map_raw(f)).collect()
        ),
        Pattern::Map(is_fuzzy, pats) => Pattern::Map(is_fuzzy,
            pats.into_iter().map(|(key, p)|
                (key, p.into_map_raw(f))
            ).collect()
        ),
    }}
}

impl<T: Eq> Pattern<T> {
    pub fn contains(&self, x: &T) -> bool { match self {
        Pattern::Arg(arg) => *x == *arg,
        Pattern::Array(pats) => pats.iter().any(|p| p.contains(x)),
        Pattern::Map(_, pats) => pats.values().any(|p| p.contains(x)),
    }}
}

impl<T: Clone> Pattern<T> {
    pub fn get_args(self) -> Vec<T> {
        let mut args = Vec::new();
        self.collect_args(&mut args);
        args
    }

    fn collect_args(self, args: &mut Vec<T>) { match self {
        Pattern::Arg(x) => args.push(x.clone()),
        Pattern::Array(pats) => for p in pats.into_iter() {
            p.collect_args(args);
        },
        Pattern::Map(_, pats) => for p in pats.into_values() {
            p.collect_args(args);
        },
    }}
}

impl Pattern<ArgId> {
    pub fn recognize(
        &self, arena: &ExprArena, input: Object
    ) -> Result<(), Object> { match self {
        Pattern::Arg(id) => { arena.set_arg(*id, input); Ok(()) },
        Pattern::Array(pats) => {
            let Array(elems) = input.cast()?;
            if elems.len() != pats.len() { return Err(eval_err!(
                "Expected {} elements, but Array has {} elements",
                pats.len(), elems.len(),
            ))}

            for (x, p) in zip(elems, pats.iter()) {
                p.recognize(arena, x)?;
            }
            Ok(())
        },

        Pattern::Map(is_fuzzy, pats) => {
            let Map(mut elems) = input.cast()?;
            for (key, p) in pats.iter() {
                if let Some(x) = elems.remove(key) {
                    p.recognize(arena, x)?;
                } else { return Err(eval_err!(
                    "Map is missing key {}", key,
                ))}
            }

            if !is_fuzzy {
                let keys: Vec<String> = elems.into_keys().collect();
                if keys.len() > 0 { return Err(eval_err!(
                    "Map contains unused keys {:?}", keys,
                ))}
            }
            Ok(())
        },
    }}
}

