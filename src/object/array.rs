use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Error, Write};
use std::cmp::Ordering;
use std::iter::repeat;

use super::opers::{Unary, Binary};
use super::{
    Operable, Object, Castable,
    NamedType, ErrObject, EvalError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
name_type!{array: Array}

impl Operable for Array {
    def_unary!{}
    def_binary!{self,
        self + other : (Array => Vec<Object>) = {
            let (mut s, mut other) = (self, other);
            s.0.append(&mut other);  s
        },

        (self ~) * idx : (Number => usize) =
            { repeat(self.0).take(idx).flatten().collect::<Object>() }
    }

    def_methods!{Array(arr),
        __call(idx: usize) = if let Some(obj) = arr.get(idx) { obj.clone() }
        else { eval_err!("Index {} is out of bounds", idx) },

        len() = arr.len().into(),
        fst() = arr.get(0).cloned().unwrap_or(
            eval_err!("Array doesn't have a first element")
        ),
        snd() = arr.get(1).cloned().unwrap_or(
            eval_err!("Array doesn't have a second element")
        ),
        last() = arr.last().cloned().unwrap_or(
            eval_err!("Array doesn't have a last element")
        ),

        map(func) = {
            let mut new_arr = Vec::with_capacity(arr.len());
            for x in arr.iter() { new_arr.push(call!(func(x.clone()))) }
            new_arr.into()
        },
        filter(pred) = {
            let mut new_arr = Vec::with_capacity(arr.len());
            for x in arr.iter() {
                if call!(pred(x.clone()) => bool) {
                    new_arr.push(x.clone());
                }
            }
            new_arr.into()
        },
        fold(init, func) = {
            let mut work = init;
            for x in arr.iter() {
                work = try_ok!(call!(func(work, x.clone())));
            }
            work
        },
        all(pred) = all_or_any(arr, true, pred),
        any(pred) = all_or_any(arr, false, pred),
        has(target) = arr.contains(&target).into(),

        sum() = arr.iter().cloned().sum(),
        prod() = arr.iter().cloned().product(),
        max() = extreme(arr, Ordering::Greater),
        min() = extreme(arr, Ordering::Less),
        rev() = arr.iter().cloned().rev().collect()
    }
}

fn all_or_any(objs: &Vec<Object>, is_all: bool, pred: Object) -> Object {
    for elem in objs.iter() {
        if is_all != call!(pred(elem.clone()) => bool) {
            return (!is_all).into();
        }
    }
    return is_all.into();
}

fn extreme(objs: &Vec<Object>, direc: Ordering) -> Object {
    if objs.len() == 0 { return eval_err!(
        "Empty array has no maximum or minimum"
    )}

    let mut ext_obj = &objs[0];
    for ob in objs  [1..].iter() {
        if let Some(ord) = ob.partial_cmp(ext_obj) {
            if ord == direc { ext_obj = ob; }
        } else { return eval_err!("Cannot compare all elements in array") }
    }
    ext_obj.clone()
}

impl Array {
    pub fn all(&self, pred: Object) -> Object
        { all_or_any(&self.0, true, pred) }
    pub fn any(&self, pred: Object) -> Object
        { all_or_any(&self.0, false, pred) }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('[')?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first { f.write_str(", ")?; }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}



impl<T> FromIterator<T> for Array where Object: From<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        Array(iter.into_iter().map(|x|
            x.into()
        ).collect())
    }
}

impl<T> FromIterator<T> for Object where Object: From<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self
        { Array::from_iter(iter).into() }
}


impl From<Array> for Object {
    fn from(arr: Array) -> Self { arr.0.into() }
}

impl<T> From<Vec<T>> for Object where Object: From<T> {
    fn from(elems: Vec<T>) -> Self {
        let mut objs = Vec::new();
        for elm in elems.into_iter() {
            let elm = Object::from(elm);
            if elm.is_err() {
                return elm;
            } else { objs.push(elm) }
        }
        Object::new(Array(objs))
    }
}

impl<T, const N: usize> From<[T; N]> for Object where Object: From<T> {
    fn from(arr: [T; N]) -> Object {
        Array(arr.into_iter().map(|x| x.into()).collect()).into()
    }
}



impl<T: Castable> Castable for Vec<T> where Object: From<T> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        let mut elems: VecDeque<Object> = Array::cast(obj)?.0.into();

        let mut casted = Vec::new();
        while let Some(x) = elems.pop_front() { match T::cast(x) {
            Ok(x) => casted.push(x),
            Err((x, err)) => {
                elems.push_front(x);
                return Err((
                    casted.into_iter().map(|x| x.into())
                    .chain(elems).collect(), err
                ))
            },
        }}
        Ok(casted.into())
    }
}

impl<T: Castable, const N: usize> Castable for [T; N]
where Object: From<T> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        Vec::<T>::cast(obj)?.try_into().map_err(|v: Vec<T>| {
            let len = v.len();
            (v.into(), eval_err!(
                "Array has {} elements, but {} were expected",
                len, N,
            ))
        })
    }
}

macro_rules! convert_tuple {
    (@multicast $on_err:expr, $($var:ident : $tp:ty),+) => {$(
        let $var = match <$tp>::cast($var) {
            Err(($var, err)) => return Err(($on_err, err)),
            Ok(good) => good,
        };
    )+};

    ($($var:ident : $tp:ident),+) => {
        impl<$($tp),+> From<($($tp,)+)> for Object
        where Object: $(From<$tp> +)+ {
            fn from(($($var,)+): ($($tp,)+)) -> Self
               { vec![$(Object::from($var)),+].into() }
        }

        impl<$($tp: Castable),+> Castable for ($($tp,)+)
        where Object: $(From<$tp> +)+ {
            fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
                let [$($var),+] = <[Object; count_tt!($($var)+)]>::cast(obj)?;
                convert_tuple!(@multicast
                    ($($var,)+).into(),
                    $($var: $tp),+
                );
                Ok(($($var,)+))
            }
        }
    };
}

convert_tuple!{x: A}
convert_tuple!{x: A, y: B}
convert_tuple!{x: A, y: B, z: C}

