use std::any::Any;
use std::fmt::{Display, Formatter, Error};

use super::opers::{Unary, Binary};
use super::{Operable, Object, NamedType, Objectish, EvalError, EvalResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curry {
    func: Object,
    arity: usize,
    args: Vec<Object>,
}

impl NamedType for Curry { fn type_name() -> &'static str { "partial evaluation" } }
impl Objectish for Curry { impl_objectish!{} }

impl Curry {
    pub fn new(mut func: Object, mut args: Vec<Object>) -> Object {
        if func.arity() < args.len() { panic!(
                "Cannot curry object, {} arguments given, but expected {}",
                args.len(), func.arity(),
        )}
        
        if let Some(Curry {arity, args: old_args, ..}) = func.downcast_mut::<Curry>() {
            *arity -= args.len();
            old_args.append(&mut args);
            func
        } else {
            let arity = func.arity() - args.len();
            Object::new(Curry {func, arity, args})
        }
    }
}

impl Operable<Object> for Curry {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        Err(unary_not_impl!(op, self))
    }
    
    fn apply_binary(&mut self, op: Binary, _: Object) -> Self::Output {
        Err(binary_not_impl!(op, self))
    }
    
    fn arity(&self) -> usize { self.arity }
    fn apply_call(&self, mut new_args: Vec<Object>) -> Self::Output {
        let mut args = self.args.clone();
        args.append(&mut new_args);
        self.func.apply_call(args)
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

