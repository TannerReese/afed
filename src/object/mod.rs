use std::any::Any;
use std::vec::Vec;
use std::clone::Clone;
use std::fmt::{Debug, Display, Formatter, Error};

use std::hash::Hash;
use std::borrow::Borrow;

use opers::{Unary, Binary};

#[macro_export]
macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError(format!($($arg)*)) };
}

macro_rules! unary_not_impl {
    ($op:expr , $self:expr) => {
        eval_err!(
            "Unary operator {} not implemented for {}",
            $op.symbol(), $self.type_name_dyn()
        )
    }
}

macro_rules! binary_not_impl {
    ($op:expr , $self:expr) => {
        eval_err!(
            "Binary operator {} not implemented for {}",
            $op.symbol(), $self.type_name_dyn()
        )
    }
}

macro_rules! call_not_impl {
    ($type:ty) => {
        fn arity(&self) -> usize { 0 }
        fn apply_call(&self, _: Vec<Object>) -> EvalResult {
            Err(eval_err!("Cannot call {}", self.type_name_dyn()))
        }
    }
}

macro_rules! impl_objectish {
    () => {
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn type_name_dyn(&self) -> &'static str { <Self as NamedType>::type_name() }
        
        fn clone(&self) -> Object { Object::new(Clone::clone(self)) }
        fn eq(&self, other: &Object) -> bool {
            if let Some(other) = other.downcast_ref::<Self>() {
                self == other
            } else { false }
        }
    }
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


pub trait Operable<Rhs = Self, U = Unary, B = Binary> {
    type Output;
    fn apply_unary(&mut self, op: U) -> Self::Output;
    fn apply_binary(&mut self, op: B, other: Rhs) -> Self::Output;
    fn arity(&self) -> usize;
    fn apply_call(&self, args: Vec<Rhs>) -> Self::Output;
}

pub trait NamedType : Any {
    fn type_name() -> &'static str;
}

pub trait Objectish : Any + Debug + Display + Operable<Object, Output = EvalResult> {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn type_name_dyn(&self) -> &'static str;
    
    fn clone(&self) -> Object;
    fn eq(&self, other: &Object) -> bool;
}

pub struct Object(Box<dyn Objectish>);

impl Object {
    pub fn new<T>(obj: T) -> Object where T: Objectish {
        Object(Box::new(obj))
    }
    
    pub fn downcast<T>(self) -> Result<T, EvalError> where T: NamedType {
        let given_type = (*self.0).type_name_dyn();
        let box_any = unsafe { Box::from_raw(Box::leak(self.0).as_any_mut()) };
        if let Ok(obj_box) = box_any.downcast() { Ok(*obj_box) }
        else { Err(eval_err!(
            "Expected {}, but found {}", T::type_name(), given_type
        ))}
    }
    
    pub fn downcast_ref<'a, T: 'static>(&'a self) -> Option<&'a T> {
        (*self.0).as_any().downcast_ref()
    }
    
    pub fn downcast_mut<'a, T: 'static>(&'a mut self) -> Option<&'a mut T> {
        (*self.0).as_any_mut().downcast_mut()
    }
    
    
    
    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq + std::fmt::Debug,
        String: Borrow<B>,
    { (*self.0).as_any().downcast_ref::<map::Map>().and_then(|m| m.get(key)) }
    
    pub fn find<'a, I, B>(&self, path: I) -> Option<&Object>
    where
        I: Iterator<Item = &'a B>,
        B: Hash + Eq + std::fmt::Debug + 'a,
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


impl Operable<Object> for Object {
    type Output = EvalResult;
    fn apply_unary(&mut self, op: Unary) -> Self::Output {
        (*self.0).apply_unary(op)
    }
    
    fn apply_binary(&mut self, op: Binary, other: Object) -> Self::Output {
        (*self.0).apply_binary(op, other)
    }
    
    fn arity(&self) -> usize { (*self.0).arity() }
    fn apply_call<'a>(&self, args: Vec<Object>) -> Self::Output {
        let arity = (*self.0).arity();
        if args.len() == arity {
            return (*self.0).apply_call(args);
        } else if args.len() < arity {
            return Ok(curry::Curry::new(self.clone(), args));
        }
        
        let mut args = args.into_iter();
        let iter = args.by_ref();
        
        let mut res = (*self.0).apply_call(iter.take(arity).collect())?;
        while iter.as_slice().len() > 0 {
            let arity = (*res.0).arity();
            res = if iter.as_slice().len() >= arity {
                (*res.0).apply_call(iter.take(arity).collect())?
            } else { curry::Curry::new(res.clone(), iter.collect()) };
        }
        Ok(res)
    }
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
    fn clone(&self) -> Self { (*self.0).clone() }
    
    fn clone_from(&mut self, source: &Self) { *self = (*source).clone(); }
}



#[derive(Debug, Clone)]
pub struct EvalError(pub String);

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Eval Error: {}", self.0)
    }
}

pub type EvalResult = Result<Object, EvalError>;

