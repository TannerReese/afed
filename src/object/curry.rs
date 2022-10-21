use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curry {
    func: Object,
    arity: usize,
    attr: Option<String>,
    args: Vec<Object>,
}

impl NamedType for Curry { fn type_name() -> &'static str { "partial evaluation" } }

impl Curry {
    pub fn new(func: Object, attr: Option<String>, mut args: Vec<Object>) -> Object {
        let attr_ref = attr.as_ref().map(|s| s.as_str());
        let arity = if let Some(x) = func.arity(attr_ref) { x }
        else { panic!( "Cannot call object with attr {:?}", attr)};

        if arity < args.len() { panic!(
            "Cannot curry object, {} arguments given, but expected {} or fewer",
            args.len(), arity,
        )}

        if func.is_a::<Curry>() {
            if let Some(name) = attr { panic!(
                "Curry object has no method {}", name
            )}

            let mut curry = try_cast!(func => Curry);
            curry.arity -= args.len();
            curry.args.append(&mut args);
            curry
        } else { Curry {
            arity: arity - args.len(),
            attr, func, args,
        }}.into()
    }
}

impl Operable for Curry {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(self.arity),
        _ => None,
    }}

    fn call(&self, attr: Option<&str>, mut new_args: Vec<Object>) -> Object {
        if attr.is_some() { panic!() }
        let mut args = self.args.clone();
        args.append(&mut new_args);
        let attr = self.attr.as_ref().map(|s| s.as_str());
        self.func.call(attr, args)
    }
}

impl Display for Curry {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "({})", self.func)?;
        if let Some(method) = &self.attr
            { write!(f, ".{}", method)?; }
        for obj in self.args.iter()
            { write!(f, " ({})", obj)?; }
        Ok(())
    }
}

impl From<Curry> for Object {
    fn from(c: Curry) -> Self { Object::new(c) }
}

