use super::expr::Bltn;

pub mod bltn_func;

pub mod arr;
pub mod num;

pub mod modulo;
pub mod prs;

mod augmat;
pub mod mat;
pub mod vec;

pub mod calc;
pub mod plot;

create_bltns! { mod {
    #[global] num: num::make_bltns(),
    #[global] arr: arr::make_bltns(),

    prs: prs::make_bltns(),
    mod: modulo::make_bltns(),

    vec: vec::make_bltns(),
    mat: mat::make_bltns(),

    calc: calc::make_bltns(),
    plt: plot::make_bltns(),
}}
