use std::fmt::{Display, Error, Formatter};

use super::{Binary, ErrObject, Object, Operable, Unary};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalError {
    pub id: usize,
    pub msg: String,
}
name_type! {error: EvalError}

// Only for generating unique identifiers on EvalError
use std::sync::atomic::AtomicUsize;

static EVAL_ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl EvalError {
    pub fn create(msg: String) -> ErrObject {
        use std::sync::atomic::Ordering;
        let id = EVAL_ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(EvalError { id, msg })
    }
}

impl Operable for EvalError {
    fn unary(self, _: Unary) -> Option<Object> {
        Some(Object::new(self))
    }
    fn binary(self, _: bool, _: Binary, _: Object) -> Result<Object, (Object, Object)> {
        Ok(Object::new(self))
    }

    fn arity(&self, _: Option<&str>) -> Option<usize> {
        Some(0)
    }
    fn help(&self, _: Option<&str>) -> Option<String> {
        Some(self.to_string())
    }
    fn call(&self, _: Option<&str>, _: Vec<Object>) -> Object {
        Object::new(self.clone())
    }
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Eval Error: {}", self.msg)
    }
}

impl<T: Into<Object>> From<Result<T, Object>> for Object {
    fn from(res: Result<T, Object>) -> Self {
        match res {
            Ok(x) => x.into(),
            Err(err) => err,
        }
    }
}

impl<T: Into<Object>> From<Result<T, String>> for Object {
    fn from(res: Result<T, String>) -> Self {
        res.map_err(EvalError::create).into()
    }
}

impl<T: Into<Object>> From<Result<T, &str>> for Object {
    fn from(res: Result<T, &str>) -> Self {
        res.map_err(|s| s.to_owned()).into()
    }
}
