use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::zip;

use afed_objects::{array::Array, eval_err, map::Map, ErrObject, Object};

// Tree of arguments used for destructuring calls
#[derive(Debug, Clone)]
pub enum Pattern<T> {
    Ignore, // Match anything and ignore it
    Arg(T),
    Array(Vec<Pattern<T>>),                 // Destructure Array
    Map(bool, HashMap<String, Pattern<T>>), // Destructure Map
}

// Apply functions across the entire tree of argument locations
impl<A> Pattern<A> {
    pub fn map<B, F>(&self, mut f: F) -> Pattern<B>
    where
        F: FnMut(&A) -> B,
    {
        self.map_raw(&mut f)
    }

    fn map_raw<B, F>(&self, f: &mut F) -> Pattern<B>
    where
        F: FnMut(&A) -> B,
    {
        match self {
            Pattern::Ignore => Pattern::Ignore,
            Pattern::Arg(x) => Pattern::Arg(f(x)),
            Pattern::Array(pats) => Pattern::Array(pats.iter().map(|p| p.map_raw(f)).collect()),
            Pattern::Map(is_fuzzy, pats) => Pattern::Map(
                *is_fuzzy,
                pats.iter()
                    .map(|(key, p)| (key.clone(), p.map_raw(f)))
                    .collect(),
            ),
        }
    }

    pub fn into_map<B, F>(self, mut f: F) -> Pattern<B>
    where
        F: FnMut(A) -> B,
    {
        self.into_map_raw(&mut f)
    }

    fn into_map_raw<B, F>(self, f: &mut F) -> Pattern<B>
    where
        F: FnMut(A) -> B,
    {
        match self {
            Pattern::Ignore => Pattern::Ignore,
            Pattern::Arg(x) => Pattern::Arg(f(x)),
            Pattern::Array(pats) => {
                Pattern::Array(pats.into_iter().map(|p| p.into_map_raw(f)).collect())
            }
            Pattern::Map(is_fuzzy, pats) => Pattern::Map(
                is_fuzzy,
                pats.into_iter()
                    .map(|(key, p)| (key, p.into_map_raw(f)))
                    .collect(),
            ),
        }
    }
}

impl<T: Eq> Pattern<T> {
    // Check if a list of `Pattern`s have any repeated arguments
    pub fn has_duplicate_args(pats: &[Self]) -> Option<&T> {
        let mut args = Vec::new();
        pats.iter()
            .filter_map(|p| p.has_duplicate_args_raw(&mut args))
            .next()
    }

    fn has_duplicate_args_raw<'a>(&'a self, args: &mut Vec<&'a T>) -> Option<&'a T> {
        match self {
            Pattern::Ignore => None,
            Pattern::Arg(x) => {
                if args.iter().any(|a| *x == **a) {
                    Some(x)
                } else {
                    args.push(x);
                    None
                }
            }
            Pattern::Array(pats) => pats
                .iter()
                .filter_map(|p| p.has_duplicate_args_raw(args))
                .next(),
            Pattern::Map(_, pats) => pats
                .values()
                .filter_map(|p| p.has_duplicate_args_raw(args))
                .next(),
        }
    }
}

impl<T: Eq + Hash + Clone> Pattern<T> {
    pub fn contains(&self, x: &T) -> bool {
        match self {
            Pattern::Ignore => false,
            Pattern::Arg(arg) => *x == *arg,
            Pattern::Array(pats) => pats.iter().any(|p| p.contains(x)),
            Pattern::Map(_, pats) => pats.values().any(|p| p.contains(x)),
        }
    }

    pub fn arg_set(&self) -> HashSet<T> {
        let mut set = HashSet::new();
        self.arg_set_raw(&mut set);
        set
    }

    fn arg_set_raw(&self, set: &mut HashSet<T>) {
        match self {
            Pattern::Ignore => {}
            Pattern::Arg(arg) => {
                set.insert(arg.clone());
            }
            Pattern::Array(pats) => pats.iter().for_each(|p| p.arg_set_raw(set)),
            Pattern::Map(_, pats) => pats.values().for_each(|p| p.arg_set_raw(set)),
        }
    }
}

impl<I> Pattern<I> {
    pub fn match_args<F>(&self, setter: &mut F, input: Object) -> Result<(), ErrObject>
    where
        F: FnMut(&I, Object),
    {
        match self {
            Pattern::Ignore => Ok(()),
            Pattern::Arg(id) => {
                setter(id, input);
                Ok(())
            }
            // Try to destructure `input` as an Array of arguments
            Pattern::Array(pats) => {
                let Array(elems) = input.cast()?;
                if elems.len() != pats.len() {
                    return Err(eval_err!(
                        "Expected {} elements, but Array has {} elements",
                        pats.len(),
                        elems.len(),
                    ));
                }

                for (x, p) in zip(elems, pats.iter()) {
                    p.match_args(setter, x)?;
                }
                Ok(())
            }

            // Try to destructure `input` as a Map of arguments
            Pattern::Map(is_fuzzy, pats) => {
                let Map(mut elems) = input.cast()?;
                for (key, p) in pats.iter() {
                    if let Some(x) = elems.remove(key) {
                        p.match_args(setter, x)?;
                    } else {
                        return Err(eval_err!("Map is missing key {}", key,));
                    }
                }

                // Fuzziness allows the map to contain other keys
                if !is_fuzzy {
                    let keys: Vec<String> = elems.into_keys().collect();
                    if !keys.is_empty() {
                        return Err(eval_err!("Map contains unused keys {:?}", keys,));
                    }
                }
                Ok(())
            }
        }
    }
}
