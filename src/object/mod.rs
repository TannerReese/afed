use std::any::{Any, TypeId};
use std::vec::Vec;
use std::clone::Clone;
use std::fmt::{Debug, Display, Formatter, Error, Write};

use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::iter::{Sum, Product};

use std::sync::atomic::{AtomicUsize, Ordering};

use std::hash::Hash;
use std::borrow::Borrow;

pub use opers::{Unary, Binary, Assoc};
use self::bool::Bool;

macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError::new(format!($($arg)*)) };
}


macro_rules! try_cast {
    ($obj:expr) => { match $obj.cast() {
        Ok(val) => val,
        Err(err) => return err,
    }};
    ($obj:expr => $type:ty) => { match $obj.cast::<$type>() {
        Ok(val) => val,
        Err(err) => return err,
    }};
}

macro_rules! try_ok {
    ($obj:expr) => {{
        let obj = $obj;
        if obj.is_err() { return obj } else { obj }
    }};
}

macro_rules! obj_call {
    ($obj:ident ($($arg:expr),*)) => {
        $obj.call(None, vec![$($arg,)*])
    };
    ($obj:ident ($($arg:expr),*) => $tp:ty) => {
        try_cast!($obj.call(None, vec![$($arg,)*]) => $tp)
    };
    (($obj:expr).$method:ident ($($arg:expr),*)) => {
        $obj.call(Some(stringify!($method)), vec![$($arg,)*])
    };
    ($obj:ident.$method:ident ($($arg:expr),*)) => {
        $obj.call(Some(stringify!($method)), vec![$($arg,)*])
    };
    (($obj:expr).$method:ident ($($arg:expr),*) => $tp:ty) => {
        try_cast!($obj.call(Some(stringify!($method)), vec![$($arg,)*]) => $tp)
    };
    ($obj:ident.$method:ident ($($arg:expr),*) => $tp:ty) => {
        try_cast!($obj.call(Some(stringify!($method)), vec![$($arg,)*]) => $tp)
    };
}

macro_rules! unary_not_impl {
    () => {
        fn unary(self, _: Unary) -> Option<Object> { None }
    };
}

macro_rules! binary_not_impl {
    () => {
        fn try_binary(&self, _: bool, _: Binary, _: &Object) -> bool { false }
        fn binary(self, _: bool, _: Binary, _: Object) -> Object { panic!() }
    };
}

macro_rules! call_not_impl {
    () => {
        fn arity(&self, _: Option<&str>) -> Option<usize> { None }
        fn call(&self, _: Option<&str>, _: Vec<Object>) -> Object {
            eval_err!("Cannot call {}", Self::type_name())
        }
    };
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
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool;
    fn binary(self, rev: bool, op: Binary, other: Object) -> Object;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
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
fn as_any_mut<T>(x: &mut T) -> &mut dyn Any where T: Objectish { x }

trait ObjectishSafe {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name_dyn(&self) -> &'static str;

    fn clone(&self) -> Object;
    fn eq(&self, other: &Object) -> bool;
    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;

    fn unary(&mut self, op: Unary) -> Option<Object>;
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool;
    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Object;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object;
}

fn to_obj<T>(obj: &mut Option<T>) -> T { std::mem::take(obj).unwrap() }
fn to_ref<T>(obj: &Option<T>) -> &T { obj.as_ref().unwrap() }
fn to_mut<T>(obj: &mut Option<T>) -> &mut T { obj.as_mut().unwrap() }

impl<T> ObjectishSafe for Option<T> where T: Objectish + 'static {
    fn as_any(&self) -> &dyn Any { as_any(to_ref(self)) }
    fn as_any_mut(&mut self) -> &mut dyn Any { as_any_mut(to_mut(self)) }
    fn type_name_dyn(&self) -> &'static str { T::type_name() }

    fn clone(&self) -> Object { Object::new(to_ref(self).clone()) }
    fn eq(&self, other: &Object) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            to_ref(self) == other
        } else { false }
    }

    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { Display::fmt(to_ref(self), f) }
    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { Debug::fmt(to_ref(self), f) }

    fn unary(&mut self, op: Unary) -> Option<Object>
        { to_obj(self).unary(op) }
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool
        { to_ref(self).try_binary(rev, op, other) }
    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Object
        { to_obj(self).binary(rev, op, other) }

    fn arity(&self, attr: Option<&str>) -> Option<usize>
        { to_ref(self).arity(attr) }
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object
        { to_ref(self).call(attr, args) }
}

pub struct Object(Box<dyn ObjectishSafe>);

impl Object {
    pub fn new<T>(obj: T) -> Object where T: Objectish {
        Object(Box::new(Some(obj)))
    }

    pub fn is_err(&self) -> bool {self.is_a::<EvalError>() }
    pub fn type_id(&self) -> TypeId { (*self.0).as_any().type_id() }
    pub fn is_a<T>(&self) -> bool where T: Any
        { TypeId::of::<T>() == self.type_id() }

    pub fn cast<T>(self) -> Result<T, Object> where T: CastObject { T::cast(self) }

    pub fn downcast_ref<'a, T: 'static>(&'a self) -> Option<&'a T>
        { (*self.0).as_any().downcast_ref() }


    pub fn do_inside(&mut self, func: impl FnOnce(Self) -> Self){
        let owned = Object(std::mem::replace(&mut self.0,
            Box::new(None::<null::Null>)
        ));
        self.0 = func(owned).0;
    }


    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    { self.downcast_ref::<map::Map>().and_then(|m| m.get(key)) }

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

pub trait CastObject: Sized {
    fn cast(obj: Object) -> Result<Self, Object>;
}

impl<T> CastObject for T where T: NamedType + Sized {
    fn cast(obj: Object) -> Result<Self, Object> {
        let given_type = (*obj.0).type_name_dyn();
        let box_any = unsafe { Box::from_raw(Box::leak(obj.0).as_any_mut()) };
        if let Ok(obj_box) = box_any.downcast() { Ok(*obj_box) }
        else { Err(eval_err!(
            "Expected {}, but found {}", T::type_name(), given_type
        ))}
    }
}

impl CastObject for Object {
    fn cast(obj: Object) -> Result<Self, Object> { Ok(obj) }
}


impl Object {
    pub fn unary(mut self, op: Unary) -> Object {
        if let Some(obj) = (*self.0).unary(op) { obj }
        else { eval_err!(
            "Unary operator {} not implemented for type {}",
            op.symbol(), (*self.0).type_name_dyn(),
        )}
    }

    fn binary_help(mut self, op: Binary, mut other: Object) -> Object {
        if (*self.0).try_binary(false, op, &other) {
            (*self.0).binary(false, op, other)
        } else if (*other.0).try_binary(true, op, &self) {
            (*other.0).binary(true, op, self)
        } else { eval_err!(
            "Binary operator {} not implemented between types {} and {}",
            op.symbol(), (*self.0).type_name_dyn(), (*other.0).type_name_dyn(),
        )}
    }

    pub fn binary(self, op: Binary, other: Object) -> Object {
    match op {
        Binary::Apply => self.call(None, vec![other]),
        Binary::Eq => Bool::new(self == other),
        Binary::Neq => Bool::new(self != other),
        Binary::Leq => self.binary_help(Binary::Leq, other),
        Binary::Geq => other.binary_help(Binary::Leq, self),
        Binary::Lt => if self == other { Bool::new(false) } else {
            self.binary_help(Binary::Leq, other)
        },
        Binary::Gt => if self == other { Bool::new(false) } else {
            other.binary_help(Binary::Leq, self)
        },
        _ => self.binary_help(op, other),
    }}

    pub fn arity(&self, attr: Option<&str>) -> Option<usize>
        { (*self.0).arity(attr) }

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
impl NamedType for EvalError { fn type_name() -> &'static str { "error" }}

static EVAL_ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl EvalError {
    pub fn new(msg: String) -> Object {
        let id = EVAL_ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(EvalError {id, msg})
    }
}

impl Operable for EvalError {
    fn unary(self, _: Unary) -> Option<Object> { Some(Object::new(self)) }
    fn try_binary(&self, _: bool, _: Binary, _: &Object) -> bool { true }
    fn binary(self, _: bool, _: Binary, _: Object) -> Object { Object::new(self) }

    fn arity(&self, _: Option<&str>) -> Option<usize> { Some(0) }
    fn call(&self, _: Option<&str>, _: Vec<Object>) -> Object {
        Object::new(self.clone())
    }
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { write!(f, "Eval Error: {}", self.msg) }
}

