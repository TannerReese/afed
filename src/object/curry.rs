use std::fmt::{Display, Formatter, Error};

use super::{
    Operable, Object,
    Unary, Binary, NamedType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curry {
    func: Object,
    arity: usize,
    attr: Option<String>,
    args: Vec<Object>,
}
name_type!{"partial evaluation": Curry}

impl Curry {
    pub fn new(func: Object, attr: Option<String>, mut args: Vec<Object>) -> Object {
        let attr_ref = attr.as_ref().map(|s| s.as_str());
        let arity = if let Some(x) = func.arity(attr_ref) { x }
        else { panic!( "Cannot call object with attr {:?}", attr)};

        if arity < args.len() { panic!(
            "Cannot curry object, {} arguments given, but expected {} or fewer",
            args.len(), arity,
        )}

        match func.try_cast::<Curry>() {
            Ok(mut curry) => {
                if let Some(name) = attr { panic!(
                    "Curry object has no method {}", name
                )}

                curry.arity -= args.len();
                curry.args.append(&mut args);
                curry
            },
            Err(func) => Curry {
                arity: arity - args.len(),
                attr, func, args,
            },
        }.into()
    }
}

impl Operable for Curry {
    fn unary(self, _: Unary) -> Option<Object> { None }
    fn binary(self,
        _: bool, _: Binary, other: Object
    ) -> Result<Object, (Object, Object)> { Err((self.into(), other)) }


    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        None => Some(self.arity),
        Some("arity") => Some(0),
        _ => None,
    }}

    fn help(&self, attr: Option<&str>) -> Option<String> { match attr {
        None => Some(concat!("Partial Evaluation:\n",
            "Function or Method with some of the arguments already given",
            "\n\nMethods:\narity -> usize"
        ).to_owned()),
        Some("arity") => Some(concat!("arity -> usize\n",
            "Number of further arguments that need to be provided\n",
            "before the function or method can be fully evaluated."
        ).to_owned()),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, mut new_args: Vec<Object>
    ) -> Object { match attr {
        None => {
            let mut args = self.args.clone();
            args.append(&mut new_args);
            let attr = self.attr.as_ref().map(|s| s.as_str());
            self.func.call(attr, args)
        },

        Some("arity") => self.arity.into(),
        _ => panic!(),
    }}
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

