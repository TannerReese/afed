// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Error, Formatter, Write};
use std::iter::repeat;

use super::{Castable, ErrObject, Object};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array(pub Vec<Object>);
name_type! {array: Array}

impl_operable! {Array:
    //! Dynamically sized heterogeneous list of objects

    /// array + array -> array
    /// Concatentate two arrays
    #[binary(Add)]
    fn _(arr1: Self, arr2: Vec<Object>) -> Self {
        let (mut arr1, mut arr2) = (arr1, arr2);
        arr1.0.append(&mut arr2);  arr1
    }

    /// array * (n: natural) -> array
    /// (n: natural) * array -> array
    /// Concatenate 'n' copies of 'array' together
    #[binary(Mul, comm)]
    fn _(arr: Vec<Object>, times: usize) -> Vec<Object>
        { repeat(arr).take(times).flatten().collect() }

    /// array (i: natural) -> any
    /// Return element of 'array' at index 'i'
    #[call]
    fn __call(&self, idx: usize) -> Result<Object, String> {
        if let Some(obj) = self.0.get(idx) { Ok(obj.clone()) }
        else { Err(format!("Index {} is out of bounds", idx)) }
    }

    /// array.len -> natural
    /// Number of elements in 'array'
    pub fn len(&self) -> usize { self.0.len() }
    /// array.is_empty -> bool
    /// Whether array has no elements
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    /// array.fst -> any
    /// First element of 'array'
    pub fn fst(&self) -> Result<Object, &str>
        { self.0.get(0).cloned().ok_or("Array has no first element") }
    /// array.snd -> any
    /// Second element of 'array'
    pub fn snd(&self) -> Result<Object, &str>
        { self.0.get(1).cloned().ok_or("Array has no second element") }
    /// array.last -> any
    /// Last element of 'array'
    pub fn last(&self) -> Result<Object, &str>
        { self.0.last().cloned().ok_or("Array has no last element") }

    /// array.map (f: (x: any) -> any) -> array
    /// Apply function 'func' to every element of 'array'
    pub fn map(self, func: Object) -> Self
        { self.0.into_iter().map(|x| call!(func(x))).collect() }

    /// array.filter (pred: (x: any) -> bool) -> array
    /// Apply 'pred' to every element of 'array'.
    /// Creates new array containing elements that return true
    pub fn filter(self, pred: Object) -> Result<Self, ErrObject> {
        let mut new_arr = Vec::with_capacity(self.0.len());
        for x in self.0.into_iter() {
            if call!(pred(x.clone())).cast()? {
                new_arr.push(x)
            }
        }
        Ok(new_arr.into())
    }

    /// array.fold (init: any) (f: (accum: any) (x: any) -> any) -> any
    /// Fold values into accumulator starting with 'init'.
    /// 'f' takes the accumulator 'accum' and
    /// the next element 'x' of 'array' as arguments
    pub fn fold(self,
        init: Object, func: Object
    ) -> Result<Object, ErrObject> {
        let mut work = init;
        for x in self.0.into_iter() {
            work = call!(func(work, x)).ok_or_err()?;
        }
        Ok(work)
    }

    /// array.all (pred: (x: any) -> bool) -> bool
    /// Check if all the elements fulfill 'pred'
    pub fn all(&self, pred: Object) -> bool { self.all_or_any(true, pred) }
    /// array.any (pred: (x: any) -> bool) -> bool
    /// Check if any element fulfills 'pred'
    pub fn any(&self, pred: Object) -> bool { self.all_or_any(false, pred) }

    /// array.has (target: any) -> bool
    /// Check if 'array' contains the element 'target'
    pub fn has(&self, target: Object) -> bool { self.0.contains(&target) }

    /// array.sum -> any
    /// Sum all of the elements in the 'array'
    pub fn sum(self) -> Object { self.0.into_iter().sum() }
    /// array.prod -> any
    /// Multiply all of the elements in the 'array'
    pub fn prod(self) -> Object { self.0.into_iter().product() }

    /// array.max -> any
    /// Maximum of the elements in the 'array'
    pub fn max(&self) -> Result<Object, &'static str>
        { self.extreme(Ordering::Greater) }
    /// array.min -> any
    /// Minimum of the elements in the 'array'
    pub fn min(&self) -> Result<Object, &'static str>
        { self.extreme(Ordering::Less) }

    /// array.rev -> array
    /// Reverse the order of elements in the 'array'
    pub fn rev(self) -> Self { self.0.into_iter().rev().collect() }
}

impl Array {
    fn all_or_any(&self, is_all: bool, pred: Object) -> bool {
        for elem in self.0.iter() {
            match call!(pred(elem.clone())).cast() {
                Err(_) => return false,
                Ok(is_true) => {
                    if is_all != is_true {
                        return !is_all;
                    }
                }
            }
        }
        is_all
    }

    fn extreme(&self, direc: Ordering) -> Result<Object, &'static str> {
        if self.0.is_empty() {
            return Err("Empty array has no maximum or minimum");
        }

        let mut ext_obj = &self.0[0];
        for ob in self.0[1..].iter() {
            if let Some(ord) = ob.partial_cmp(ext_obj) {
                if ord == direc {
                    ext_obj = ob;
                }
            } else {
                return Err("Cannot compare all elements in array");
            }
        }
        Ok(ext_obj.clone())
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('[')?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first {
                f.write_str(", ")?;
            }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}

impl<T: Into<Object>> FromIterator<T> for Array {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Array(iter.into_iter().map(|x| x.into()).collect())
    }
}

impl<T: Into<Object>> FromIterator<T> for Object {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Array::from_iter(iter).into()
    }
}

impl From<Array> for Vec<Object> {
    fn from(arr: Array) -> Self {
        arr.0
    }
}

impl From<Array> for Object {
    fn from(arr: Array) -> Self {
        arr.0.into()
    }
}

impl<T: Into<Object>> From<Vec<T>> for Array {
    fn from(elems: Vec<T>) -> Self {
        elems.into_iter().map(|x| x.into()).collect()
    }
}

impl<T: Into<Object>> From<Vec<T>> for Object {
    fn from(elems: Vec<T>) -> Self {
        let mut objs = Vec::new();
        for elm in elems.into_iter() {
            let elm = elm.into();
            if elm.is_err() {
                return elm;
            } else {
                objs.push(elm)
            }
        }
        Object::new(Array(objs))
    }
}

impl<T: Into<Object>, const N: usize> From<[T; N]> for Object {
    fn from(arr: [T; N]) -> Object {
        Array(arr.into_iter().map(|x| x.into()).collect()).into()
    }
}

impl<T: Into<Object> + Castable> Castable for Vec<T> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        let mut elems: VecDeque<Object> = Array::cast(obj)?.0.into();

        let mut casted = Vec::new();
        while let Some(x) = elems.pop_front() {
            match T::cast(x) {
                Ok(x) => casted.push(x),
                Err((x, err)) => {
                    elems.push_front(x);
                    return Err((
                        casted.into_iter().map(|x| x.into()).chain(elems).collect(),
                        err,
                    ));
                }
            }
        }
        Ok(casted)
    }
}

impl<T: Into<Object> + Castable, const N: usize> Castable for [T; N] {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        Vec::<T>::cast(obj)?.try_into().map_err(|v: Vec<T>| {
            let len = v.len();
            (
                v.into(),
                eval_err!("Array has {} elements, but {} were expected", len, N,),
            )
        })
    }
}

// Creates conversion trait implementations between tuples and `Object`
macro_rules! convert_tuple {
    (@multicast $on_err:expr, $($var:ident : $tp:ty),+) => {$(
        let $var = match <$tp>::cast($var) {
            Err(($var, err)) => return Err(($on_err, err)),
            Ok(good) => good,
        };
    )+};

    ($($var:ident : $tp:ident),+) => {
        impl<$($tp: Into<Object>),+> From<($($tp,)+)> for Object {
            fn from(($($var,)+): ($($tp,)+)) -> Self
               { vec![$($var.into()),+].into() }
        }

        impl<$($tp: Into<Object> + Castable),+> Castable for ($($tp,)+) {
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

convert_tuple! {x: A}
convert_tuple! {x: A, y: B}
convert_tuple! {x: A, y: B, z: C}
