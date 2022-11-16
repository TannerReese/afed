use std::collections::HashMap;
use std::fmt::{Display, Formatter, Error, Write};
use std::ops::{Index, IndexMut};

use super::bltn_func::BltnFunc;

use crate::expr::Bltn;
use crate::object::{
    Object, Operable,
    Unary, Binary,
    NamedType, EvalError,
};

#[derive(Debug, Clone)]
pub struct Plot {
    corner: (f64, f64),
    width: f64, height: f64,
    rows: usize, columns: usize,
    chars: Vec<char>,
}
impl NamedType for Plot { fn type_name() -> &'static str { "plot" }}

impl Operable for Plot {
    unary_not_impl!{}
    binary_not_impl!{}

    fn arity(&self, attr: Option<&str>) -> Option<usize> { match attr {
        Some("width") | Some("height") => Some(0),
        Some("center") => Some(0),
        Some("rows") | Some("cols") => Some(0),
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
        _ => panic!(),
    }}
}


impl Plot {
    pub fn new(
        corner: (f64, f64), width: f64, height: f64,
        rows: usize, columns: usize,
    ) -> Self { Plot {
        corner, width, height, rows, columns,
        chars: vec![' '; rows * columns],
    }}

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


    fn draw_vertical(&mut self, x: f64, symb: char) -> bool {
        if let Some(c) = self.x_to_col(x) {
            for r in 0..self.rows { self[(r, c)] = symb; }
            true
        } else { false }
    }

    fn draw_horizontal(&mut self, y: f64, symb: char) -> bool {
        if let Some(r) = self.y_to_row(y) {
            for c in 0..self.columns { self[(r, c)] = symb; }
            true
        } else { false }
    }

    fn draw_axes(&mut self) {
        self.draw_vertical(0.0, '$');
        self.draw_horizontal(0.0, '=');
        if self.contains((0.0, 0.0)) { self[(0.0, 0.0)] = '#'; }
    }

    pub fn good_cell_size(&self, divs: usize) -> (f64, f64) {
        let cell_size = |dim: f64| {
            let log_dim = (dim / (divs as f64)).log10();
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

    pub fn draw_gridlines(&mut self, (cell_wid, cell_hei): (f64, f64)) {
        let xs = linspace(
            (self.corner.0 / cell_wid).ceil() * cell_wid,
            self.corner.0 + self.width, cell_wid
        ).filter(|x| x.abs() > 1e-9);
        let ys = linspace(
            (self.corner.1 / cell_hei).floor() * cell_hei,
            self.corner.1 - self.height, -cell_hei
        ).filter(|y| y.abs() > 1e-9);

        for x in xs.clone() {
            self.draw_vertical(x, '|');
        }

        for y in ys {
            self.draw_horizontal(y, '-');
            for x in xs.clone() { self[(x, y)] = '+'; }
        }
        self.draw_axes();
    }

    pub fn plot<F, E>(&mut self, mut f: F) -> Result<(), E>
    where F: FnMut(f64) -> Result<f64, E> {
        let mut ny = f(self.col_to_x(0))?;
        for c in 0..self.columns {
            let y = ny;
            ny = f(self.col_to_x(c + 1))?;

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
        Ok(())
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
    pub fn make_plot(
        mut options: HashMap<String, Object>, funcs: Vec<Object>
    ) -> Object {
        macro_rules! get_params {
            ($($prm:ident : $tp:ty = $def:expr),*) => { $(
                let $prm: $tp;
                if let Some(x) = options.remove(stringify!($prm)) {
                    $prm = try_cast!(x);
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

            divs: usize = 5,
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

        let mut plot = Plot::new(
            (xmin, ymax), width, height, rows, cols
        );

        let cell_size = plot.good_cell_size(divs);
        plot.draw_gridlines(cell_size);

        for f in funcs {
            if let Err(err) = plot.plot(|x| {
                let res = obj_call!(f(x));
                if res.is_err() { Err(res) }
                else { res.cast() }
            }) { return err; }
        }
        plot.into()
    }
}


pub fn make_bltns() -> Bltn {
    let mut plt = HashMap::new();
    def_bltn!(plt.plot(
        options: HashMap<String, Object>, funcs: Vec<Object>
    ) = Plot::make_plot(options, funcs));
    Bltn::Map(plt)
}

