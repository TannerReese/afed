use std::collections::HashMap;

use super::expr::Bltn;

macro_rules! def_bltn {
    (impl type_or_default) => { Object };
    (impl type_or_default $tp:ty) => { $tp };


    ($pkg:ident.$name:ident = $val:expr) =>
        { def_bltn!(false, $pkg.$name = $val) };
    (static $pkg:ident.$name:ident = $val:expr) =>
        { def_bltn!(true, $pkg.$name = $val) };

    ($global:literal, $pkg:ident.$name:ident = $val:expr) => {
        if $pkg.insert(
            stringify!($name).to_owned(),
            ($global, Bltn::Const($val.into())),
        ).is_some() {
            panic!(concat!(stringify!($name), " redeclared"))
        }
    };


    ($pkg:ident.$func:ident($($tok:tt)*) = $body:expr) =>
        { def_bltn!(false, $pkg(stringify!($pkg)).$func($($tok)*) = $body) };
    (static $pkg:ident.$func:ident($($tok:tt)*) = $body:expr) =>
        { def_bltn!(true, $pkg(stringify!($pkg)).$func($($tok)*) = $body) };
    ($pkg:ident($name:expr).$func:ident($($tok:tt)*) = $body:expr) =>
        { def_bltn!(false, $pkg($name).$func($($tok)*) = $body) };
    (static $pkg:ident($name:expr).$func:ident($($tok:tt)*) = $body:expr) =>
        { def_bltn!(true, $pkg($name).$func($($tok)*) = $body) };

    ($global:literal, $pkg:ident($name:expr).$func:ident (
        $($arg:ident $(: $tp:ty)?),+
    ) = $body:expr) => {
        if $pkg.insert(stringify!($func).to_owned(),
            ($global, Bltn::Const(BltnFunc::new(
                concat!($name, '.', stringify!($func)),
                {
                    fn unwrap(
                        arr: [Object; count_tt!($($arg)+)]
                    ) -> Result<($(
                        def_bltn!(impl type_or_default $($tp)?),
                    )*), Object> {
                        let [$($arg,)+] = arr;
                        let mut _idx = 0;
                        Ok(($(match $arg.cast() {
                            Ok(val) => { _idx += 1; val },
                            Err(err) => return Err(err),
                        },)*))
                    }

                    |args: [Object; count_tt!($($arg)+)]| {
                        match unwrap(args) {
                            Ok(($($arg,)*)) => $body,
                            Err(err) => err,
                        }
                    }
                }
            )))
        ).is_some() { panic!(
            concat!($name, '.', stringify!($func), " redeclared")
        )}
    };
}

macro_rules! def_getter {
    ($pkg:ident.$getter:ident) =>
        { def_getter!(false, $pkg.$getter, stringify!($getter)) };
    (static $pkg:ident.$getter:ident) =>
        { def_getter!(true, $pkg.$getter, stringify!($getter)) };

    ($pkg:ident.$getter:ident, attr=$attr:expr) =>
        { def_getter!(false, $pkg.$method, $attr) };
    (static $pkg:ident.$getter:ident, attr=$attr:expr) =>
        { def_getter!(true, $pkg.$method, $attr) };

    ($global:literal, $pkg:ident.$getter:ident, $attr:expr) => {
        def_bltn!($global, $pkg(stringify!($pkg)).$getter(obj) =
            obj.call(Some($attr), Vec::with_capacity(0))
        )
    };
}

macro_rules! def_pkg {
    ($parent:ident.$name:ident = $pkg:expr) =>
        { def_pkg!(false, $parent(stringify!($name)) = $pkg) };
    (static $parent:ident.$name:ident = $pkg:expr) =>
        { def_pkg!(true, $parent(stringify!($name)) = $pkg) };
    ($parent:ident($name:expr) = $pkg:expr) =>
        { def_pkg!(false, $parent($name) = $pkg) };
    (static $parent:ident($name:expr) = $pkg:expr) =>
        { def_pkg!(true, $parent($name) = $pkg) };

    ($make_global:literal, $parent:ident($name:expr) = $pkg:expr) => {{
        let mut pkg = match $pkg {
            Bltn::Const(_) => panic!("Package must be map"),
            Bltn::Map(elems) => elems,
        };

        if $make_global {
            for (_, (is_global, _)) in pkg.iter_mut() {
                *is_global = true;
            }
        }

        if $parent.insert(
            $name.to_owned(), (false, Bltn::Map(pkg))
        ).is_some() { panic!(
            concat!("Package ", $name, " redeclared")
        )}
    }};
}

pub mod bltn_func;

pub mod num;
pub mod arr;

pub mod prs;
pub mod modulo;

pub mod vec;
pub mod mat;
mod augmat;

pub mod calc;
pub mod plot;

pub fn make_bltns() -> Bltn {
    let mut root = HashMap::new();
    def_pkg!(static root.num = num::make_bltns());
    def_pkg!(static root.arr = arr::make_bltns());

    def_pkg!(root.prs = prs::make_bltns());
    def_pkg!(root("mod") = modulo::make_bltns());

    def_pkg!(root.vec = vec::make_bltns());
    def_pkg!(root.mat = mat::make_bltns());

    def_pkg!(root.calc = calc::make_bltns());
    def_pkg!(root.plt = plot::make_bltns());
    Bltn::Map(root)
}

