use super::expr::Bltn;

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

create_bltns!{ mod {
    #[global] num: num::make_bltns(),
    #[global] arr: arr::make_bltns(),

    prs: prs::make_bltns(),
    mod: modulo::make_bltns(),

    vec: vec::make_bltns(),
    mat: mat::make_bltns(),

    calc: calc::make_bltns(),
    plt: plot::make_bltns(),
}}

