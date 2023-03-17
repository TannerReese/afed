use std::collections::HashMap;
use std::cmp::Ordering;

use afed_objects::{call, declare_pkg, ErrObject, Object};

use calc::{Bounds, pt_to_obj, integral, extremum_grid};
use plot::Plot;

pub mod calc;
pub mod plot;

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



    /// calc.Plot (options: map) -> plot
    /// Construct a plot using 'options'
    /// Options:
    ///   xmax: real -- X-coordinate of right side of the viewport
    ///   xmin: real -- X-coordinate of left side of the viewport
    ///   ymax: real -- Y-coordinate of top of the viewport
    ///   ymin: real -- Y-coordinate of bottom of the viewport
    ///   width: positive real -- Width of the the viewport (default: 2.0)
    ///   height: positive real -- Height of the the viewport (default: 2.0)
    ///   xcenter: real -- X-coordinate of center of the viewport (default: 0.0)
    ///   ycenter: real -- Y-coordinate of center of the viewport (default: 0.0)
    ///
    ///   rows: natural -- Number of rows of characters in grid (default: 40)
    ///   cols: natural -- Number of columns of characters in grid (default: 100)
    ///   labels: bool -- Whether gridlines should be labelled (default: true)
    ///   divs: positive real -- Approximate number of gridlines
    ///       vertically and horizontally across the window (default: 5.0)
    ///
    /// Example:
    ///     calc.Plot {
    ///         rows: 50, cols: 80,
    ///         divs: 7, labels: false,
    ///         width: 2.1, height: 3,
    ///         xmin: -0.5, ymax: 0,
    ///     }
    #[allow(non_snake_case)]
    fn Plot(opts: HashMap<String, Object>) -> Result<Plot, ErrObject>
        { Plot::new(opts) }

    /// calc.draw (obj: any) (p: plot) -> plot
    /// Draw the object 'obj' onto 'p' and return 'p'
    fn draw(obj: Object, plot: Plot) -> Plot { plot + obj }
}
