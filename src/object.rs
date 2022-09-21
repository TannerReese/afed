use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;

use super::opers::{Unary, Binary};


#[derive(Debug, Clone)]
pub struct EvalError(pub String);

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Eval Error: {}", self.0)
    }
}

#[macro_export]
macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError(format!($($arg)*)) };
}

#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Object>),
    Map(Vec<Object>, HashMap<String, Object>),
}

pub type EvalResult = Result<Object, EvalError>;


impl Object {
    pub fn apply_unary(self, op: Unary) -> EvalResult {
        match self {
            Object::Null => Err(eval_err!("Cannot apply unary operator to null")),
            Object::Bool(b) => match op {
                Unary::Neg => Ok(Object::Bool(!b)),
            },
            Object::Num(r) => match op {
                Unary::Neg => Ok(Object::Num(-r)),
            },
            Object::Str(_) => Err(eval_err!("Unary operator not implemented for string")),
            
            Object::Arr(_) => Err(eval_err!("Cannot apply unary operator to array")),
            Object::Map(_, _) => Err(eval_err!("Cannot apply unary operator to map")),
        }
    }
    
    pub fn apply_binary(self, op: Binary, other: Self) -> EvalResult {
        match self {
            Object::Null => Err(eval_err!("Binary operator cannot be applied to null")),
            Object::Bool(b1) => if let Object::Bool(b2) = other { match op {
                Binary::Add | Binary::Sub => Ok(Object::Bool(b1 ^ b2)),
                Binary::Mul => Ok(Object::Bool(b1 && b2)),
                _ => Err(eval_err!("Binary operator not implemented for booleans")),
            }} else { Err(eval_err!("Boolean can only be combined with another boolean")) },
            
            Object::Num(r1) => if let Object::Num(r2) = other { match op {
                Binary::Add => Ok(Object::Num(r1 + r2)),
                Binary::Sub => Ok(Object::Num(r1 - r2)),
                Binary::Mul => Ok(Object::Num(r1 * r2)),
                Binary::Div => Ok(Object::Num(r1 / r2)),
                Binary::Mod => Ok(Object::Num(r1.rem_euclid(r2))),
                Binary::Pow => Ok(Object::Num(r1.powf(r2))),
            }} else { Err(eval_err!("Number can only be combined with another number")) },
            
            Object::Str(s1) => if let Object::Str(s2) = other { match op {
                Binary::Add => Ok(Object::Str(s1 + &s2)),
                _ => Err(eval_err!("Binary operator not implemented for string")),
            }} else {Err(eval_err!("String can only be combined with another string")) },
            
            Object::Arr(_) => Err(eval_err!("Cannot apply binary operator to array")),
            Object::Map(_, _) => Err(eval_err!("Cannot apply binary operator to map")),
        }
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use std::fmt::Write;
        
        match self {
            Object::Null => write!(f, "null")?,
            Object::Bool(b) => write!(f, "{}", b)?,
            Object::Num(r) => write!(f, "{}", r)?,
            Object::Str(s) => write!(f, "\"{}\"", s)?,
            
            Object::Arr(elems) => {
                f.write_char('[')?;
                let mut is_first = true;
                for obj in elems.iter() {
                    if !is_first { f.write_str(", ")?; }
                    is_first = false;
                    write!(f, "{}", obj)?;
                }
                f.write_char(']')?;
            },
            Object::Map(free_elems, elems) => {
                f.write_char('{')?;
                let mut is_first = true;
                for obj in free_elems.iter() {
                    if !is_first { f.write_str(", ")?; }
                    is_first = false;
                    write!(f, "{}", obj)?;
                }
                
                for (key, obj) in elems.iter() {
                    if !is_first { f.write_str(", ")?; }
                    is_first = false;
                    write!(f, "\"{}\": {}", key, obj)?;
                }
                f.write_char('}')?;
            },
        }
        Ok(())
    }
}

