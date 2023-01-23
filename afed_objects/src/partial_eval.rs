use std::fmt::{Display, Error, Formatter};

use super::{Binary, Object, Operable, Unary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialEval {
    func: Object,
    arity: usize,
    attr: Option<String>,
    args: Vec<Object>,
}
name_type! {"partial evaluation": PartialEval}

impl PartialEval {
    pub fn create(func: Object, attr: Option<String>, mut args: Vec<Object>) -> Object {
        let attr_ref = attr.as_deref();
        let arity = if let Some(x) = func.arity(attr_ref) {
            x
        } else {
            panic!("Cannot call object with attr {:?}", attr)
        };

        if arity < args.len() {
            panic!(
                "Cannot curry object, {} arguments given, but expected {} or fewer",
                args.len(),
                arity,
            )
        }

        match func.try_cast::<PartialEval>() {
            Ok(mut curry) => {
                if let Some(name) = attr {
                    panic!("PartialEval object has no method {}", name)
                }

                curry.arity -= args.len();
                curry.args.append(&mut args);
                curry
            }
            Err(func) => PartialEval {
                arity: arity - args.len(),
                attr,
                func,
                args,
            },
        }
        .into()
    }
}

impl Operable for PartialEval {
    fn unary(self, _: Unary) -> Option<Object> {
        None
    }
    fn binary(self, _: bool, _: Binary, other: Object) -> Result<Object, (Object, Object)> {
        Err((self.into(), other))
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        match attr {
            None => Some(self.arity),
            Some("arity") => Some(0),
            _ => None,
        }
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        match attr {
            None => Some(
                concat!(
                    "Partial Evaluation:\n",
                    "Function or Method with some of the arguments already given",
                    "\n\nMethods:\narity -> usize"
                )
                .to_owned(),
            ),
            Some("arity") => Some(
                concat!(
                    "arity -> usize\n",
                    "Number of further arguments that need to be provided\n",
                    "before the function or method can be fully evaluated."
                )
                .to_owned(),
            ),
            _ => None,
        }
    }

    fn call(&self, attr: Option<&str>, mut new_args: Vec<Object>) -> Object {
        match attr {
            None => {
                let mut args = self.args.clone();
                args.append(&mut new_args);
                let attr = self.attr.as_deref();
                self.func.call(attr, args)
            }

            Some("arity") => self.arity.into(),
            _ => panic!(),
        }
    }
}

impl Display for PartialEval {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "({})", self.func)?;
        if let Some(method) = &self.attr {
            write!(f, ".{}", method)?;
        }
        for obj in self.args.iter() {
            write!(f, " ({})", obj)?;
        }
        Ok(())
    }
}

impl From<PartialEval> for Object {
    fn from(c: PartialEval) -> Self {
        Object::new(c)
    }
}
