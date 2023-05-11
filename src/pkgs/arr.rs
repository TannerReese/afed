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

use afed_objects::{array::Array, call, declare_pkg, number::Number, ErrObject, Object};

declare_pkg! {arr: #![bltn_pkg]
    /// arr.range (start: number) (end: number) -> array of numbers
    /// Generate sequence of numbers starting at 'start'
    /// increasing by one up to and potentially including 'end'
    fn range(start: Number, end: Number) -> Result<Vec<Number>, &'static str>
        { range_step(start, end, 1.into()) }

    /// arr.range_step (start: number) (end: number) (step: number) -> array of numbers
    /// Generate sequence of numbers starting at 'start'
    /// increasing by 'step' up to and potentially including 'end'
    pub fn range_step(
        start: Number, end: Number, step: Number
    ) -> Result<Vec<Number>, &'static str> {
        let mut start = start;
        let zero = 0.into();
        if step == zero { return Err("Cannot have a step of zero") }

        let is_desc = step < zero;
        if is_desc && start <= end { return Err(
            "When descending, the start must be greater than the end"
        )} else if !is_desc && end <= start { return Err(
            "When ascending, the start must be less than the end"
        )}

        let mut elems = Vec::new();
        let is_desc = if is_desc { -1 } else { 1 }.into();
        while (end - start) * is_desc >= zero {
            elems.push(start);
            start += step;
        }
        Ok(elems)
    }

    /// arr.iter (x0: any) (len: natural) (f: any -> any) -> array
    /// Generate array by repeatedly applying 'f' to 'x0'
    /// so the final array has length 'len'
    /// Example:    'arr.iter x 3 f == [x, f x, f (f x)]'
    pub fn iter(
        init: Object, times: usize, func: Object
    ) -> Result<Vec<Object>, ErrObject> {
        if times == 0 { return Ok(vec![]) }

        let mut elems = Vec::new();
        let mut work = init;
        for _ in 1..times {
            elems.push(work.clone());
            work = call!(func(work)).ok_or_err()?;
        }
        elems.push(work);
        Ok(elems)
    }

    /// arr.iter_while (x0: any) (pred: any -> bool) (f: any -> any) -> array
    /// Generate array by repeatedly applying 'f' to 'x0'
    /// until 'pred' fails for one of the results
    /// Example:    'arr.iter_while 0 (\x: x < 3) (\x: x + 1)== [0, 1, 2]'
    pub fn iter_while(
        init: Object, pred: Object, func: Object
    ) -> Result<Vec<Object>, ErrObject> {
        let mut elems = Vec::new();
        let mut work = init;
        while call!(pred(work.clone())).ok_or_err()?.cast()? {
            elems.push(work.clone());
            work = call!(func(work));
        }
        Ok(elems)
    }



    /// arr.zip (xs: array) (ys: array) -> array of [any, any]
    /// Create array by pairing up corresponding
    /// elements of 'xs' and 'ys' for their shared length
    /// Example:    'arr.zip [0, true, 2] ["a", 2] == [[0, "a"], [true, 2]]'
    pub fn zip(v1: Vec<Object>, v2: Vec<Object>) -> Vec<(Object, Object)>
        { std::iter::zip(v1, v2).collect() }

    /// arr.zip_with (f: (x: any) (y: any) -> any) (xs: array) (ys: array) -> array
    /// Create array by applying 'f' to corresponding
    /// elements of 'xs' and 'ys' for their shared length
    /// Example:    'arr.zip_with (\x y: x + y) [0, 1, 2] [10, 20] == [10, 21]'
    pub fn zip_with(f: Object, v1: Vec<Object>, v2: Vec<Object>) -> Vec<Object>
        { std::iter::zip(v1, v2).map(|(x, y)| call!(f(x, y))).collect() }



    /// arr.fst (a: array) -> any
    /// First element of 'a'
    pub fn fst(v: Vec<Object>) -> Result<Object, &'static str> {
        let mut v = v;
        if !v.is_empty() { Ok(v.remove(0)) }
        else { Err("Array does not have a first element")}
    }

    /// arr.snd (a: array) -> any
    /// Second element of 'a'
    pub fn snd(v: Vec<Object>) -> Result<Object, &'static str> {
        let mut v = v;
        if v.len() >= 2 { Ok(v.remove(1)) }
        else { Err("Array does not have a second element")}
    }

    /// arr.last (a: array) -> any
    /// Last element of 'a'
    pub fn last(v: Vec<Object>) -> Result<Object, &'static str> {
        let mut v = v;
        if let Some(elem) = v.pop() { Ok(elem) }
        else { Err("Array does not have a last element")}
    }



    /// arr.len (x: any) -> any
    /// Call method 'len' on 'x'
    fn len(obj: Object) -> Object { call!(obj.len) }
    /// arr.sum (x: any) -> any
    /// Call method 'sum' on 'x'
    fn sum(obj: Object) -> Object { call!(obj.sum) }
    /// arr.prod (x: any) -> any
    /// Call method 'prod' on 'x'
    fn prod(obj: Object) -> Object { call!(obj.prod) }
    /// arr.max (x: any) -> any
    /// Call method 'max' on 'x'
    fn max(obj: Object) -> Object { call!(obj.max) }
    /// arr.min (x: any) -> any
    /// Call method 'min' on 'x'
    fn min(obj: Object) -> Object { call!(obj.min) }
    /// arr.rev (x: any) -> any
    /// Call method 'rev' on 'x'
    fn rev(obj: Object) -> Object { call!(obj.rev) }

    /// arr.map (f: any -> any) (a: array) -> array
    /// Apply 'f' to every element of 'a'
    fn map(f: Object, a: Array) -> Array { a.map(f) }
    /// arr.filter (pred: any -> bool) (a: array) -> array
    /// Apply 'pred' to every element of 'a'.
    /// Creates new array containing elements that return true
    fn filter(pred: Object, a: Array) -> Result<Array, Object>
        { a.filter(pred) }
    /// arr.fold (init: any) (f: (accum: any) (x: any) -> any) (a: array) -> any
    /// Fold values into accumulator starting with 'init'.
    /// 'f' takes the accumulator 'accum' and
    /// the next element 'x' of 'a' as arguments
    fn fold(init: Object, f: Object, a: Array) -> Result<Object, Object>
        { a.fold(init, f) }
    /// arr.all (pred: any -> bool) (a: array) -> bool
    /// Check if all the elements of 'a' fulfill 'pred'
    fn all(pred: Object, a: Array) -> bool { a.all(pred) }
    /// arr.any (pred: any -> bool) (a: array) -> bool
    /// Check if any element in 'a' fulfills 'pred'
    fn any(pred: Object, a: Array) -> bool { a.any(pred) }
    /// arr.has (target: any) (a: array) -> bool
    /// Check if 'a' contains the element 'target'
    fn has(elem: Object, a: Array) -> bool { a.has(elem) }
}
