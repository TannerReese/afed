use std::any::{Any, TypeId};
use std::vec::Vec;
use std::clone::Clone;
use std::fmt::{Debug, Display, Formatter, Error};

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

macro_rules! try_expect {
    ($obj:expr) => { match $obj.expect() {
        Ok(val) => val,
        Err(err) => return err,
    }};
    ($obj:expr, $type:ty) => { match $obj.expect::<$type>() {
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

macro_rules! unary_not_impl {
    ($op:expr , $self:expr) => {
        eval_err!(
            "Unary operator {} not implemented for {}",
            $op.symbol(), $self.type_name_dyn()
        )
    };
}

macro_rules! binary_not_impl {
    ($op:expr , $self:expr) => {
        eval_err!(
            "Binary operator {} not implemented for {}",
            $op.symbol(), $self.type_name_dyn()
        )
    };
}

macro_rules! call_not_impl {
    ($type:ty) => {
        fn arity(&self) -> usize { 0 }
        fn apply_call(&self, _: Vec<Object>) -> Object {
            eval_err!("Cannot call {}", self.type_name_dyn())
        }
    };
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

pub trait Objectish : Any + Debug + Display + Operable<Object, Output = Object> {
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
    
    pub fn is_err(&self) -> bool {
        TypeId::of::<EvalError>() == (*self.0).type_id()
    }
    
    pub fn expect<T>(self) -> Result<T, Object> where T: NamedType {
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

impl<const N: usize> From<[Object; N]> for Object {
    fn from(arr: [Object; N]) -> Object {
        Object::new(array::Array(arr.into()))
    }
}

impl<const N: usize> From<[(&str, Object); N]> for Object {
    fn from(arr: [(&str, Object); N]) -> Object {
        Object::new(map::Map {
            unnamed: Vec::new(),
            named: arr.map(|(key, obj)| (key.to_owned(), obj)).into(),
        })
    }
}


impl Object {
    pub fn apply_unary(mut self, op: Unary) -> Object {
        (*self.0).apply_unary(op)
    }
    
    pub fn apply_binary(mut self, op: Binary, mut other: Object) -> Object { match op {
        Binary::Apply => self.apply_call(vec![other]),
        Binary::Eq => Bool::new((*self.0).eq(&other)),
        Binary::Neq => Bool::new(!(*self.0).eq(&other)),
        Binary::Leq => (*self.0).apply_binary(Binary::Leq, other),
        Binary::Geq => (*other.0).apply_binary(Binary::Leq, self),
        Binary::Lt => if (*self.0).eq(&other) { Bool::new(false) } else {
            (*self.0).apply_binary(Binary::Leq, other)
        },
        Binary::Gt => if (*self.0).eq(&other) { Bool::new(false) } else {
            (*other.0).apply_binary(Binary::Leq, self)
        },
        _ => (*self.0).apply_binary(op, other),
    }}
    
    pub fn arity(&self) -> usize { (*self.0).arity() }
    pub fn apply_call<'a>(&self, args: Vec<Object>) -> Object {
        let arity = (*self.0).arity();
        if args.len() == arity {
            return (*self.0).apply_call(args);
        } else if args.len() < arity {
            return curry::Curry::new(self.clone(), args);
        }
        
        let mut args = args.into_iter();
        let iter = args.by_ref();
        
        let mut res = try_ok!((*self.0).apply_call(iter.take(arity).collect()));
        while iter.as_slice().len() > 0 {
            let arity = (*res.0).arity();
            res = if iter.as_slice().len() >= arity {
                try_ok!((*res.0).apply_call(iter.take(arity).collect()))
            } else { curry::Curry::new(res.clone(), iter.collect()) };
        }
        res
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



#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalError {
    pub id: usize,
    pub msg: String,
}
impl NamedType for EvalError { fn type_name() -> &'static str { "error" }}
impl Objectish for EvalError { impl_objectish!{} }

static EVAL_ERROR_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl EvalError {
    pub fn new(msg: String) -> Object {
        let id = EVAL_ERROR_COUNTER.fetch_add(1, Ordering::Relaxed);
        Object::new(EvalError {id, msg})
    }
}

impl Operable<Object> for EvalError {
    type Output = Object;
    fn apply_unary(&mut self, _: Unary) -> Self::Output {
        Object::new(mem::take(self))
    }
    
    fn apply_binary(&mut self, _: Binary, _: Object) -> Self::Output {
        Object::new(mem::take(self))
    }
    
    fn arity(&self) -> usize { 0 }
    fn apply_call(&self, _: Vec<Object>) -> Object {
        Objectish::clone(self)
    }
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Eval Error: {}", self.msg)
    }
}

