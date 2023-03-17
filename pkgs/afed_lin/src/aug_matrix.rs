use super::matrix::Matrix;
use afed_objects::{call, Object};

#[derive(Debug, Clone)]
pub struct AugMatrix {
    rows: usize,
    pub matrices: Vec<Matrix>,
    pub deter: Object,
}

impl Matrix {
    fn swap_rows(&mut self, r1: usize, r2: usize) {
        let cols = self.cols();
        let (r1, r2) = (r1 * cols, r2 * cols);
        for c in 0..cols {
            self.comps.swap(r1 + c, r2 + c);
        }
    }

    fn scale_row(&mut self, r: usize, scalar: &Object) {
        let cols = self.cols();
        let r = r * cols;
        for c in 0..self.cols() {
            self.comps[r + c] *= scalar.clone();
        }
    }

    fn add_rows(&mut self, src: usize, tgt: usize, scalar: &Object) {
        let cols = self.cols();
        let (src, tgt) = (src * cols, tgt * cols);
        for c in 0..cols {
            let mut elem = self.comps[src + c].clone();
            elem *= scalar.clone();
            self.comps[tgt + c] += elem;
        }
    }
}

impl AugMatrix {
    pub fn new(matrices: Vec<Matrix>) -> Self {
        let rows;
        if let Some(m) = matrices.get(0) {
            rows = m.rows();
            if matrices.iter().any(|m| m.rows() != rows) {
                panic!("All row dimensions must match in Augmented Matrix")
            }
        } else {
            panic!("Augmented Matrix must contain matrices")
        }
        AugMatrix {
            rows,
            matrices,
            deter: 1.into(),
        }
    }

    fn swap_rows(&mut self, r1: usize, r2: usize) {
        if r1 >= self.rows || r2 >= self.rows {
            panic!("Bad row indices {} and {}", r1, r2)
        } else if r1 == r2 {
            return;
        }

        for m in self.matrices.iter_mut() {
            m.swap_rows(r1, r2);
        }
        self.deter.do_inside(|x| -x);
    }

    fn scale_row(&mut self, r: usize, scalar: &Object) {
        if r >= self.rows {
            panic!("Bad row index {}", r)
        }

        for m in self.matrices.iter_mut() {
            m.scale_row(r, scalar);
        }
        self.deter *= scalar.clone();
    }

    fn add_rows(&mut self, src: usize, tgt: usize, scalar: &Object) {
        if src >= self.rows || tgt >= self.rows {
            panic!("Bad row indices {} and {}", src, tgt)
        }

        for m in self.matrices.iter_mut() {
            m.add_rows(src, tgt, scalar);
        }
    }

    pub fn gauss_elim(&mut self, target: usize) -> Result<(), Object> {
        if target >= self.matrices.len() {
            panic!("Bad matrix index {} for Augmented Matrix", target);
        }

        let rows = self.rows;
        let cols = self.matrices[target].cols();
        let mut pivot_row = 0;
        for c in 0..cols {
            let mat = &self.matrices[target];

            let mut inv = None;
            for r in pivot_row..rows {
                let elem = &mat[(r, c)];
                match call!(elem.has_inv()).cast() {
                    Ok(true) => {
                        inv = Some(call!(elem.inv()));
                        self.swap_rows(pivot_row, r);
                        break;
                    }
                    Ok(false) => {}
                    Err(err) => return Err(err),
                }
            }
            let inv = if let Some(i) = inv { i } else { continue };

            self.scale_row(pivot_row, &inv);
            for r in 0..rows {
                if r == pivot_row {
                    continue;
                }
                let elem = -self.matrices[target][(r, c)].clone();
                self.add_rows(pivot_row, r, &elem);
            }
            pivot_row += 1;
        }
        Ok(())
    }
}
