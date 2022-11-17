use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error, Write};
use std::ops::{Add, AddAssign, Index, IndexMut};

use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{
    Operable, Object, CastObject,
    Unary, Binary,
    NamedType, ErrObject, EvalError,
};

#[derive(Debug, Clone)]
pub struct Plot {
    corner: (f64, f64),
    width: f64, height: f64,
    rows: usize, columns: usize,
    chars: Vec<char>,

    errors: Vec<String>,
}
name_type!{plot: Plot}

impl Operable for Plot {
    unary_not_impl!{}

    fn try_binary(&self, _: bool, op: Binary, _: &Object) -> bool { match op {
        Binary::Add => true,
        _ => false,
    }}

    fn binary(
        self, _: bool, op: Binary, other: Object
    ) -> Object { match op {
        Binary::Add => (self + other).into(),
        _ => panic!(),
    }}

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        Some("width") | Some("height") => Some(0),
        Some("center") => Some(0),
        Some("rows") | Some("cols") => Some(0),
        Some("errors") => Some(0),
        _ => None,
    }}

    fn call(&self,
        attr: Option<&str>, _: Vec<Object>
    ) -> Object { match attr {
        Some("width") => self.width.into(),
        Some("height") => self.height.into(),
        Some("center") => vec![self.corner.0, self.corner.1].into(),

        Some("rows") => self.rows.into(),
        Some("cols") => self.columns.into(),
        Some("errors") => self.errors.clone().into(),
        _ => panic!(),
    }}
}


impl Plot {
    pub fn contains(&self, (x, y): (f64, f64)) -> bool {
        self.corner.0 <= x && x < self.corner.0 + self.width &&
        self.corner.1 >= y && y > self.corner.1 - self.height
    }

    fn x_to_col(&self, x: f64) -> Option<usize> {
        let x = (x - self.corner.0) / self.width;
        if x < 0.0 || 1.0 < x { None }
        else if x == 1.0 { Some(self.columns - 1) }
        else {
            Some((x * self.columns as f64).floor() as usize)
        }
    }

    fn y_to_row(&self, y: f64) -> Option<usize> {
        let y = (self.corner.1 - y) / self.height;
        if y < 0.0 || 1.0 < y { None }
        else if y == 1.0 { Some(self.rows - 1) }
        else {
            Some((y * self.rows as f64).floor() as usize)
        }
    }

    fn col_to_x(&self, c: usize) -> f64 {
        self.corner.0 + self.width * (c as f64) / (self.columns as f64)
    }

    fn row_to_y(&self, r: usize) -> f64 {
        self.corner.1 - self.height * (r as f64) / (self.rows as f64)
    }


    fn draw_char(&mut self, (x, y): (f64, f64), symb: char) -> bool {
        if let (Some(r), Some(c)) = (self.y_to_row(y), self.x_to_col(x)) {
            self[(r, c)] = symb;  true
        } else { false }
    }

    fn draw_vertical(&mut self, x: f64, symb: char) {
        if let Some(c) = self.x_to_col(x) {
            for r in 0..self.rows { self[(r, c)] = symb; }
        }
    }

    fn draw_horizontal(&mut self, y: f64, symb: char) {
        if let Some(r) = self.y_to_row(y) {
            for c in 0..self.columns { self[(r, c)] = symb; }
        }
    }

    fn draw_str(
        &mut self, (r, c): (usize, usize),
        string: &str, max_len: usize,
    ) {
        let max_len = std::cmp::min(max_len, string.len());
        let mut iter = string.chars();
        for i in 0..max_len {
            if i + c < self.columns {
                self[(r, i + c)] = iter.next().unwrap();
            }
        }
    }

    pub fn good_cell_size(&self, divs: f64) -> (f64, f64) {
        let cell_size = |dim: f64| {
            let log_dim = (dim / divs).log10();
            let n = log_dim.floor();
            let frac = log_dim - n;

            let two_log = (2.0 as f64).log10();
            let five_log = (5.0 as f64).log10();
            let log_dim = n + (
                if frac > five_log { five_log }
                else if frac > two_log { two_log }
                else { 0.0 }
            );
            (10.0 as f64).powf(log_dim)
        };
        (cell_size(self.width), cell_size(self.height))
    }

    pub fn draw_gridlines(&mut self,
        with_labels: bool, (cell_wid, cell_hei): (f64, f64)
    ) {
        let xs = linspace(
            (self.corner.0 / cell_wid).ceil() * cell_wid,
            self.corner.0 + self.width, cell_wid
        );
        let ys = linspace(
            (self.corner.1 / cell_hei).floor() * cell_hei,
            self.corner.1 - self.height, -cell_hei
        );

        for x in xs.clone() {
            self.draw_vertical(x, if x.abs() < 1e-9 { '$' } else { '|' });
        }

        for y in ys.clone() {
            self.draw_horizontal(y, if y.abs() < 1e-9 { '=' } else { '-' });
            for x in xs.clone() { self.draw_char((x, y),
                if x.abs() < 1e-9 && y.abs() < 1e-9 { '#' } else { '+' }
            );}
        }

        if with_labels {
            for x in xs { if let Some(c) = self.x_to_col(x) {
                let label = format!("{}", x);
                self.draw_str((0, c), label.as_str(), 6);
            }}

            for y in ys { if let Some(r) = self.y_to_row(y) {
                let label = format!("{}", y);
                self.draw_str((r, 0), label.as_str(), 6);
            }}
        }
    }
}



impl Plot {
    pub fn plot<F, E>(&mut self, mut f: F)
    where F: FnMut(f64) -> Result<f64, E> {
        let mut next_res = f(self.col_to_x(0)).ok();
        for c in 0..self.columns {
            let res = next_res;
            next_res = f(self.col_to_x(c + 1)).ok();
            let (y, ny) = if let (Some(y), Some(ny)) = (res, next_res) {
                (y, ny)
            } else { continue };

            let (miny, maxy) = if y < ny { (y, ny) } else { (ny, y) };
            if maxy <= self.corner.1 - self.height
            || miny >= self.corner.1 { continue }

            let minr = self.y_to_row(maxy);
            let maxr = self.y_to_row(miny);

            if let Some(r) = minr { if Some(r) == maxr {
                self[(r, c)] = '~';
                continue
            }}

            for r in minr.unwrap_or(0)..maxr.unwrap_or(self.rows) {
                self[(r, c)] = '!';
            }

            if let Some(r) = minr { self[(r, c)] = ','; }
            if let Some(r) = maxr { self[(r, c)] = '\''; }
        }
    }

    pub fn plot_impl<F, E>(&mut self, mut f: F)
    where F: FnMut(f64, f64) -> Result<f64, E> {
        #[derive(Clone, Copy)]
        enum S { P, N, Invalid }
        let (grid_rows, grid_cols) = (self.rows + 1, self.columns + 1);
        let mut states = vec![S::P; grid_rows * grid_cols];

        for r in 0..grid_rows { for c in 0..grid_cols {
            let idx = c + r * grid_cols;
            states[idx] = if let Ok(val) = f(
                self.col_to_x(c), self.row_to_y(r)
            ) { if val > 0.0 { S::P } else { S::N } }
            else { S::Invalid };
        }}

        for r in 0..self.rows { for c in 0..self.columns {
            let upper = c + r * grid_cols;
            let lower = upper + grid_cols;

            let symb = match (
                states[upper], states[upper + 1],
                states[lower + 1], states[lower],
            ) {
                (S::N, S::P, S::P, S::P) |
                (S::P, S::N, S::N, S::N) |
                (S::P, S::N, S::P, S::P) |
                (S::N, S::P, S::N, S::N) => '\'',

                (S::P, S::P, S::P, S::N) |
                (S::N, S::N, S::N, S::P) |
                (S::P, S::P, S::N, S::P) |
                (S::N, S::N, S::P, S::N) => ',',

                (S::P, S::P, S::N, S::N) |
                (S::N, S::N, S::P, S::P) => '~',

                (S::P, S::N, S::N, S::P) |
                (S::N, S::P, S::P, S::N) => '!',

                (S::N, S::P, S::N, S::P) |
                (S::P, S::N, S::P, S::N) => '+',
                _ => continue,
            };
            self[(r, c)] = symb;
        }}
    }
}



impl Display for Plot {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_char('\n')?;
        for r in 0..self.rows {
            for c in 0..self.columns {
                f.write_char(self[(r, c)])?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}


impl PartialEq for Plot {
    fn eq(&self, other: &Self) -> bool {
        self.corner == other.corner &&
        self.width == other.width &&
        self.height == other.height &&
        self.rows == other.rows &&
        self.columns == other.columns
    }
}

impl Eq for Plot {}

impl Index<(usize, usize)> for Plot {
    type Output = char;
    fn index(&self, (r, c): (usize, usize)) -> &char
        { &self.chars[c + r * self.columns] }
}

impl IndexMut<(usize, usize)> for Plot {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut char {
        let cols = self.columns;
        &mut self.chars[c + r * cols]
    }
}

impl Index<(f64, f64)> for Plot {
    type Output = char;
    fn index(&self, (x, y): (f64, f64)) -> &char {
        if let (Some(r), Some(c)) = (self.y_to_row(y), self.x_to_col(x)) {
            &self[(r, c)]
        } else { panic!("Point out of bounds ({}, {})", x, y) }
    }
}

impl IndexMut<(f64, f64)> for Plot {
    fn index_mut(&mut self, (x, y): (f64, f64)) -> &mut char {
        if let (Some(r), Some(c)) = (self.y_to_row(y), self.x_to_col(x)) {
            &mut self[(r, c)]
        } else { panic!("Point out of bounds ({}, {})", x, y) }
    }
}

impl From<Plot> for Object {
    fn from(p: Plot) -> Self { Object::new(p) }
}



#[derive(Debug, Clone, Copy)]
struct LinSpace {
    pos: f64,
    step: f64,
    end: f64,
}

fn linspace(start: f64, end: f64, step: f64) -> LinSpace {
    LinSpace { pos: start, step, end }
}

impl Iterator for LinSpace {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        let pos = self.pos;
        if (pos - self.end) * self.step >= 0.0 { None } else {
            self.pos += self.step;
            Some(pos)
        }
    }
}


impl Plot {
    pub fn new(mut options: HashMap<String, Object>) -> Object {
        macro_rules! get_params {
            ($($prm:ident : $tp:ty = $def:expr),*) => { $(
                let $prm: $tp;
                if let Some(x) = options.remove(stringify!($prm)) {
                    $prm = cast!(x);
                } else { $prm = $def; }
            )*};
        }

        get_params!(
            width: f64 = 2.0, height: f64 = 2.0,
            xcenter: f64 = 0.0, ycenter: f64 = 0.0,

            xmin: f64 = xcenter - width / 2.0,
            ymin: f64 = ycenter - height / 2.0,
            xmax: f64 = xmin + width,
            ymax: f64 = ymin + height,

            labels: bool = true,
            divs: f64 = 5.0,
            rows: usize = 40, cols: usize = 100
        );
        let keys: Vec<String> = options.into_keys().collect();
        if !keys.is_empty() { return eval_err!(
            "Unknown options '{:?}'", keys,
        )}

        let width = xmax - xmin;
        let height = ymax - ymin;
        if width <= 0.0 || height <= 0.0 { return eval_err!(
            "Width and Height of plot must be positive"
        )}

        let mut plot = Plot {
            corner: (xmin, ymax), width, height,
            rows, columns: cols,
            chars: vec![' '; rows * cols],
            errors: Vec::new(),
        };

        let cell_size = plot.good_cell_size(divs);
        plot.draw_gridlines(labels, cell_size);
        plot.into()
    }

    pub fn draw_obj(&mut self, obj: Object) { _ = match_cast!(obj,
        err: EvalError => self.errors.push(err.msg),
        pt => { self.draw_char(pt, 'O'); },
        elems: Vec<Object> => for obj in elems {
            self.draw_obj(obj);
        },
        obj: Object => match call!(obj.arity())
        .ok_or_err().and_then(|obj| obj.cast()) {
            Err(err) => self.errors.push(
                EvalError::cast(err).unwrap().msg
            ),
            Ok(1) => self.plot(|x|
                call!(obj(x)).ok_or_err()?.cast()
            ),
            Ok(2) => self.plot_impl(|x, y|
                call!(obj(x, y)).ok_or_err()?.cast()
            ),
            _ => {},
        }
    );}
}



impl AddAssign<Object> for Plot {
    fn add_assign(&mut self, rhs: Object) { self.draw_obj(rhs); }
}

impl Add<Object> for Plot {
    type Output = Plot;
    fn add(mut self, rhs: Object) -> Plot { self += rhs; self }
}

impl Add<Plot> for Object {
    type Output = Plot;
    fn add(self, mut rhs: Plot) -> Plot { rhs += self; rhs }
}



pub fn make_bltns() -> Bltn {
    let mut plt = HashMap::new();
    def_bltn!(plt.Plot(opts: HashMap<String, Object>) = Plot::new(opts));
    def_bltn!(plt.draw(obj: Object, plot: Plot) = (plot + obj).into());
    Bltn::Map(plt)
}

