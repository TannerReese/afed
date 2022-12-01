use std::collections::HashMap;
use std::iter::zip;

use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{Object, EvalError, ErrObject};
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
        elems.push(start);
        start += step;
    }
    elems.into()
}

pub fn iter(init: Object, times: usize, func: Object) -> Object {
    if times == 0 { return Vec::<Object>::new().into() }

    let mut elems = Vec::new();
    let mut work = init;
    for _ in 1..times {
        elems.push(work.clone());
        work = call!(func(work));
    }
    elems.push(work);
    elems.into()
}

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


pub fn make_bltns() -> Bltn {
    let mut arr = HashMap::new();
    def_bltn!(arr.range(x: Number, y: Number) = range(x, y, 1.into()));
    def_bltn!(arr.range_step(x: Number, y: Number, step: Number) =
        range(x, y, step));

    def_bltn!(arr.iter(init, times: usize, f) = iter(init, times, f));
    def_bltn!(arr.iter_while(init, pred, f) =
        iter_while(init, pred, f).into());

    def_bltn!(arr.zip(v1: Vec<Object>, v2: Vec<Object>) =
        zip(v1, v2).collect());
    def_bltn!(arr.zip_with(f, v1: Vec<Object>, v2: Vec<Object>) =
        zip(v1, v2).map(|(x, y)| call!(f(x, y))).collect());

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

    def_bltn!(arr.map(f, a: Array) = a.map(f).into());
    def_bltn!(arr.filter(f, a: Array) = a.filter(f).into());
    def_bltn!(arr.fold(init, f, a: Array) = a.fold(init, f).into());
    def_bltn!(arr.all(f, a: Array) = a.all(f).into());
    def_bltn!(arr.any(f, a: Array) = a.any(f).into());
    def_bltn!(arr.has(elm, a: Array) = a.has(elm).into());
    Bltn::Map(arr)
}

