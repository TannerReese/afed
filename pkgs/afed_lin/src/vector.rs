use std::convert::TryInto;
use std::fmt::{Display, Error, Formatter, Write};
use std::iter::{zip, FromIterator};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

use super::matrix::Matrix;

use afed_objects::{call, impl_operable, name_type, Object};

macro_rules! check_dims_panic {
    ($a:expr, $b:expr) => {
        let (adims, bdims) = ($a.dims(), $b.dims());
        if adims != bdims {
            panic!("Dimension {} and {} don't match", adims, bdims)
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vector(Vec<Object>);
name_type! {vector: Vector}

impl_operable! {Vector:
    //! Vector of any dimension containing heterogeneous components.

    /// -vector -> vector
    /// Negation of vectors
    #[unary(Neg)] fn _(v: Self) -> Self { -v }
    /// vector + vector -> vector
    /// Addition of vectors
    #[binary(Add)] fn _(v1: Self, v2: Self) -> Result<Self, String>
        { Self::check_dims(v1, v2).map(|(v1, v2)| v1 + v2) }
    /// vector - vector -> vector
    /// Subtraction of vectors
    #[binary(Sub)] fn _(v1: Self, v2: Self) -> Result<Self, String>
        { Self::check_dims(v1, v2).map(|(v1, v2)| v1 - v2) }
    /// vector * vector -> any
    /// Dot product of vectors
    #[binary(Mul)] fn _(v1: Self, v2: Self) -> Result<Object, String>
        { Self::check_dims(v1, v2).map(|(v1, v2)| v1 * v2) }

    /// (scalar: any) * vector -> vector
    /// vector * (scalar: any) -> vector
    /// Scale vector by 'scalar' (which cannot be a matrix)
    #[binary(Mul)]
    #[exclude(Matrix)]
    fn _(v: Self, scalar: Object) -> Self { v * scalar }

    #[binary(rev, Mul)]
    #[exclude(Matrix)]
    fn _(v: Self, scalar: Object) -> Self { scalar * v }

    /// vector / (scalar: any) -> vector
    /// Scale vector by inverse of 'scalar' (which cannot be a matrix)
    #[binary(Div)]
    #[exclude(Matrix)]
    fn _(v: Self, scalar: Object) -> Self { v / scalar }

    /// vector % (mod: any) -> vector
    /// Reduce each component of 'vector' with modulo 'mod'
    #[binary(Mod)]
    #[exclude(Matrix)]
    fn _(v: Self, scalar: Object) -> Self { v % scalar }

    /// vector // (divisor: any) -> vector
    /// Floor divide each component of 'vector' by 'divisor'
    #[binary(FlrDiv)]
    #[exclude(Matrix)]
    fn _(v: Self, scalar: Object) -> Self { v.flrdiv(scalar) }

    /// vector (i: natural) -> any
    /// Get the 'i'th component of 'vector'
    #[call]
    fn __call(&self, idx: usize) -> Result<Object, String> {
        if let Some(obj) = self.0.get(idx) { Ok(obj.clone()) }
        else { Err(format!(
            "Index {} is larger or equal to dimension {}", idx, self.dims()
        ))}
    }

    /// vector.dims -> natural
    /// Dimension of 'vector' which is the number of components
    pub fn dims(&self) -> usize { self.0.len() }
    /// vector.comps -> array
    /// Array containing the components of 'vector'
    pub fn comps(self) -> Vec<Object> { self.0 }

    /// vector.mag2 -> any
    /// Dot product of 'vector' with itself
    /// which is the squared magnitude of 'vector'
    pub fn mag2(self) -> Object
        { self.0.into_iter().map(|x| x.clone() * x).sum() }
    /// vector.mag -> any
    /// Square root of the dot product of 'vector' with itself.
    /// Requires dot product to have 'sqrt' method
    pub fn mag(self) -> Object
        { call!((self.mag2()).sqrt()) }
}

impl Vector {
    pub fn new(comps: Vec<Object>) -> Self {
        Self(comps)
    }

    fn check_dims(v1: Self, v2: Self) -> Result<(Self, Self), String> {
        if v1.dims() == v2.dims() {
            Ok((v1, v2))
        } else {
            Err(format!(
                "Vector dimensions {} and {} do not match",
                v1.dims(),
                v2.dims(),
            ))
        }
    }

    pub fn check_errs(self) -> Result<Self, Object> {
        if self.0.iter().any(|c| c.is_err()) {
            Err(self.0.into_iter().find(|c| c.is_err()).unwrap())
        } else {
            Ok(self)
        }
    }

    pub fn flrdiv_assign(&mut self, rhs: Object) {
        self.0
            .iter_mut()
            .for_each(|r| r.do_inside(|x| x.flrdiv(rhs.clone())));
    }
    pub fn flrdiv(mut self, rhs: Object) -> Self {
        self.flrdiv_assign(rhs);
        self
    }
}

impl Neg for Vector {
    type Output = Self;
    fn neg(mut self) -> Self {
        self.0.iter_mut().for_each(|a| a.do_inside(|x| -x));
        self
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        check_dims_panic!(self, rhs);
        zip(self.0.iter_mut(), rhs.0).for_each(|(a, b)| *a += b);
    }
}

impl Add for Vector {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        check_dims_panic!(self, rhs);
        zip(self.0.iter_mut(), rhs.0).for_each(|(a, b)| *a -= b);
    }
}

impl Sub for Vector {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl Mul<Vector> for Vector {
    type Output = Object;
    fn mul(self, rhs: Vector) -> Object {
        check_dims_panic!(self, rhs);
        zip(self.0, rhs.0).map(|(a, b)| a * b).sum()
    }
}

impl Mul<Vector> for Object {
    type Output = Vector;
    fn mul(self, mut rhs: Vector) -> Vector {
        rhs.0
            .iter_mut()
            .for_each(|r| r.do_inside(|x| self.clone() * x));
        rhs
    }
}

impl MulAssign<Object> for Vector {
    fn mul_assign(&mut self, rhs: Object) {
        self.0.iter_mut().for_each(|x| *x *= rhs.clone());
    }
}

impl Mul<Object> for Vector {
    type Output = Vector;
    fn mul(mut self, rhs: Object) -> Self {
        self *= rhs;
        self
    }
}

impl DivAssign<Object> for Vector {
    fn div_assign(&mut self, rhs: Object) {
        self.0.iter_mut().for_each(|x| *x /= rhs.clone());
    }
}

impl Div<Object> for Vector {
    type Output = Vector;
    fn div(mut self, rhs: Object) -> Self {
        self /= rhs;
        self
    }
}

impl RemAssign<Object> for Vector {
    fn rem_assign(&mut self, rhs: Object) {
        self.0.iter_mut().for_each(|x| *x %= rhs.clone());
    }
}

impl Rem<Object> for Vector {
    type Output = Vector;
    fn rem(mut self, rhs: Object) -> Self::Output {
        self %= rhs;
        self
    }
}

impl Display for Vector {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("V[")?;
        let mut is_first = true;
        for obj in self.0.iter() {
            if !is_first {
                f.write_str(", ")?;
            }
            is_first = false;
            write!(f, "{}", obj)?;
        }
        f.write_char(']')
    }
}

impl FromIterator<Object> for Vector {
    fn from_iter<T: IntoIterator<Item = Object>>(iter: T) -> Self {
        Vector(Vec::from_iter(iter))
    }
}

impl From<Vector> for Object {
    fn from(v: Vector) -> Self {
        if v.0.iter().any(|x| x.is_err()) {
            v.0.into_iter().find(|x| x.is_err()).unwrap()
        } else {
            Object::new(v)
        }
    }
}
