use std::any::{Any, TypeId};
use std::vec::Vec;
use std::clone::Clone;
use std::fmt::{Debug, Display, Formatter, Error, Write};

use std::ops::{Neg, Add, Sub, Mul, Div, Rem};
use std::iter::Sum;

use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use std::hash::Hash;
use std::borrow::Borrow;

use opers::{Unary, Binary};
use self::bool::Bool;

#[macro_export]
macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError::new(format!($($arg)*)) };
}


#[macro_export]
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

#[macro_export]
macro_rules! try_ok {
    ($obj:expr) => {{
        let obj = $obj;
        if obj.is_err() { return obj } else { obj }
    }};
}

#[macro_export]
macro_rules! unary_not_impl {
    () => {
        fn try_unary(&self, _: Unary) -> bool { false }
        fn unary(self, _: Unary) -> Object { panic!() }
    };
}

#[macro_export]
macro_rules! binary_not_impl {
    () => {
        fn try_binary(&self, _: bool, _: Binary, _: &Object) -> bool { false }
        fn binary(self, _: bool, _: Binary, _: Object) -> Object { panic!() }
    };
}

#[macro_export]
macro_rules! call_not_impl {
    ($type:ty) => {
        fn arity(&self) -> usize { 0 }
        fn call(&self, _: Vec<Object>) -> Object {
            eval_err!("Cannot call {}", Self::type_name())
        }
    };
}

pub mod opers;
pub mod null;
pub mod bool;
pub mod number;
pub mod string;
pub mod array;
pub mod map;
pub mod curry;
pub mod bltn_func;


pub trait Operable<Rhs = Object, U = Unary, B = Binary> {
    type Output;
    fn try_unary(&self, op: U) -> bool;
    fn unary(self, op: U) -> Self::Output;
    fn try_binary(&self, rev: bool, op: B, other: &Rhs) -> bool;
    fn binary(self, rev: bool, op: B, other: Rhs) -> Self::Output;
    fn arity(&self) -> usize;
    fn call(&self, args: Vec<Rhs>) -> Self::Output;
}

pub trait NamedType : Any {
    fn type_name() -> &'static str;
    fn type_name_dyn(&self) -> &'static str { Self::type_name() }
}

pub trait Objectish :
    Eq + Clone + Any + NamedType
    + Debug + Display
    + Operable<Output=Object>
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
    
    fn try_unary(&self, op: Unary) -> bool;
    fn unary(&mut self, op: Unary) -> Object;
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool;
    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Object;
    fn arity(&self) -> usize;
    fn call(&self, args: Vec<Object>) -> Object;
}

fn to_obj<T>(obj: &mut Option<T>) -> T { mem::take(obj).unwrap() }
fn to_ref<T>(obj: &Option<T>) -> &T { obj.as_ref().unwrap() }
fn to_mut<T>(obj: &mut Option<T>) -> &mut T { obj.as_mut().unwrap() }

impl<T> ObjectishSafe for Option<T> where T: Objectish + 'static {
    fn as_any(&self) -> &dyn Any { as_any(to_ref(self)) }
    fn as_any_mut(&mut self) -> &mut dyn Any { as_any_mut(to_mut(self)) }
    fn type_name_dyn(&self) -> &'static str { to_ref(self).type_name_dyn() }
    
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
    
    fn try_unary(&self, op: Unary) -> bool
        { to_ref(self).try_unary(op) }
    fn unary(&mut self, op: Unary) -> Object
        { to_obj(self).unary(op) }
    fn try_binary(&self, rev: bool, op: Binary, other: &Object) -> bool
        { to_ref(self).try_binary(rev, op, other) }
    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Object
        { to_obj(self).binary(rev, op, other) }
    
    fn arity(&self) -> usize { to_ref(self).arity() }
    fn call(&self, args: Vec<Object>) -> Object { to_ref(self).call(args) }
}

pub struct Object(Box<dyn ObjectishSafe>);

impl Object {
    pub fn new<T>(obj: T) -> Object where T: Objectish {
        Object(Box::new(Some(obj)))
    }
    
    pub fn is_err(&self) -> bool {self.is_a::<EvalError>() }
    pub fn is_a<T>(&self) -> bool where T: Any
        { TypeId::of::<T>() == (*self.0).as_any().type_id() }
    
    pub fn cast<T>(self) -> Result<T, Object> where T: NamedType {
        let given_type = (*self.0).type_name_dyn();
        let box_any = unsafe { Box::from_raw(Box::leak(self.0).as_any_mut()) };
        if let Ok(obj_box) = box_any.downcast() { Ok(*obj_box) }
        else { Err(eval_err!(
            "Expected {}, but found {}", T::type_name(), given_type
        ))}
    }
    
    pub fn downcast_ref<'a, T: 'static>(&'a self) -> Option<&'a T>
        { (*self.0).as_any().downcast_ref() }
    
    pub fn downcast_mut<'a, T: 'static>(&'a mut self) -> Option<&'a mut T>
        { (*self.0).as_any_mut().downcast_mut() }
    
    
    
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

impl<T> From<T> for Object where T: Objectish {
    fn from(obj: T) -> Self { Object::new(obj) }
}



impl Object {
    pub fn unary(mut self, op: Unary) -> Object {
        if (*self.0).try_unary(op) { (*self.0).unary(op) }
        else { eval_err!(
            "Unary operator {} not implemented for type {}",
            op.symbol(), (*self.0).type_name_dyn(),
        )}
    }
    
    fn binary_help(mut self, op: Binary, mut other: Object) -> Object {
        if (*self.0).try_binary(false, op, &other) { (*self.0).binary(false, op, other) }
        else if (*other.0).try_binary(true, op, &self) { (*other.0).binary(true, op, self) }
        else { eval_err!(
            "Binary operator {} not implemented between types {} and {}",
            op.symbol(), (*self.0).type_name_dyn(), (*self.0).type_name_dyn(),
        )}
    }
    
    pub fn binary(self, op: Binary, other: Object) -> Object {
    match op {
        Binary::Apply => self.call(vec![other]),
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
    
    pub fn arity(&self) -> usize { (*self.0).arity() }
    pub fn call<'a>(&self, args: Vec<Object>) -> Object {
        let arity = (*self.0).arity();
        if args.len() == arity {
            return (*self.0).call(args);
        } else if args.len() < arity {
            return curry::Curry::new(self.clone(), args);
        }
        
        let mut args = args.into_iter();
        let iter = args.by_ref();
        
        let mut res = try_ok!((*self.0).call(iter.take(arity).collect()));
        while iter.as_slice().len() > 0 {
            let arity = (*res.0).arity();
            res = if iter.as_slice().len() >= arity {
                try_ok!((*res.0).call(iter.take(arity).collect()))
            } else { curry::Curry::new(res.clone(), iter.collect()) };
        }
        res
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
    fn sum<I>(iter: I) -> Self where I: Iterator<Item = Self> {
        iter.reduce(|accum, x| accum + x)
        .expect("Can only sum objects for non-empty interators")
    }
}

impl Object {
    pub fn flrdiv(self, rhs: Self) -> Self
        { self.binary(Binary::FlrDiv, rhs) }
}



#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalError {
    pub id: usize,
    pub msg: String,
}
impl NamedType for EvalError { fn type_name() -> &'static str { "error" }}
impl Objectish for EvalError {}

static EVAL_ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl EvalError {
    pub fn new(msg: String) -> Object {
        let id = EVAL_ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(EvalError {id, msg})
    }
}

impl Operable for EvalError {
    type Output = Object;
    fn try_unary(&self, _: Unary) -> bool { true }
    fn unary(self, _: Unary) -> Self::Output { Object::new(self) }
    fn try_binary(&self, _: bool, _: Binary, _: &Object) -> bool { true }
    fn binary(self, _: bool, _: Binary, _: Object) -> Self::Output { Object::new(self) }
    
    fn arity(&self) -> usize { 0 }
    fn call(&self, _: Vec<Object>) -> Object { Object::new(self.clone()) }
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
        { write!(f, "Eval Error: {}", self.msg) }
}

