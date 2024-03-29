// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::{Debug, Display, Error, Formatter};
use std::iter::zip;

use super::{ArgId, ExprArena, ExprId, Pattern};
use afed_objects::{Binary, NamedType, NamedTypeId, Object, Operable, Unary};

// User defined function who's evaluated using a private AST
#[derive(Debug, Clone)]
pub struct Func {
    // Name of function given by user
    name: Option<String>,
    id: usize, // Unique ID

    // List of patterns matched against arguments
    pats: Vec<Pattern<ArgId>>,
    body: ExprId,     // ID referring to the root node of `arena`
    arena: ExprArena, // Syntax Tree evaluated when calling `Func`
}

impl NamedType for Func {
    fn type_name() -> &'static str {
        "function"
    }
    fn type_id() -> NamedTypeId {
        NamedTypeId::_from_context("Func", Self::type_name(), line!(), column!())
    }
}

// Only used to generate unique identifiers
use std::sync::atomic::{AtomicUsize, Ordering};
static FUNC_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Func {
    pub fn create(
        name: Option<String>,
        pats: Vec<Pattern<ArgId>>,
        body: ExprId,
        arena: ExprArena,
    ) -> Object {
        // Create unique ID for function that is preserved by cloning
        let id = FUNC_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(Func {
            name,
            id,
            pats,
            body,
            arena,
        })
    }
}

impl Operable for Func {
    fn unary(self, _: Unary) -> Option<Object> {
        None
    }
    fn binary(self, _: bool, _: Binary, other: Object) -> Result<Object, (Object, Object)> {
        Err((self.into(), other))
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        match attr {
            None => Some(self.pats.len()),
            Some("arity") => Some(0),
            _ => None,
        }
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        match attr {
            None => Some(
                concat!(
                    "user-defined function:\n",
                    "Lambda or Function defined by user",
                    "\n\nMethods:\narity -> usize"
                )
                .to_owned(),
            ),
            Some("arity") => Some(
                concat!(
                    "arity -> usize\n",
                    "Number of arguments to function or lambda"
                )
                .to_owned(),
            ),
            _ => None,
        }
    }

    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object {
        match attr {
            None => {
                self.arena.clear_cache();
                // Match up given arguments with pattern `self.pats`
                for (pat, obj) in zip(self.pats.iter(), args.into_iter()) {
                    if let Err(err) =
                        pat.match_args(&mut |id: &ArgId, val| self.arena.set_arg(*id, val), obj)
                    {
                        return err;
                    }
                }
                // Evaluate arena same as is done for whole program
                self.arena.eval(self.body)
            }

            Some("arity") => self.pats.len().into(),
            _ => panic!(),
        }
    }
}

impl Display for Func {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(name) = &self.name {
            write!(
                f,
                "Func<name='{}', id={}, arity={}>",
                name,
                self.id,
                self.pats.len(),
            )
        } else {
            write!(f, "Lambda<id={}, arity={}>", self.id, self.pats.len(),)
        }
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Func {}

impl From<Func> for Object {
    fn from(f: Func) -> Self {
        Object::new(f)
    }
}
