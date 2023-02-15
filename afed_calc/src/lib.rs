use std::cmp::Ordering;
use std::iter::{zip, Product};
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use rand::{
    distributions::{uniform::SampleUniform, Distribution, Uniform},
    rngs::ThreadRng,
    thread_rng,
};

use afed_objects::{call, declare_pkg, eval_err, Castable, ErrObject, Object};

macro_rules! dim_check {
    ($d1:expr, $d2:expr) => {
        let (d1, d2) = ($d1, $d2);
        if d1 != d2 {
            panic!("Dimension mismatch {} doesn't equal {}", d1, d2)
        }
    };
}

#[derive(Debug, Clone)]
pub struct Bounds<T>(Vec<(T, T)>);
pub type Point<T> = Vec<T>;

impl<T: PartialOrd> Bounds<T> {
    pub fn contains(&self, point: &Point<T>) -> bool {
        dim_check!(self.0.len(), point.len());
        zip(self.0.iter(), point.iter()).all(|((lower, upper), x)| *lower <= *x && *x <= *upper)
    }

    pub fn intersect(mut self, other: &Bounds<T>) -> Bounds<T>
    where
        T: Clone,
    {
        dim_check!(self.0.len(), other.0.len());
        for ((sl, su), (ol, ou)) in zip(self.0.iter_mut(), other.0.iter()) {
            if *sl < *ol {
                *sl = ol.clone();
            }
            if *su > *ou {
                *su = ou.clone();
            }
        }
        self
    }
}

impl<T: Clone + Sub<Output = T> + Product> Bounds<T> {
    pub fn volume(&self) -> T {
        self.0
            .iter()
            .cloned()
            .map(|(lower, upper)| upper - lower)
            .product()
    }
}

impl<
        T: Clone
            + From<u8>
            + PartialOrd
            + Add<Output = T>
            + Sub<Output = T>
            + Mul<Output = T>
            + Div<Output = T>,
    > Bounds<T>
{
    fn shrink(&mut self, center: Point<T>, scaling: T) {
        dim_check!(self.0.len(), center.len());
        if scaling <= 0.into() || scaling >= 1.into() {
            panic!("Shrink must decrease the size of the bounds")
        }

        for ((lower, upper), x) in zip(self.0.iter_mut(), center) {
            let width = upper.clone() - lower.clone();
            let shift = width * scaling.clone() / 2.into();

            let new_lower = x.clone() - shift.clone();
            if new_lower > *lower {
                *lower = new_lower;
            }
            let new_upper = x + shift;
            if new_upper < *upper {
                *upper = new_upper;
            }
        }
    }
}

impl<T: PartialOrd + Into<Object> + Castable> Castable for Bounds<T> {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        let mut is_single: bool = false;
        let mut bounds = match obj.try_cast() {
            Ok(pair) => {
                is_single = true;
                vec![pair]
            }
            Err(obj) => Vec::<(T, T)>::cast(obj)?,
        };

        for (lower, upper) in bounds.iter() {
            if lower >= upper {
                return Err((
                    if is_single {
                        bounds.remove(0).into()
                    } else {
                        bounds.into()
                    },
                    eval_err!("Lower bound must be less than upper bound"),
                ));
            }
        }
        Ok(Bounds(bounds))
    }
}

#[derive(Debug, Clone)]
pub struct GridSample<'a, T> {
    dims: usize,
    bounds: &'a Bounds<T>,
    step_sizes: Vec<T>,
    indices: Option<Point<T>>,
}

impl<'a, T> Iterator for GridSample<'a, T>
where
    T: Clone + PartialOrd + AddAssign,
{
    type Item = Point<T>;
    fn next(&mut self) -> Option<Point<T>> {
        let indices = std::mem::take(&mut self.indices);
        let pt = indices.clone();

        if let Some(mut indices) = indices {
            for i in 0..self.dims {
                indices[i] += self.step_sizes[i].clone();
                if self.bounds.contains(&indices) {
                    self.indices = Some(indices);
                    return pt;
                } else {
                    indices[i] = self.bounds.0[i].0.clone();
                }
            }
        }
        pt
    }
}

impl<T: Clone + From<u32> + Sub<Output = T> + Div<Output = T>> Bounds<T> {
    pub fn grid(&self, grid_size: u32) -> GridSample<T> {
        let dims = self.0.len();
        let indices = Some(self.0.iter().map(|(lower, _)| lower.clone()).collect());
        let step_sizes = self
            .0
            .iter()
            .cloned()
            .map(|(lower, upper)| (upper - lower) / grid_size.into())
            .collect();
        GridSample {
            dims,
            indices,
            step_sizes,
            bounds: self,
        }
    }
}

pub struct RandSample<T: SampleUniform> {
    count: Option<usize>,
    rng: ThreadRng,
    geners: Vec<Uniform<T>>,
}

impl<T: Clone + SampleUniform> Iterator for RandSample<T> {
    type Item = Point<T>;
    fn next(&mut self) -> Option<Point<T>> {
        if let Some(count) = self.count {
            if count == 0 {
                return None;
            }
            self.count = Some(count - 1)
        }

        let mut pt = Vec::new();
        for i in 0..self.geners.len() {
            pt.push(self.geners[i].sample(&mut self.rng));
        }
        Some(pt)
    }
}

impl<T: Clone + SampleUniform> Bounds<T> {
    pub fn rand(&self, count: Option<usize>) -> RandSample<T> {
        let geners = self
            .0
            .iter()
            .map(|(lower, upper)| Uniform::from(lower.clone()..upper.clone()))
            .collect();
        RandSample {
            count,
            rng: thread_rng(),
            geners,
        }
    }
}

pub fn integral<I>(sample: I, volume: f64, f: Object) -> Object
where
    I: Iterator<Item = Vec<f64>>,
{
    let mut count: u32 = 0;
    sample
        .map(|pt| {
            count += 1;
            let pt = pt.into_iter().map(Object::from).collect();
            f.call(None, pt)
        })
        .sum::<Object>()
        * Object::from(volume / count as f64)
}

pub fn extremum_grid(
    count: usize,
    direc: Ordering,
    mut bnds: Bounds<f64>,
    f: Object,
) -> Result<(Vec<f64>, Object), Object> {
    if count == 0 {
        return Err(eval_err!(
            "Must perform at least one iteration to find extremum"
        ));
    }

    const GRID_SIZE: u32 = 10;
    const SMALLEST_BOUND: f64 = (GRID_SIZE as f64) * f64::EPSILON;
    let (mut max_pt, mut max_val) = (None, None);
    for _ in 0..count {
        if bnds
            .0
            .iter()
            .any(|(lower, upper)| (lower - upper).abs() < SMALLEST_BOUND)
        {
            break;
        }

        for pt in bnds.grid(GRID_SIZE) {
            let args = pt.iter().cloned().map(Object::from).collect();
            let val = f.call(None, args);
            if val.is_err() {
                return Err(val);
            }

            if let Some(max_val) = &max_val {
                let comp = val.partial_cmp(max_val);
                if comp.is_none() {
                    return Err(eval_err!("Incomparable return values"));
                } else if comp != Some(direc) {
                    continue;
                }
            }
            max_val = Some(val);
            max_pt = Some(pt);
        }

        let scaling = 2.0 / f64::from(GRID_SIZE);
        bnds.shrink(max_pt.clone().unwrap(), scaling);
    }
    Ok((max_pt.unwrap(), max_val.unwrap()))
}

fn pt_to_obj<T: Into<Object>>(mut pt: Point<T>) -> Object {
    if pt.is_empty() {
        eval_err!("Point cannot have zero dimension")
    } else if pt.len() == 1 {
        pt.remove(0).into()
    } else {
        pt.into()
    }
}

const EXTR_ITERS: usize = 30;

declare_pkg! {calc:
    /// calc.integ_grid (size: natural) (bounds: array of [number, number]) (f: n-ary function) -> any
    /// 'bounds' is an array with 'n' pairs (lower, upper).
    /// 'f' must take 'n' numbers as arguments and return a scalable and summable value.
    /// Integrates 'f' over the region defined by 'bounds'
    /// by evaluating 'f' at the points of an 'n'-dimensional cubic grid
    /// with 'size' grid points along one side.
    fn integ_grid(size: u32, bnds: Bounds<f64>, f: Object) -> Object {
        let vol = bnds.volume();
        let grid = bnds.grid(size);
        integral(grid, vol, f)
    }

    /// calc.integ_rand (count: natural) (bounds: array of [number, number]) (f: n-ary function) -> any
    /// 'bounds' is an array with 'n' pairs (lower, upper).
    /// 'f' must take 'n' numbers as arguments and return a scalable and summable value.
    /// Integrates 'f' over the region defined by 'bounds'
    /// by evaluating 'f' at 'count' randomly chosen points in 'boudns'.
    fn integ_rand(count: usize, bnds: Bounds<f64>, f: Object) -> Object {
        let vol = bnds.volume();
        let rnd = bnds.rand(Some(count));
        integral(rnd, vol, f)
    }



    /// calc.deriv (f: any -> any) (x: any) -> any
    /// Differentiate 'f' at 'x' by comparing the value of
    /// 'f (x - h)' and 'f (x + h)' where 'h' is a very small real number.
    /// So 'x' must be able to be added to a real number.
    fn deriv(f: Object, x: Object) -> Object { direc_deriv(1.into(), f, x) }

    /// calc.direc_deriv (direc: any) (f: any -> any) (x: any) -> any
    /// Differentiate 'f' at 'x' in the direction of 'direc'
    /// by comparing the value of 'f (x - h * direc)' and 'f (x + h * direc)'
    /// where 'h' is a very small real number.
    /// So 'direc' must be able to be multiplied by a
    /// real number and added to 'x'.
    fn direc_deriv(direc: Object, f: Object, x: Object) -> Object {
        let h = f64::EPSILON.cbrt();
        let px = x.clone() - direc.clone() * Object::from(h);
        let nx = x + direc * Object::from(h);
        (call!(f(nx)) - call!(f(px))) / Object::from(2.0 * h)
    }



    /// calc.max (bounds: array of [number, number]) (f: n-ary function) -> any
    /// Same arguments as 'calc.max_pt'.
    /// Returns only the value of the maximum.
    fn max(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Greater, bnds, f) {
            Err(err) => err,
            Ok((_, val)) => val,
        }
    }

    /// calc.argmax (bounds: array of [number, number]) (f: n-ary function) -> point
    /// Same arguments as 'calc.max_pt'.
    /// Returns only the point that achieves the maximum.
    fn argmax(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Greater, bnds, f) {
            Err(err) => err,
            Ok((pt, _)) => pt_to_obj(pt),
        }
    }

    /// calc.max_pt (bounds: array of [number, number]) (f: n-ary function) -> (point, any)
    /// 'bounds' is an array with 'n' pairs (lower, upper).
    /// 'f' must take 'n' numbers as arguments and return an orderable value.
    /// Find the point with the maximum value of 'f'
    /// whose coordinates lie within the bounds given by 'bounds'.
    /// Both the point and value of the maximum are returned as a pair.
    fn max_pt(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Greater, bnds, f) {
            Err(err) => err,
            Ok((pt, val)) => vec![pt_to_obj(pt), val].into(),
        }
    }



    /// calc.min (bounds: array of [number, number]) (f: n-ary function) -> any
    /// Same arguments as 'calc.min_pt'.
    /// Returns only the value of the minimum.
    fn min(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Less, bnds, f) {
            Err(err) => err,
            Ok((_, val)) => val,
        }
    }

    /// calc.argmin (bounds: array of [number, number]) (f: n-ary function) -> point
    /// Same arguments as 'calc.min_pt'.
    /// Returns only the point that achieves the minimum.
    fn argmin(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Less, bnds, f) {
            Err(err) => err,
            Ok((pt, _)) => pt_to_obj(pt),
        }
    }

    /// calc.min_pt (bounds: array of [number, number]) (f: n-ary function) -> (point, any)
    /// 'bounds' is an array with 'n' pairs (lower, upper).
    /// 'f' must take 'n' numbers as arguments and return an orderable value.
    /// Find the point with the minimum value of 'f'
    /// whose coordinates lie within the bounds given by 'bounds'.
    /// Both the point and value of the minimum are returned as a pair.
    fn min_pt(bnds: Bounds<f64>, f: Object) -> Object {
        match extremum_grid(EXTR_ITERS, Ordering::Less, bnds, f) {
            Err(err) => err,
            Ok((pt, val)) => vec![pt_to_obj(pt), val].into(),
        }
    }
}
