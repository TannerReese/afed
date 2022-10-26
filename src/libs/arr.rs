use std::collections::HashMap;

use crate::object::{Object, EvalError};
use crate::object::bool::Bool;
use crate::object::number::Number;
use crate::object::array::Array;
use crate::object::bltn_func::BltnFunc;

pub fn range(mut start: Number, end: Number, step: Number) -> Object {
    let zero = (0 as i64).into();
    if step == zero {
        return eval_err!("Cannot have a step of zero")
    }

    let is_desc = step < zero;
    if is_desc && start <= end {
        return eval_err!("When descending, the start must be greater than the end")
    } else if !is_desc && end <= start {
        return eval_err!("When ascending, the start must be less than the end")
    }

    let mut elems = Vec::new();
    let is_desc = (if is_desc { -1 } else { 1 } as i64).into();
    while (end - start) * is_desc >= zero {
        elems.push(start.into());
        start += step;
    }
    elems.into()
}

pub fn iter(init: Object, times: Number, func: Object) -> Object {
    let times = if let Some(x) = times.as_index() { x } else {
        return eval_err!("Cannot cast number to correct integer type")
    };
    if times == 0 { return Vec::new().into() }

    let mut elems = Vec::new();
    let mut work = init;
    for _ in 1..times {
        elems.push(work.clone());
        work = obj_call!(func(work));
    }
    elems.push(work);
    elems.into()
}

pub fn iter_while(init: Object, pred: Object, func: Object) -> Object {
    let mut elems = Vec::new();
    let mut work = init;
    while obj_call!(pred(work.clone()) => Bool).0 {
        elems.push(work.clone());
        work = obj_call!(func(work));
    }
    elems.into()
}


pub fn make_bltns() -> Object {
    let mut arr = HashMap::new();
    def_bltn!(arr.range(x: Number, y: Number) =
        range(x, y, (1 as i64).into()));
    def_bltn!(arr.range_step(x: Number, y: Number, step: Number) =
        range(x, y, step));

    def_bltn!(arr.iter(init: Object, times: Number, f: Object) =
        iter(init, times, f));

    def_bltn!(arr.iter_while(init: Object, pred: Object, f: Object) =
        iter_while(init, pred, f));

    def_bltn!(arr.fst(arr: Array) = {
        let mut arr = arr;
        if arr.0.len() >= 1 { arr.0.remove(0) } else {
            eval_err!("Array does not have a first element")
        }
    });
    def_bltn!(arr.snd(arr: Array) = {
        let mut arr = arr;
        if arr.0.len() >= 2 { arr.0.remove(1) } else {
            eval_err!("Array does not have a second element")
        }
    });

    def_getter!(arr.len);
    def_getter!(arr.sum);
    def_getter!(arr.prod);

    def_bltn!(arr.map(f: Object, obj: Object) = obj_call!(obj.map(f)));
    def_bltn!(arr.filter(f: Object, obj: Object) = obj_call!(obj.filter(f)));
    def_bltn!(arr.all(f: Object, obj: Object) = obj_call!(obj.all(f)));
    def_bltn!(arr.any(f: Object, obj: Object) = obj_call!(obj.any(f)));
    def_bltn!(arr.has(elm: Object, obj: Object) = obj_call!(obj.has(elm)));
    arr.into()
}
