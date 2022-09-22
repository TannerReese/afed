use std::any::Any;
use core::slice::Iter;
use std::clone::Clone;
use std::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display, Formatter, Error};

use super::opers::{Unary, Binary};

#[macro_export]
macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError(format!($($arg)*)) };
}

macro_rules! unary_not_impl {
    ($op:expr , $type:literal) => {
        eval_err!(concat!(
            "Unary operator {} not implemented for ", $type
        ), $op.symbol())
    }
}

macro_rules! binary_not_impl {
    ($op:expr , $type:literal) => {
        eval_err!(concat!(
            "Binary operator {} not implemented for ", $type
        ), $op.symbol())
    }
}

macro_rules! impl_objectish {
    ($t:ty) => {
        impl Objectish for $t {
            fn as_any(&self) -> &dyn Any { self }
            fn as_any_mut(&mut self) -> &mut dyn Any { self }
            
            fn clone(&self) -> Object { Object::new(Clone::clone(self)) }
            fn eq(&self, other: &Object) -> bool {
                if let Some(other) = other.downcast_ref::<$t>() {
                    self == other
                } else { false }
            }
        }
    }
}


pub mod null;
pub mod bool;
pub mod number;
pub mod string;
pub mod array;
pub mod map;


pub trait Operable<Rhs = Self, U = Unary, B = Binary> {
    type Output;
    fn apply_unary(&mut self, op: U) -> Self::Output;
    fn apply_binary(&mut self, op: B, other: Rhs) -> Self::Output;
   
    // Inclusive bounds for number of arguments
    fn arity(&self) -> (usize, usize);
    fn apply_call<'a>(&self, args: Iter<'a, Rhs>) -> Self::Output;
}

pub trait Objectish : Any + Debug + Display + Operable<Object, Output = EvalResult> {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    fn clone(&self) -> Object;
    fn eq(&self, other: &Object) -> bool;
}

pub struct Object(Box<dyn Objectish>);

impl Object {
    pub fn new<T>(obj: T) -> Object where T: Objectish {
        Object(Box::new(obj))
    }
    
    pub fn downcast_ref<'a, T: 'static>(&'a self) -> Option<&'a T> {
        (*self.0).as_any().downcast_ref()
    }
    
    pub fn downcast_mut<'a, T: 'static>(&'a mut self) -> Option<&'a mut T> {
        (*self.0).as_any_mut().downcast_mut()
    }
}


impl Operable<Object> for Object {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        (*self.0).apply_unary(op)
    }

    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        (*self.0).apply_binary(op, other)
    }
   
    fn arity(&self) -> (usize, usize) { (*self.0).arity() }
    fn apply_call<'a>(&self, args: Iter<'a, Object>) -> Self::Output { (*self.0).apply_call(args) }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Object({:?})", self.0)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool { (*self.0).eq(other) }
}

impl Eq for Object {}

impl Clone for Object {
    fn clone(&self) -> Self { self.0.deref().clone() }
    
    fn clone_from(&mut self, source: &Self) { *self = (*source).clone(); }
}

impl Deref for Object {
    type Target = dyn Objectish;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut Self::Target { self.0.deref_mut() }
}



#[derive(Debug, Clone)]
pub struct EvalError(pub String);

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Eval Error: {}", self.0)
    }
}

pub type EvalResult = Result<Object, EvalError>;

