use std::collections::HashMap;

use super::object::Object;

macro_rules! def_bltn {
    ($pkg:ident.$name:ident = $val:expr) => {
        if $pkg.insert(
            stringify!($name).to_owned(),
            Object::new($val)
        ).is_some() {
            panic!(concat!(stringify!($name), " redeclared"))
        }
    };
    
    ($pkg:ident.$func:ident ($arg:ident : $tp:ty) = $body:expr) => {
        if $pkg.insert(stringify!($func).to_owned(),
        BltnFuncSingle::new(
            concat!(stringify!($pkg), '.', stringify!($func)),
            |$arg: $tp| $body
        )).is_some() {
            panic!(concat!(stringify!($func), " redeclared"))
        }
    };
    
    ($pkg:ident.$func:ident (
        $arg1:ident : $tp1:ty, $arg2:ident: $tp2:ty
    ) = $body:expr) => {
        if $pkg.insert(stringify!($func).to_owned(),
        BltnFuncDouble::new(
            concat!(stringify!($pkg), '.', stringify!($func)),
            |$arg1: $tp1, $arg2: $tp2| $body
        )).is_some() {
            panic!(concat!(stringify!($func), " redeclared"))
        }
    };
}

pub mod num;
pub mod vec;

pub fn make_bltns() -> HashMap<String, Object> {[
    ("num", num::make_bltns()),
    ("vec", vec::make_bltns()),
].map(|(key, obj)| (key.to_owned(), obj)).into()}

