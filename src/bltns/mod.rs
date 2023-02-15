use afed_objects::pkg::Pkg;
use std::collections::HashMap;

pub mod arr;
pub mod num;

pub mod modulo;
pub mod prs;

mod augmat;
pub mod mat;
pub mod vec;

// pub mod calc;
pub mod plot;

// Convert every member of a package into a global member
fn make_all_global(pkg: &mut Pkg) {
    if let Pkg::Map(map) = pkg {
        for (_, (is_global, _)) in map.iter_mut() {
            *is_global = true;
        }
    }
}

pub fn build_bltns() -> HashMap<String, Pkg> {
    let mut num = num::build_pkg();
    make_all_global(&mut num);
    let mut arr = arr::build_pkg();
    make_all_global(&mut arr);

    let prs = prs::build_pkg();
    let modulo = modulo::build_pkg();

    let vec = vec::build_pkg();
    let mat = mat::build_pkg();

    let plt: Pkg = plot::build_pkg();

    [
        ("num".into(), num),
        ("arr".into(), arr),
        ("prs".into(), prs),
        ("mod".into(), modulo),
        ("vec".into(), vec),
        ("mat".into(), mat),
        ("plt".into(), plt),
    ]
    .into()
}
