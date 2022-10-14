use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curry {
    func: Object,
    arity: usize,
    args: Vec<Object>,
}

impl NamedType for Curry { fn type_name() -> &'static str { "partial evaluation" } }

impl Curry {
    pub fn new(func: Object, mut args: Vec<Object>) -> Object {
        if func.arity() < args.len() { panic!(
                "Cannot curry object, {} arguments given, but expected {}",
                args.len(), func.arity(),
        )}
        
        if func.is_a::<Curry>() {
            let mut curry = try_cast!(func => Curry);
            curry.arity -= args.len();
            curry.args.append(&mut args);
            curry
        } else {
            Curry {arity: func.arity() - args.len(), func, args}
        }.into()
    }
}

impl Operable for Curry {
    type Output = Object;
    unary_not_impl!{}
    binary_not_impl!{}
    
    fn arity(&self) -> usize { self.arity }
    fn call(&self, mut new_args: Vec<Object>) -> Object {
        let mut args = self.args.clone();
        args.append(&mut new_args);
        self.func.call(args)
    }
}

impl Display for Curry {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "({})", self.func)?;
        for obj in self.args.iter() {
            write!(f, " ({})", obj)?;
        }
        Ok(())
    }
}

impl From<Curry> for Object {
    fn from(c: Curry) -> Self { Object::new(c) }
}

