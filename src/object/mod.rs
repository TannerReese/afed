use std::any::{Any, TypeId};
use std::vec::Vec;
use std::clone::Clone;
use std::fmt::{Debug, Display, Formatter, Error, Write};

use std::cmp::Ordering;
use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::iter::{Sum, Product};

use std::sync::atomic::AtomicUsize;

use std::hash::Hash;
use std::borrow::Borrow;

pub use opers::{Unary, Binary, Assoc};
use self::bool::Bool;

macro_rules! try_ok {
    ($obj:expr) => {{
        let obj = $obj;
        if obj.is_err() { return obj } else { obj }
    }};
}



mod opers;

pub mod null;
pub mod bool;
pub mod number;
pub mod string;
pub mod array;
pub mod map;
pub mod curry;


pub trait Operable {
    fn unary(self, op: Unary) -> Option<Object>;
    fn binary(
        self, rev: bool, op: Binary, other: Object
    ) -> Result<Object, (Object, Object)>;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
    fn help(&self, attr: Option<&str>) -> Option<String>;
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object;
}

pub trait NamedType : Any {
    fn type_name() -> &'static str;
}

pub trait Objectish :
    Eq + Clone + Any + NamedType
    + Debug + Display + Operable
{}

impl<T> Objectish for T where T:
    Eq + Clone + Any + NamedType
    + Debug + Display + Operable
{}

fn as_any<T>(x: &T) -> &dyn Any where T: Objectish { x }

trait ObjectishSafe {
    fn as_any(&self) -> &dyn Any;
    fn as_any_opt(&mut self) -> &mut dyn Any;
    fn type_name_dyn(&self) -> &'static str;

    fn clone(&self) -> Object;
    fn eq(&self, other: &Object) -> bool;
    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;

    fn unary(&mut self, op: Unary) -> Option<Object>;
    fn binary(&mut self,
        rev: bool, op: Binary, other: Object
    ) -> Result<Object, (Object, Object)>;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
    fn help(&self, attr: Option<&str>) -> Option<String>;
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object;
}

fn to_obj<T>(obj: &mut Option<T>) -> T { std::mem::take(obj).unwrap() }
fn to_ref<T>(obj: &Option<T>) -> &T { obj.as_ref().unwrap() }

impl<T> ObjectishSafe for Option<T> where T: Objectish + 'static {
    fn as_any(&self) -> &dyn Any { as_any(to_ref(self)) }
    fn as_any_opt(&mut self) -> &mut dyn Any { self }
    fn type_name_dyn(&self) -> &'static str { T::type_name() }

    fn clone(&self) -> Object { Object::new(to_ref(self).clone()) }
    fn eq(&self, other: &Object) -> bool {
        if let Some(other) = other.cast_ref::<T>() {
            to_ref(self) == other
        } else { false }
    }

    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { Display::fmt(to_ref(self), f) }
    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { Debug::fmt(to_ref(self), f) }

    fn unary(&mut self, op: Unary) -> Option<Object>
        { to_obj(self).unary(op) }
    fn binary(&mut self,
        rev: bool, op: Binary, other: Object
    ) -> Result<Object, (Object, Object)>
        { to_obj(self).binary(rev, op, other) }

    fn arity(&self, attr: Option<&str>) -> Option<usize>
        { to_ref(self).arity(attr) }
    fn help(&self, attr: Option<&str>) -> Option<String>
        { to_ref(self).help(attr) }
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object
        { to_ref(self).call(attr, args) }
}

pub struct Object(Box<dyn ObjectishSafe>);
pub type ErrObject = Object;

impl Object {
    pub fn new<T>(obj: T) -> Object where T: Objectish {
        Object(Box::new(Some(obj)))
    }

    pub fn is_err(&self) -> bool {self.is_a::<EvalError>() }
    pub fn ok_or_err(self) -> Result<Object, ErrObject>
        { if self.is_err() { Err(self) } else { Ok(self) } }
    pub fn type_id(&self) -> TypeId { (*self.0).as_any().type_id() }
    pub fn is_a<T>(&self) -> bool where T: Any
        { TypeId::of::<T>() == self.type_id() }

    pub fn cast_ref<'a, T: 'static>(&'a self) -> Option<&'a T>
        { (*self.0).as_any().downcast_ref() }


    pub fn do_inside(&mut self, func: impl FnOnce(Self) -> Self){
        let owned = Object(std::mem::replace(&mut self.0,
            Box::new(None::<null::Null>)
        ));
        self.0 = func(owned).0;
    }

    pub fn cast<T: Castable>(self) -> Result<T, ErrObject>
        { T::cast(self).map_err(|(_, err)| err) }
    pub fn try_cast<T: Castable>(self) -> Result<T, Object>
        { T::cast(self).map_err(|(obj, _)| obj) }
    pub fn cast_with_err<T: Castable>(self)
        -> Result<T, (Object, ErrObject)> { T::cast(self) }

    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    { self.cast_ref::<map::Map>().and_then(|m| m.get(key)) }

    pub fn find<'a, I, B>(&self, path: I) -> Option<&Object>
    where
        I: Iterator<Item = &'a B>,
        B: Hash + Eq + 'a,
        String: Borrow<B>,
    {
        let mut target = self;
        for nm in path {
            if let Some(new_target) = target.get(nm) {
                target = new_target;
            } else { return None; }
        }
        return Some(target);
    }
}


// Don't try to replace with TryFrom<Object>
pub trait Castable: Sized {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>;
}

impl Castable for Object {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> { Ok(obj) }
}

impl<T> Castable for T where T: Objectish {
    fn cast(mut obj: Object) -> Result<T, (Object, ErrObject)> {
        let given_type = (*obj.0).type_name_dyn();
        let any_ref = (*obj.0).as_any_opt();
        if let Some(opt) = any_ref.downcast_mut::<Option<T>>() {
            Ok(std::mem::take(opt).unwrap())
        } else { Err((obj, eval_err!(
            "Expected {}, but found {}", T::type_name(), given_type
        )))}
    }
}



impl Object {
    pub fn unary(mut self, op: Unary) -> Object {
        if let Some(obj) = (*self.0).unary(op) { obj }
        else { eval_err!(
            "Unary operator {} not implemented for type {}",
            op.symbol(), (*self.0).type_name_dyn(),
        )}
    }

    fn binary_raw(mut self, op: Binary, other: Object) -> Object {
        let (self_, mut other) = match (*self.0).binary(false, op, other) {
            Ok(result) => return result,
            Err(args) => args,
        };

        let (other, self_) = match (*other.0).binary(true, op, self_) {
            Ok(result) => return result,
            Err(args) => args,
        };

        eval_err!(
            "Binary operator {} not implemented between {} and {}",
            op.symbol(), self_, other,
        )
    }

    pub fn binary(self, op: Binary, other: Object) -> Object {
    match op {
        Binary::Apply => self.call(None, vec![other]),
        Binary::Eq => Bool::new(self == other),
        Binary::Neq => Bool::new(self != other),
        Binary::Leq => self.binary_raw(Binary::Leq, other),
        Binary::Geq => other.binary_raw(Binary::Leq, self),
        Binary::Lt => if self == other { Bool::new(false) } else {
            self.binary_raw(Binary::Leq, other)
        },
        Binary::Gt => if self == other { Bool::new(false) } else {
            other.binary_raw(Binary::Leq, self)
        },
        _ => self.binary_raw(op, other),
    }}

    pub fn arity(&self, attr: Option<&str>) -> Option<usize>
        { (*self.0).arity(attr) }
    pub fn help(&self, attr: Option<&str>) -> Option<String>
        { (*self.0).help(attr) }

    pub fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object {
        let arity: usize;
        if let Some(x) = (*self.0).arity(attr) {
            arity = x;
        } else if let Some(method) = attr { return eval_err!(
            "Cannot call method {} on type {}", method, (*self.0).type_name_dyn()
        )} else { return eval_err!(
            "Cannot call type {}", (*self.0).type_name_dyn(),
        )}

        if args.len() == arity {
            return (*self.0).call(attr, args);
        } else if args.len() < arity {
            let attr = attr.map(|s| s.to_owned());
            return curry::Curry::new(self.clone(), attr, args);
        }

        let mut args = args.into_iter();
        let iter = args.by_ref();

        let mut res = try_ok!((*self.0).call(attr, iter.take(arity).collect()));
        while iter.as_slice().len() > 0 {
            let arity;
            if let Some(x) = (*res.0).arity(None) {
                arity = x;
            } else { return eval_err!(
                "Cannot call type {}", (*res.0).type_name_dyn(),
            )}

            res = if iter.as_slice().len() >= arity {
                (*res.0).call(None, iter.take(arity).collect())
            } else { curry::Curry::new(res.clone(), None, iter.collect()) };
        }
        res
    }

    fn get_attr(&self, attr: &str) -> Object {
        if let Some(arity) = self.arity(Some(attr)) {
            if arity > 0 { eval_err!(
                "Method {} has arity {}, but wasn't given any arguments",
                attr, arity,
            )} else { self.call(Some(attr), Vec::with_capacity(0)) }
        } else { eval_err!(
            "Object of type {} has no method {}",
            (*self.0).type_name_dyn(), attr,
        )}
    }

    pub fn call_path(&self, mut path: Vec<&str>, args: Vec<Object>) -> Object {
        let last = path.pop();
        if path.len() > 0 {
            let mut obj = try_ok!(self.get_attr(path.remove(0)));
            for key in path.into_iter() { obj = try_ok!(obj.get_attr(key)); }
            obj.call(last, args)
        } else { self.call(last, args) }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("Object(")?;
        (*self.0).debug_fmt(f)?;
        f.write_char(')')
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { (*self.0).display_fmt(f) }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool { (*self.0).eq(other) }
}

impl Eq for Object {}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { return Some(Ordering::Equal) }
        let leq = self.clone().binary(Binary::Leq, other.clone());
        match leq.cast() {
            Ok(true) => Some(Ordering::Less),
            Ok(false) => Some(Ordering::Greater),
            Err(_) => None,
        }
    }
}

impl Clone for Object {
    fn clone(&self) -> Self { (*self.0).clone() }
}


impl Neg for Object {
    type Output = Self;
    fn neg(self) -> Self::Output { self.unary(Unary::Neg) }
}

impl Add for Object {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output
        { self.binary(Binary::Add, rhs) }
}

impl Sub for Object {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output
        { self.binary(Binary::Sub, rhs) }
}

impl Mul for Object {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output
        { self.binary(Binary::Mul, rhs) }
}

impl Div for Object {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output
        { self.binary(Binary::Div, rhs) }
}

impl Rem for Object {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output
        { self.binary(Binary::Mod, rhs) }
}

impl Sum for Object {
    fn sum<I>(mut iter: I) -> Self where I: Iterator<Item = Self> {
        let mut total = if let Some(x) = iter.next() { x }
        else { return eval_err!(
            "Can only sum objects for non-empty iterators"
        )};

        for x in iter { total = try_ok!(total + x); }
        total
    }
}

impl Product for Object {
    fn product<I>(mut iter: I) -> Self where I: Iterator<Item = Self> {
        let mut total = if let Some(x) = iter.next() { x }
        else { return eval_err!(
            "Can only take product of objects for non-empty iterators"
        )};

        for x in iter { total = try_ok!(total * x); }
        total
    }
}

impl Object {
    pub fn flrdiv(self, rhs: Self) -> Self
        { self.binary(Binary::FlrDiv, rhs) }
}

impl AddAssign for Object {
    fn add_assign(&mut self, rhs: Self) { self.do_inside(|x| x + rhs) }
}

impl SubAssign for Object {
    fn sub_assign(&mut self, rhs: Self) { self.do_inside(|x| x - rhs) }
}

impl MulAssign for Object {
    fn mul_assign(&mut self, rhs: Self) { self.do_inside(|x| x * rhs) }
}

impl DivAssign for Object {
    fn div_assign(&mut self, rhs: Self) { self.do_inside(|x| x / rhs) }
}

impl RemAssign for Object {
    fn rem_assign(&mut self, rhs: Self) { self.do_inside(|x| x % rhs) }
}




#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalError {
    pub id: usize,
    pub msg: String,
}
name_type!{error: EvalError}

static EVAL_ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl EvalError {
    pub fn new(msg: String) -> ErrObject {
        use std::sync::atomic::Ordering;
        let id = EVAL_ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(EvalError {id, msg})
    }
}

impl Operable for EvalError {
    fn unary(self, _: Unary) -> Option<Object> { Some(Object::new(self)) }
    fn binary(self,
        _: bool, _: Binary, _: Object
    ) -> Result<Object, (Object, Object)> { Ok(Object::new(self)) }

    fn arity(&self, _: Option<&str>) -> Option<usize> { Some(0) }
    fn help(&self, _: Option<&str>) -> Option<String>
        { Some(self.to_string()) }
    fn call(&self, _: Option<&str>, _: Vec<Object>) -> Object
        { Object::new(self.clone()) }
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { write!(f, "Eval Error: {}", self.msg) }
}

impl<T: Into<Object>> From<Result<T, Object>> for Object {
    fn from(res: Result<T, Object>) -> Self { match res {
        Ok(x) => x.into(),
        Err(err) => err,
    }}
}

impl<T: Into<Object>> From<Result<T, String>> for Object {
    fn from(res: Result<T, String>) -> Self
        { res.map_err(&EvalError::new).into() }
}

impl<T: Into<Object>> From<Result<T, &str>> for Object {
    fn from(res: Result<T, &str>) -> Self
        { res.map_err(|s| s.to_owned()).into() }
}

