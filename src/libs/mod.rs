use std::collections::HashMap;

use super::object::Object;

macro_rules! count_tt {
    () => { 0 };
    ($fst:tt $($item:tt)*) => {1 + count_tt!($($item)*)};
}

macro_rules! def_bltn {
    ($pkg:ident.$name:ident = $val:expr) => {
        if $pkg.insert(
            stringify!($name).to_owned(),
            Object::new($val)
        ).is_some() {
            panic!(concat!(stringify!($name), " redeclared"))
        }
    };

    ($pkg:ident.$func:ident ($($arg:ident : $tp:ty),+) = $body:expr) => {
        if $pkg.insert(stringify!($func).to_owned(),
        BltnFunc::new(
            concat!(stringify!($pkg), '.', stringify!($func)),
            {
                fn unwrap(
                    arr: [Object; count_tt!($($arg)+)]
                ) -> Result<($($tp,)*), Object> {
                    let [$($arg,)+] = arr;
                    let mut _idx = 0;
                    Ok(($(match $arg.cast::<$tp>() {
                        Ok(val) => { _idx += 1; val },
                        Err(err) => return Err(err),
                    },)*))
                }

                |args: [Object; count_tt!($($arg)+)]| match unwrap(args) {
                    Ok(($($arg,)*)) => $body,
                    Err(err) => err,
                }
            }
       )).is_some() {
            panic!(concat!(stringify!($func), " redeclared"))
        }
    };
}

macro_rules! def_getter {
    ($pkg:ident.$getter:ident) => {
        def_getter!($pkg.$getter, stringify!($getter))
    };
    ($pkg:ident.$getter:ident, attr=$attr:expr) => {
        def_getter!($pkg.$method, $attr)
    };
    ($pkg:ident.$getter:ident, $attr:expr) => {
        def_bltn!($pkg.$getter(obj: Object) =
            obj.call(Some($attr), Vec::with_capacity(0))
        )
    };
}

pub mod bltn_func;

pub mod num;
pub mod arr;
pub mod vec;
pub mod mat;
mod augmat;

pub fn make_bltns() -> HashMap<String, Object> {[
    ("num", num::make_bltns()),
    ("arr", arr::make_bltns()),
    ("vec", vec::make_bltns()),
    ("mat", mat::make_bltns()),
].map(|(key, obj)| (key.to_owned(), obj)).into()}

