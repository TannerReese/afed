use std::fmt::{Display, Formatter, Error};

use super::{
    Operable, Object, Castable,
    Unary, Binary,
    NamedType, ErrObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
name_type!{boolean: Bool}

impl Bool {
    pub fn new(b: bool) -> Object { Bool(b).into() }
}

impl Operable for Bool {
    def_unary!{self,
        -self = !self.0,
        !self = !self.0
    }
    def_binary!{self,
        self && other : (Bool => bool) = { self.0 && other },
        self || other : (Bool => bool) = { self.0 || other },

        self + other : (Bool => bool) = { self.0 ^ other },
        self - other : (Bool => bool) = { self.0 ^ other },
        self * other : (Bool => bool) = { self.0 && other }
    }
    def_methods!{}
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl From<Bool> for Object {
    fn from(b: Bool) -> Object { Object::new(b) }
}

impl From<bool> for Object {
    fn from(b: bool) -> Object { Object::new(Bool(b)) }
}

impl Castable for bool {
    fn cast(obj: Object) -> Result<bool, (Object, ErrObject)>
        { Ok(Bool::cast(obj)?.0) }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ternary();
impl NamedType for Ternary { fn type_name() -> &'static str { "ternary" } }

impl Operable for Ternary {
    def_unary!{}
    def_binary!{}

    def_methods!{_, __call(cond: bool, on_true, on_false) =
        if cond { on_true } else { on_false }
    }
}

impl Display for Ternary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> { write!(f, "if") }
}

impl From<Ternary> for Object {
    fn from(t: Ternary) -> Self { Object::new(t) }
}

