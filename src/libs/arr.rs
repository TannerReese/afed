use std::collections::HashMap;

use super::bltn_func::BltnFunc;

use crate::object::{Object, EvalError};
use crate::object::number::Number;
use crate::object::array::Array;

pub fn range(mut start: Number, end: Number, step: Number) -> Object {
    let zero = 0.into();
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
    let is_desc = if is_desc { -1 } else { 1 }.into();
    while (end - start) * is_desc >= zero {
        elems.push(start.into());
        start += step;
    }
    elems.into()
}

pub fn iter(init: Object, times: usize, func: Object) -> Object {
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
    while obj_call!(pred(work.clone()) => bool) {
        elems.push(work.clone());
        work = obj_call!(func(work));
    }
    elems.into()
}


pub fn make_bltns() -> Object {
    let mut arr = HashMap::new();
    def_bltn!(arr.range(x: Number, y: Number) = range(x, y, 1.into()));
    def_bltn!(arr.range_step(x: Number, y: Number, step: Number) =
        range(x, y, step));

    def_bltn!(arr.iter(init: Object, times: usize, f: Object) =
        iter(init, times, f));

    def_bltn!(arr.iter_while(init: Object, pred: Object, f: Object) =
        iter_while(init, pred, f));

    def_bltn!(arr.fst(vc: Vec<Object>) = {
        let mut vc = vc;
        if vc.len() >= 1 { vc.remove(0) } else {
            eval_err!("Array does not have a first element")
        }
    });
    def_bltn!(arr.snd(vc: Vec<Object>) = {
        let mut vc = vc;
        if vc.len() >= 2 { vc.remove(1) } else {
            eval_err!("Array does not have a second element")
        }
    });
   def_bltn!(arr.last(vc: Vec<Object>) = {
        let mut vc = vc;
        if let Some(elem) = vc.pop() { elem } else {
            eval_err!("Array does not have a last element")
        }
    });


    def_getter!(arr.len);
    def_getter!(arr.sum);
    def_getter!(arr.prod);
    def_getter!(arr.max);
    def_getter!(arr.min);
    def_getter!(arr.rev);

    def_bltn!(arr.map(f: Object, obj: Object) = obj_call!(obj.map(f)));
    def_bltn!(arr.filter(f: Object, obj: Object) = obj_call!(obj.filter(f)));
    def_bltn!(arr.fold(init: Object, f: Object, a: Array) = a.fold(init, f));
    def_bltn!(arr.all(f: Object, a: Array) = a.all(f));
    def_bltn!(arr.any(f: Object, a: Array) = a.any(f));
    def_bltn!(arr.has(elm: Object, obj: Object) = obj_call!(obj.has(elm)));
    arr.into()
}

