use std::fmt::{Display, Error, Formatter};

use super::{Binary, Castable, ErrObject, NamedType, Object, Operable, Unary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bool(pub bool);
name_type! {bool: Bool}

impl Bool {
    pub fn create(b: bool) -> Object {
        Bool(b).into()
    }
}

impl_operable! {Bool:
    //! Boolean value. Either true or false.
    //! Arithmetic operations behave like a field of order two
    //! where true = 1 (mod 2) and false = 0 (mod 2)

    /// -bool -> bool
    /// Returns same value
    #[unary(Neg)] fn _(own: bool) -> bool { !own }
    /// !bool -> bool
    /// Logical NOT
    #[unary(Not)] fn _(own: bool) -> bool { !own }

    /// bool && bool -> bool
    /// Logical AND
    #[binary(And)] fn _(b1: bool, b2: bool) -> bool { b1 && b2 }
    /// bool || bool -> bool
    /// Logical OR
    #[binary(Or)] fn _(b1: bool, b2: bool) -> bool { b1 || b2 }

    /// bool + bool -> bool
    /// Logical XOR
    #[binary(Add)] fn _(b1: bool, b2: bool) -> bool { b1 ^ b2 }
    /// bool - bool -> bool
    /// Logical XOR
    #[binary(Sub)] fn _(b1: bool, b2: bool) -> bool { b1 ^ b2 }
    /// bool * bool -> bool
    /// Logical AND
    #[binary(Mul)] fn _(b1: bool, b2: bool) -> bool { b1 && b2 }
}

impl Display for Bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl From<Bool> for bool {
    fn from(b: Bool) -> bool {
        b.0
    }
}

impl From<Bool> for Object {
    fn from(b: Bool) -> Object {
        Object::new(b)
    }
}

impl From<bool> for Bool {
    fn from(b: bool) -> Bool {
        Bool(b)
    }
}

impl From<bool> for Object {
    fn from(b: bool) -> Object {
        Object::new(Bool(b))
    }
}

impl Castable for bool {
    fn cast(obj: Object) -> Result<bool, (Object, ErrObject)> {
        Ok(Bool::cast(obj)?.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ternary();
name_type! {ternary: Ternary}

impl_operable! {Ternary:
    //! If statement for deciding between objects

    #[call]
    /// if (cond: bool) (on_true: any) (on_false: any) -> any
    /// Returns 'on_true' when 'cond' is true, otherwise returns 'on_false'
    fn __call(&self,
        cond: bool, on_true: Object, on_false: Object
    ) -> Object { if cond { on_true } else { on_false } }
}

impl Display for Ternary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "if")
    }
}

impl From<Ternary> for Object {
    fn from(t: Ternary) -> Self {
        Object::new(t)
    }
}
