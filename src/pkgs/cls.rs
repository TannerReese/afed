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

use afed_objects::{
    call, declare_pkg, eval_err, name_type, Binary, Castable, Object, Operable, Unary,
};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Error, Formatter};
use std::rc::Rc;

// HashMap of methods used by class instance
#[derive(Clone, Debug, Eq)]
struct Class {
    name: Option<String>,
    id: usize,
    constructor: Option<Object>,
    methods: HashMap<String, Object>,
}

// Only used for generating unique IDs for Classes
use std::sync::atomic::AtomicUsize;
static CLASS_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Class {
    fn new(mut cls: HashMap<String, Object>) -> Result<Self, &'static str> {
        let constructor = cls.remove("new");
        let name = if let Some(name) = cls.remove("clsname") {
            Some(name.cast().map_err(|_| "Class name must be a string")?)
        } else {
            None
        };

        use std::sync::atomic::Ordering;
        let id = CLASS_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(Class {
            name,
            id,
            constructor,
            methods: cls,
        })
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// Wrapper around `constructor` that converts the result into ClassInst
#[derive(Clone, Debug, PartialEq, Eq)]
struct ClassObj(Rc<Class>);
name_type! {"class": ClassObj}

impl Display for ClassObj {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(name) = &self.0.name {
            write!(f, "Class {}", name)
        } else {
            write!(f, "Class<id={}>", self.0.id)
        }
    }
}

impl Operable for ClassObj {
    fn unary(self, _: Unary) -> Option<Object> {
        None
    }

    fn binary(self, _: bool, _: Binary, other: Object) -> Result<Object, (Object, Object)> {
        Err((self.into(), other))
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        match attr {
            None => {
                if let Some(constr) = &self.0.constructor {
                    usize::cast(call!(constr.arity)).ok()
                } else {
                    Some(1)
                }
            }
            Some("clsname") => self.0.name.as_ref().map(|_| 0),
            Some(name) => self
                .0
                .methods
                .get(name)
                .map(|method| usize::cast(call!(method.arity)).unwrap_or(0)),
        }
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        if let Some(attr) = attr {
            if self.0.methods.contains_key(attr) {
                if let Some(name) = &self.0.name {
                    Some(format!("method for {}", name))
                } else {
                    Some("method for a class".into())
                }
            } else {
                None
            }
        } else {
            let mut msg = String::from("class");
            if let Some(name) = &self.0.name {
                msg = msg + " " + name;
            }

            msg += ":\n Class defined by user\n\nMethods & Operators:";
            if self.0.name.is_some() {
                msg += " clsname";
            }
            for method in self.0.methods.keys() {
                msg = msg + " " + method;
            }
            Some(msg)
        }
    }

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Object {
        if let Some(attr) = attr {
            if attr == "clsname" {
                if let Some(name) = &self.0.name {
                    return name.clone().into();
                }
            }

            if let Some(method) = self.0.methods.get(attr) {
                method.call(None, args)
            } else {
                eval_err!("No method {} present", attr)
            }
        } else {
            let data = if let Some(constr) = &self.0.constructor {
                constr.call(None, args)
            } else {
                args.pop().unwrap()
            };
            Object::new(ClassInst {
                class: Rc::clone(&self.0),
                data,
            })
        }
    }
}

#[derive(Clone, Debug, Eq)]
struct ClassInst {
    class: Rc<Class>,
    data: Object,
}
name_type! {"class instance": ClassInst}

impl PartialEq for ClassInst {
    fn eq(&self, other: &Self) -> bool {
        if self.class != other.class {
            return false;
        }

        if let Some(equals) = self.class.methods.get("__eq") {
            let self_copy = Object::new(self.clone());
            let other_copy = Object::new(other.clone());
            let res = call!(equals(self_copy, other_copy));
            res.cast().unwrap_or(false)
        } else {
            self.data == other.data
        }
    }
}

impl Display for ClassInst {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(to_str) = self.class.methods.get("__str") {
            let copy = Object::new(self.clone());
            if let Ok(s) = String::cast(call!(to_str(copy))) {
                return write!(f, "{}", s);
            }
        }

        if let Some(name) = &self.class.name {
            write!(f, "Instance<{}> {}", name, self.data)
        } else {
            write!(f, "Instance<{}> {}", self.class.id, self.data)
        }
    }
}

impl Operable for ClassInst {
    fn unary(self, op: Unary) -> Option<Object> {
        // WARNING: Do not place _ case in below match.
        // This ensures that this match is kept up to.
        let name = match op {
            Unary::Neg => Some("__neg"),
            Unary::Not => Some("__not"),
        };

        // Copy class reference so that self can be passed to the call
        let class = Rc::clone(&self.class);
        name.and_then(|name| class.methods.get(name))
            .map(|method| call!(method(Object::new(self))))
    }

    fn binary(self, rev: bool, op: Binary, other: Object) -> Result<Object, (Object, Object)> {
        // WARNING: Do not place _ case in below match.
        // This ensures that this match is kept up to.
        let name = match (op, rev) {
            (Binary::And, false) => Some("__and"),
            (Binary::And, true) => Some("__rand"),
            (Binary::Or, false) => Some("__or"),
            (Binary::Or, true) => Some("__ror"),
            (Binary::Leq, false) => Some("__leq"),
            (Binary::Leq, true) => None,

            (Binary::Add, false) => Some("__add"),
            (Binary::Add, true) => Some("__radd"),
            (Binary::Sub, false) => Some("__sub"),
            (Binary::Sub, true) => Some("__rsub"),
            (Binary::Mul, false) => Some("__mul"),
            (Binary::Mul, true) => Some("__rmul"),
            (Binary::Div, false) => Some("__div"),
            (Binary::Div, true) => Some("__rdiv"),
            (Binary::Mod, false) => Some("__mod"),
            (Binary::Mod, true) => Some("__rmod"),
            (Binary::FlrDiv, false) => Some("__flrdiv"),
            (Binary::FlrDiv, true) => Some("__rflrdiv"),
            (Binary::Pow, false) => Some("__pow"),
            (Binary::Pow, true) => Some("__rpow"),

            (Binary::Apply, _)
            | (Binary::Eq, _)
            | (Binary::Neq, _)
            | (Binary::Lt, _)
            | (Binary::Gt, _)
            | (Binary::Geq, _) => None,
        };

        // Copy class reference so that self can be passed to the call
        let class = Rc::clone(&self.class);
        if let Some(method) = name.and_then(|name| class.methods.get(name)) {
            Ok(call!(method(Object::new(self), other)))
        } else {
            Err((Object::new(self), other))
        }
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        let name = attr.unwrap_or("__call");
        if name == "new" {
            // Static access to this instances constructor
            if let Some(constr) = &self.class.constructor {
                usize::cast(call!(constr.arity)).ok()
            } else {
                Some(1)
            }
        } else if name == "__data" {
            Some(0)
        } else if let Some(method) = self.class.methods.get(name) {
            let count = usize::cast(call!(method.arity)).ok()?;
            if count > 0 {
                Some(count - 1)
            } else {
                None
            }
        } else {
            self.data.arity(attr)
        }
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        let name = if let Some(name) = &self.class.name {
            name.as_str()
        } else {
            "a class"
        };

        if attr == Some("new") {
            // Static access to this instances constructor
            Some(format!("constructor for {}", name))
        } else if attr == Some("__data") {
            Some(
                concat!(
                    "inst.__data -> any\n",
                    " Internal object containing the data of the instance"
                )
                .into(),
            )
        } else if let Some(attr) = attr {
            if self.class.methods.contains_key(attr) {
                Some(format!("method for {}", name))
            } else {
                self.data.help(Some(attr))
            }
        } else {
            let mut msg = format!("instance of {}:\n", name);
            msg += " Instance created by the class constructor\n\n";
            msg += "Methods & Operators:";
            for method in self.class.methods.keys() {
                msg = msg + " " + method;
            }
            Some(msg)
        }
    }

    fn call(&self, attr: Option<&str>, mut args: Vec<Object>) -> Object {
        let name = attr.unwrap_or("__call");
        if name == "new" {
            // Static access to this instances constructor
            let data = if let Some(constr) = &self.class.constructor {
                constr.call(None, args)
            } else {
                args.pop().unwrap()
            };
            Object::new(Self {
                class: Rc::clone(&self.class),
                data,
            })
        } else if name == "__data" {
            self.data.clone()
        } else if let Some(method) = self.class.methods.get(name) {
            args.insert(0, Object::new(self.clone()));
            method.call(None, args)
        } else {
            self.data.call(attr, args)
        }
    }
}

impl From<ClassObj> for Object {
    fn from(cls: ClassObj) -> Self {
        Object::new(cls)
    }
}

impl From<ClassInst> for Object {
    fn from(inst: ClassInst) -> Self {
        Object::new(inst)
    }
}

declare_pkg! {cls: #![bltn_pkg]
    #[allow(non_snake_case)]
    fn Class(cls: HashMap<String, Object>) -> Result<ClassObj, &'static str> {
        Ok(ClassObj(Rc::new(Class::new(cls)?)))
    }
}
