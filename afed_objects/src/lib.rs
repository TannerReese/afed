use std::clone::Clone;
use std::fmt::{Debug, Display, Error, Formatter, Write};
use std::mem::take;
use std::vec::Vec;

use std::cmp::Ordering;
use std::iter::{Product, Sum};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

use std::borrow::Borrow;
use std::hash::Hash;

use self::bool::Bool;
pub use self::opers::{Assoc, Binary, Unary};
use self::partial_eval::PartialEval;

macro_rules! try_ok {
    ($obj:expr) => {{
        let obj = $obj;
        if obj.is_err() {
            return obj;
        } else {
            obj
        }
    }};
}

mod opers;

#[macro_use]
pub mod macros;
pub mod testing;

pub mod array;
pub mod bool;
pub mod error;
pub mod map;
pub mod null;
pub mod number;
pub mod partial_eval;
pub mod pkg;
pub mod string;

// Allows afed to check that afed_objects version match across dynamic link boundaries
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub trait Operable {
    fn unary(self, op: Unary) -> Option<Object>;
    fn binary(self, rev: bool, op: Binary, other: Object) -> Result<Object, (Object, Object)>;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
    fn help(&self, attr: Option<&str>) -> Option<String>;
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object;
}

/* WARNING: Don't try to replace this with `Any` and `TypeId`
 * The way TypeId's are assigned they can differ between different
 * compilations of the same file causing issues with dynamic linking.
 * To avoid this, the `name_type!` macro assigns IDs in a way that
 * is consistent for a given file.
 */
pub trait NamedType {
    fn type_name() -> &'static str;
    fn type_id() -> NamedTypeId;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NamedTypeId(u32);

const fn hash_str(mut state: u64, s: &'static str) -> u64 {
    let mod_prime: u64 = 0xfffffffb;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        state = ((state + bytes[i] as u64) << 4) % mod_prime;
        i += 1;
    }
    state
}

impl NamedTypeId {
    pub const fn _from_context(
        _type: &'static str,
        typename: &'static str,
        line: u32,
        column: u32,
    ) -> Self {
        let mod_prime: u64 = 0xfffffffb;
        let mut id = (line as u64) << 4;
        id = ((id + column as u64) << 4) % mod_prime;
        id = hash_str(id, _type);
        id = hash_str(id, typename);
        Self(id as u32)
    }
}

/* An `Object` is a pointer to any of a number of dynamic types.
 * Types implementing `Objectish` are those to which `Object` can point.
 * An `Objectish` type can be cast to an `Object` by "forgetting" its type.
 */
pub trait Objectish: Eq + Clone + NamedType + Debug + Display + Operable {}

impl<T> Objectish for T where T: Eq + Clone + NamedType + Debug + Display + Operable {}

/* The two layer structure of `ObjectishSafe` and `Objectish` is necessary
 * because `Objectish` requires a type to have object-unsafe methods.
 *
 * Specifically, any methods which are pass-by-value (i.e. with `self`)
 * or have an argument of type `Self` require the type to be `Sized`.
 * However, the value behind a dynamic pointer can't be `Sized`.
 *
 * To get around this, `ObjectishSafe` is a dummy trait which has only
 * pass-by-reference methods.  These are implemented by storing the
 * `Objectish` type as an `Option` on the dynamic type.  The dynamic value
 * can then be "stolen" out of the `Option` to perform the unsafe operations.
 */
trait ObjectishSafe {
    fn as_dummy(&self) -> &dyn DummySafe;
    fn as_dummy_mut(&mut self) -> &mut dyn DummySafe;
    fn type_name_dyn(&self) -> &'static str;
    fn type_id_dyn(&self) -> NamedTypeId;

    fn clone(&self) -> Object;
    fn eq(&self, other: &Object) -> bool;

    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;

    fn unary(&mut self, op: Unary) -> Option<Object>;
    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Result<Object, (Object, Object)>;

    fn arity(&self, attr: Option<&str>) -> Option<usize>;
    fn help(&self, attr: Option<&str>) -> Option<String>;
    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object;
}

impl<T: Objectish + 'static> ObjectishSafe for Option<T> {
    fn as_dummy(&self) -> &dyn DummySafe {
        self
    }

    fn as_dummy_mut(&mut self) -> &mut dyn DummySafe {
        self
    }

    fn type_name_dyn(&self) -> &'static str {
        T::type_name()
    }

    fn type_id_dyn(&self) -> NamedTypeId {
        T::type_id()
    }

    fn clone(&self) -> Object {
        Object::new(self.as_ref().unwrap().clone())
    }

    fn eq(&self, other: &Object) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self.as_ref().unwrap() == other
        } else {
            false
        }
    }

    fn display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Display::fmt(self.as_ref().unwrap(), f)
    }

    fn debug_fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Debug::fmt(self.as_ref().unwrap(), f)
    }

    fn unary(&mut self, op: Unary) -> Option<Object> {
        take(self).unwrap().unary(op)
    }

    fn binary(&mut self, rev: bool, op: Binary, other: Object) -> Result<Object, (Object, Object)> {
        take(self).unwrap().binary(rev, op, other)
    }

    fn arity(&self, attr: Option<&str>) -> Option<usize> {
        self.as_ref().unwrap().arity(attr)
    }

    fn help(&self, attr: Option<&str>) -> Option<String> {
        self.as_ref().unwrap().help(attr)
    }

    fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object {
        self.as_ref().unwrap().call(attr, args)
    }
}

pub struct Object(Box<dyn ObjectishSafe>);
pub type ErrObject = Object;

// Helper trait for casting to an arbitrary NamedType
trait DummySafe {}
impl<T: NamedType> DummySafe for Option<T> {}

impl Object {
    fn downcast_ref<T: NamedType>(&self) -> Option<&T> {
        if T::type_id() == (*self.0).type_id_dyn() {
            let dummy_ptr = (*self.0).as_dummy() as *const dyn DummySafe;
            unsafe { (*(dummy_ptr as *const Option<T>)).as_ref() }
        } else {
            None
        }
    }

    fn downcast_into<T: NamedType>(mut self) -> Result<T, Self> {
        if T::type_id() == (*self.0).type_id_dyn() {
            let dummy_ptr = (*self.0).as_dummy_mut() as *mut dyn DummySafe;
            unsafe { Ok(take(&mut *(dummy_ptr as *mut Option<T>)).unwrap()) }
        } else {
            Err(self)
        }
    }
}

impl Object {
    pub fn new<T: Objectish + 'static>(obj: T) -> Object
    where
        T: Objectish,
    {
        Object(Box::new(Some(obj)))
    }

    pub fn is_err(&self) -> bool {
        self.is_a::<error::EvalError>()
    }

    pub fn ok_or_err(self) -> Result<Object, ErrObject> {
        if self.is_err() {
            Err(self)
        } else {
            Ok(self)
        }
    }

    pub fn type_id(&self) -> NamedTypeId {
        (*self.0).type_id_dyn()
    }

    pub fn is_a<T: NamedType>(&self) -> bool {
        T::type_id() == self.type_id()
    }

    pub fn do_inside(&mut self, func: impl FnOnce(Self) -> Self) {
        let owned = Object(std::mem::replace(&mut self.0, Box::new(None::<null::Null>)));
        self.0 = func(owned).0;
    }

    pub fn cast<T: Castable>(self) -> Result<T, ErrObject> {
        T::cast(self).map_err(|(_, err)| err)
    }

    pub fn try_cast<T: Castable>(self) -> Result<T, Object> {
        T::cast(self).map_err(|(obj, _)| obj)
    }

    pub fn cast_with_err<T: Castable>(self) -> Result<T, (Object, ErrObject)> {
        T::cast(self)
    }

    pub fn get<B>(&self, key: &B) -> Option<&Object>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    {
        self.downcast_ref::<map::Map>().and_then(|m| m.get(key))
    }

    pub fn find<'a, I, B>(&self, path: I) -> Option<&Object>
    where
        I: Iterator<Item = &'a B>,
        B: Hash + Eq + 'a,
        String: Borrow<B>,
    {
        let mut target = self;
        for nm in path {
            if let Some(new_target) = target.get(nm) {
                target = new_target;
            } else {
                return None;
            }
        }
        Some(target)
    }
}

/* WARNING: Don't try to replace this with `TryFrom<Object>`
 * Because of the orphan rule, it isn't possible to implement such a `TryFrom`
 * on genericized types like `Vec<T>` or `(A, B)`.  Since generic conversions
 * between these types and `Object` are so crucial for ergonomic library
 * code, it is important to keep this trait defined inside the crate.
 */
pub trait Castable: Sized {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)>;
}

impl Castable for Object {
    fn cast(obj: Object) -> Result<Self, (Object, ErrObject)> {
        Ok(obj)
    }
}

impl<T> Castable for T
where
    T: Objectish,
{
    fn cast(obj: Object) -> Result<T, (Object, ErrObject)> {
        let given_typename = (*obj.0).type_name_dyn();
        obj.downcast_into::<T>().map_err(|obj| {
            (
                obj,
                eval_err!("Expected {}, but found {}", T::type_name(), given_typename),
            )
        })
    }
}

impl Object {
    pub fn unary(mut self, op: Unary) -> Object {
        if let Some(obj) = (*self.0).unary(op) {
            obj
        } else {
            eval_err!(
                "Unary operator {} not implemented for type {}",
                op.symbol(),
                (*self.0).type_name_dyn(),
            )
        }
    }

    fn binary_raw(mut self, op: Binary, other: Object) -> Object {
        let (self_, mut other) = match (*self.0).binary(false, op, other) {
            Ok(result) => return result,
            Err(args) => args,
        };

        let (other, self_) = match (*other.0).binary(true, op, self_) {
            Ok(result) => return result,
            Err(args) => args,
        };

        eval_err!(
            "Binary operator {} not implemented between {} and {}",
            op.symbol(),
            self_,
            other,
        )
    }

    pub fn binary(self, op: Binary, other: Object) -> Object {
        match op {
            Binary::Apply => self.call(None, vec![other]),
            Binary::Eq => Bool::create(self == other),
            Binary::Neq => Bool::create(self != other),
            Binary::Leq => self.binary_raw(Binary::Leq, other),
            Binary::Geq => other.binary_raw(Binary::Leq, self),
            Binary::Lt => {
                if self == other {
                    Bool::create(false)
                } else {
                    self.binary_raw(Binary::Leq, other)
                }
            }
            Binary::Gt => {
                if self == other {
                    Bool::create(false)
                } else {
                    other.binary_raw(Binary::Leq, self)
                }
            }
            _ => self.binary_raw(op, other),
        }
    }

    pub fn arity(&self, attr: Option<&str>) -> Option<usize> {
        (*self.0).arity(attr)
    }
    pub fn help(&self, attr: Option<&str>) -> Option<String> {
        (*self.0).help(attr)
    }

    pub fn call(&self, attr: Option<&str>, args: Vec<Object>) -> Object {
        let arity: usize;
        if attr.is_none() && args.is_empty() {
            // Pass through trivial calls
            return self.clone();
        } else if let Some(x) = (*self.0).arity(attr) {
            arity = x;
        } else if let Some(method) = attr {
            return eval_err!(
                "Cannot call method {} on type {}",
                method,
                (*self.0).type_name_dyn()
            );
        } else {
            return eval_err!("Cannot call type {}", (*self.0).type_name_dyn());
        }

        match args.len().cmp(&arity) {
            // Call attribute if arguments match
            Ordering::Equal => return (*self.0).call(attr, args),
            // Defer call if not enough arguments present
            Ordering::Less => {
                let attr = attr.map(|s| s.to_owned());
                return PartialEval::create(self.clone(), attr, args);
            }
            _ => {}
        }

        let mut args = args.into_iter();
        let iter = args.by_ref();

        let mut res = try_ok!((*self.0).call(attr, iter.take(arity).collect()));
        while !iter.as_slice().is_empty() {
            let arity;
            if let Some(x) = (*res.0).arity(None) {
                arity = x;
            } else {
                return eval_err!("Cannot call type {}", (*res.0).type_name_dyn(),);
            }

            res = if iter.as_slice().len() >= arity {
                (*res.0).call(None, iter.take(arity).collect())
            } else {
                PartialEval::create(res.clone(), None, iter.collect())
            };
        }
        res
    }

    fn get_attr(&self, attr: &str) -> Object {
        if let Some(arity) = self.arity(Some(attr)) {
            if arity > 0 {
                eval_err!(
                    "Method {} has arity {}, but wasn't given any arguments",
                    attr,
                    arity,
                )
            } else {
                self.call(Some(attr), Vec::with_capacity(0))
            }
        } else {
            eval_err!(
                "Object of type {} has no method {}",
                (*self.0).type_name_dyn(),
                attr,
            )
        }
    }

    pub fn call_path(&self, mut path: Vec<&str>, args: Vec<Object>) -> Object {
        let last = path.pop();
        if !path.is_empty() {
            let mut obj = try_ok!(self.get_attr(path.remove(0)));
            for key in path.into_iter() {
                obj = try_ok!(obj.get_attr(key));
            }
            obj.call(last, args)
        } else {
            self.call(last, args)
        }
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("Object(")?;
        (*self.0).debug_fmt(f)?;
        f.write_char(')')
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (*self.0).display_fmt(f)
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        (*self.0).eq(other)
    }
}

impl Eq for Object {}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }
        let leq = self.clone().binary(Binary::Leq, other.clone());
        match leq.cast() {
            Ok(true) => Some(Ordering::Less),
            Ok(false) => Some(Ordering::Greater),
            Err(_) => None,
        }
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        (*self.0).clone()
    }
}

impl Neg for Object {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self.unary(Unary::Neg)
    }
}

impl Add for Object {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.binary(Binary::Add, rhs)
    }
}

impl Sub for Object {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.binary(Binary::Sub, rhs)
    }
}

impl Mul for Object {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.binary(Binary::Mul, rhs)
    }
}

impl Div for Object {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self.binary(Binary::Div, rhs)
    }
}

impl Rem for Object {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        self.binary(Binary::Mod, rhs)
    }
}

impl Sum for Object {
    fn sum<I>(mut iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        let mut total = if let Some(x) = iter.next() {
            x
        } else {
            return eval_err!("Can only sum objects for non-empty iterators");
        };

        for x in iter {
            total = try_ok!(total + x);
        }
        total
    }
}

impl Product for Object {
    fn product<I>(mut iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        let mut total = if let Some(x) = iter.next() {
            x
        } else {
            return eval_err!("Can only take product of objects for non-empty iterators");
        };

        for x in iter {
            total = try_ok!(total * x);
        }
        total
    }
}

impl Object {
    pub fn flrdiv(self, rhs: Self) -> Self {
        self.binary(Binary::FlrDiv, rhs)
    }
}

impl AddAssign for Object {
    fn add_assign(&mut self, rhs: Self) {
        self.do_inside(|x| x + rhs)
    }
}

impl SubAssign for Object {
    fn sub_assign(&mut self, rhs: Self) {
        self.do_inside(|x| x - rhs)
    }
}

impl MulAssign for Object {
    fn mul_assign(&mut self, rhs: Self) {
        self.do_inside(|x| x * rhs)
    }
}

impl DivAssign for Object {
    fn div_assign(&mut self, rhs: Self) {
        self.do_inside(|x| x / rhs)
    }
}

impl RemAssign for Object {
    fn rem_assign(&mut self, rhs: Self) {
        self.do_inside(|x| x % rhs)
    }
}
