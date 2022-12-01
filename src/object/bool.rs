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

impl_operable!{Bool:
    #[unary(Neg)] fn _(own: bool) -> bool { !own }
    #[unary(Not)] fn _(own: bool) -> bool { !own }

    #[binary(And)] fn _(b1: bool, b2: bool) -> bool { b1 && b2 }
    #[binary(Or)] fn _(b1: bool, b2: bool) -> bool { b1 || b2 }

    #[binary(Add)] fn _(b1: bool, b2: bool) -> bool { b1 ^ b2 }
    #[binary(Sub)] fn _(b1: bool, b2: bool) -> bool { b1 ^ b2 }
    #[binary(Mul)] fn _(b1: bool, b2: bool) -> bool { b1 && b2 }
}


impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl From<Bool> for bool {
    fn from(b: Bool) -> bool { b.0 }
}

impl From<Bool> for Object {
    fn from(b: Bool) -> Object { Object::new(b) }
}

impl From<bool> for Bool {
    fn from(b: bool) -> Bool { Bool(b) }
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

impl_operable!{Ternary:
    #[call]
    fn __call(&self,
        cond: bool, on_true: Object, on_false: Object
    ) -> Object { if cond { on_true } else { on_false } }
}

impl Display for Ternary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> { write!(f, "if") }
}

impl From<Ternary> for Object {
    fn from(t: Ternary) -> Self { Object::new(t) }
}

