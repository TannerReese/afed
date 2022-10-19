use std::borrow::Borrow;
use std::cell::Cell;
use std::hash::Hash;
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Error};
use id_arena::{Arena, Id};

use super::object::{Object, Objectish, EvalError};
use super::object::opers;
use super::object::array::Array;
use super::object::map::Map;

use func::Func;

pub mod func;


#[derive(Debug, Clone)]
pub struct ExprArena(Arena<Node>);
pub type ExprId = Id<Node>;
type VarId = ExprId;
pub type ArgId = ExprId;

#[derive(Debug, Clone)]
enum Inner {
    Const(Object),
    Array(Vec<ExprId>),
    Map(Vec<ExprId>, HashMap<String, ExprId>),

    Var(Var),
    Unary(opers::Unary, ExprId),
    Binary(opers::Binary, ExprId, ExprId),
    Call(ExprId, Vec<ExprId>),
    Arg(String),
    Func(String, Vec<ArgId>, ExprId),
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    evaling: Cell<bool>,
    target: Option<ExprId>,
}

pub struct Node {
    vars: Vec<VarId>,
    inner: Inner,
    saved: bool, value: Cell<Option<Object>>,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        let val = self.value.take();
        let cloned = val.clone();
        self.value.set(val);
        Node {
            vars: self.vars.clone(), inner: self.inner.clone(),
            saved: self.saved, value: Cell::new(cloned),
        }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let val = self.value.take();
        let cloned = val.clone();
        self.value.set(val);
        write!(f, concat!("Node {{ ",
                "vars: {:?}, inner: {:?}, ",
                "saved: {}, value: Cell({:?}) }}",
            ),
            self.vars, self.inner,
            self.saved, cloned,
        )
    }
}


impl ExprArena {
    fn get_var(&mut self, id: VarId) -> &mut Var {
        if let Inner::Var(var) = &mut self.0[id].inner { var }
        else { panic!("VarId doesn't refer to a variable node") }
    }

    fn get_argname(&self, id: ArgId) -> &str {
        if let Inner::Arg(argname) = &self.0[id].inner { argname }
        else { panic!("ArgId doesn't refer to an argument node") }
    }

    pub fn get<B>(&self, exp: ExprId, key: &B) -> Option<ExprId>
    where
        B: Hash + Eq + ?Sized,
        String: Borrow<B>,
    { if let Some(Node {inner: Inner::Map(_, named), ..}) = self.0.get(exp) {
        named.get(key).cloned()
    } else { None }}

    pub fn find<'a, I, B>(&self, mut target: ExprId, path: I) -> Option<ExprId>
    where
        I: Iterator<Item = &'a B>,
        B: Hash + Eq + 'a + ?Sized,
        String: Borrow<B>,
    {
        for nm in path {
            if let Some(new_target) = self.get(target, nm) {
                target = new_target;
            } else { return None; }
        }
        return Some(target);
    }

    pub fn resolve<F>(&mut self, target: ExprId, mut resolver: F)
    where F: FnMut(&mut ExprArena, &str) -> Option<ExprId> {
        let vars = std::mem::replace(&mut self.0[target].vars, Vec::with_capacity(0));

        let unresolved = vars.into_iter().filter(|&id| {
            let name = std::mem::take(&mut self.get_var(id).name);
            let tgt = resolver(self, &name);

            let vr = self.get_var(id);
            vr.name = name;
            vr.target = tgt;

            if let Some(tgt) = tgt { self.0[tgt].saved = true; }
            tgt.is_none()
        }).collect();
        self.0[target].vars = unresolved;
    }

    pub fn resolve_builtins(&mut self, root: ExprId, bltns: HashMap<String, Object>) {
        let bltns: HashMap<String, ExprId> = bltns.into_iter()
            .map(|(key, obj)| (key, self.from_obj(obj))).collect();

        self.resolve(root, |ar, name| {
            name.split_once('.').and_then(|(pkg, rest)|
                bltns.get(pkg).and_then(|&id| ar.find(id, rest.split('.')))
            ).or_else(|| bltns.values().filter_map(|&pkg_id|
                ar.find(pkg_id, name.split('.'))
            ).next())
        });
    }
}


impl ExprArena {
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }

    fn create_node(&mut self, vars: Vec<VarId>, inner: Inner) -> ExprId {
        self.0.alloc(Node {
            vars, inner, saved: false,
            value: Cell::new(None),
        })
    }

    fn take_vars(&mut self, child: ExprId) -> Vec<VarId> {
        std::mem::replace(&mut self.0[child].vars, Vec::with_capacity(0))
    }

    pub fn create_array(&mut self, elms: Vec<ExprId>) -> Option<ExprId> {
        let vars = elms.iter().map(|&id| self.take_vars(id)).flatten().collect();
        Some(self.create_node(vars, Inner::Array(elms)))
    }

    pub fn create_map(&mut self,
        unnamed: Vec<ExprId>, named: HashMap<String, ExprId>
    ) -> Option<ExprId> {
        let vars = unnamed.iter().chain(named.values()).map(|&id| {
            self.resolve(id, |ar, path| {
                let mut comps = path.split('.');
                comps.next()
                .and_then(|fst| named.get(fst))
                .and_then(|&tgt| ar.find(tgt, comps))
            });
            self.take_vars(id)
        }).flatten().collect();
        Some(self.create_node(vars, Inner::Map(unnamed, named)))
    }


    pub fn create_var(&mut self, name: String) -> VarId {
        self.0.alloc_with_id(|id| Node {
            vars: vec![id], inner: Inner::Var(Var {
                name, evaling: Cell::new(false), target: None,
            }),
            saved: true, value: Cell::new(None),
        })
    }

    pub fn create_unary(&mut self, op: opers::Unary, arg: ExprId) -> Option<ExprId> {
        let vars = self.take_vars(arg);
        Some(self.create_node(vars, Inner::Unary(op, arg)))
    }

    pub fn create_binary(&mut self, op: opers::Binary, arg1: ExprId, arg2: ExprId) -> Option<ExprId> {
        let mut vars = self.take_vars(arg1);
        vars.append(&mut self.take_vars(arg2));
        Some(self.create_node(vars, Inner::Binary(op, arg1, arg2)))
    }

    pub fn create_call(&mut self, func: ExprId, args: Vec<ExprId>) -> Option<ExprId> {
        let mut vars = self.take_vars(func);
        vars.extend(args.iter().map(|&id| self.take_vars(id)).flatten());
        Some(self.create_node(vars, Inner::Call(func, args)))
    }

    pub fn create_func(&mut self, name: String, args: Vec<String>, body: ExprId) -> Option<ExprId> {
        let args = args.into_iter().map(|name|
            self.create_node(Vec::with_capacity(0), Inner::Arg(name))
        ).collect::<Vec<ArgId>>();

        self.resolve(body, |ar, name| {
            args.iter().filter_map(|&id|
                if name == ar.get_argname(id) { Some(id) }
                else { None }
            ).next()
        });
        let vars = self.take_vars(body);
        Some(self.create_node(vars, Inner::Func(name, args, body)))
    }


    pub fn from_obj(&mut self, obj: Object) -> ExprId {
        let inner = if obj.is_a::<Map>() {
            let Map {unnamed, named} = obj.cast::<Map>().unwrap();
            Inner::Map(
                unnamed.into_iter().map(|child|
                    self.from_obj(child)
                ).collect(),
                named.into_iter().map(|(key, child)|
                    (key, self.from_obj(child))
                ).collect(),
            )
        } else if obj.is_a::<Array>() {
            let Array(elems) = obj.cast::<Array>().unwrap();
            Inner::Array(elems.into_iter().map(|child|
                self.from_obj(child)
            ).collect())
        } else { Inner::Const(obj) };

        self.0.alloc(Node {
            vars: Vec::with_capacity(0), inner,
            saved: false, value: Cell::new(None),
        })
    }

    pub fn create_obj<T>(&mut self, obj: T) -> ExprId where T: Objectish {
        self.from_obj(Object::new(obj))
    }

    pub fn set_saved(&mut self, exp: ExprId) { self.0[exp].saved = true; }
}


impl ExprArena {
    fn get_node(&self, mut exp: ExprId) -> &Node {
        loop {
            let node = &self.0[exp];
            if let Some(obj) = node.value.take() {
                node.value.set(Some(obj));
                return node;
            } else if let Inner::Var(
                Var {target: Some(id), ..}
            ) = &node.inner { exp = *id; }
            else { return node; }
        }
    }

    fn take(&self, exp: ExprId) -> Object {
        let node = self.get_node(exp);
        let obj = node.value.take();
        if !node.saved { obj } else {
            let cloned = obj.clone();
            node.value.set(obj);
            cloned
        }.unwrap()
    }

    fn has_value(&self, exp: ExprId) -> bool {
        let node = self.get_node(exp);
        if let Some(obj) = node.value.take() {
            node.value.set(Some(obj));  true
        } else { false }
    }

    fn simplify(&self, exp: ExprId) -> bool {
        if self.has_value(exp) { return true }

        macro_rules! simplify { ($child:ident) => {{
            let is_const = self.simplify($child);
            let node = self.get_node($child);
            if let Some(obj) = node.value.take() {
                if obj.is_err() {
                    self.0[exp].value.set(Some(obj));
                    return true;
                }
                node.value.set(Some(obj));
            }
            is_const
        }};}

        let obj = match &self.0[exp].inner {
            Inner::Const(obj) => obj.clone(),
            Inner::Array(elems) => {
                let elems = elems.clone();
                let mut cnst = true;
                for &id in elems.iter() { cnst &= simplify!(id); }
                if !cnst { return false }

                let elems = elems.iter().map(|&id| self.take(id)).collect();
                Object::new(Array(elems))
            },

            Inner::Map(unnamed, named) => {
                let (unnamed, named) = (unnamed.clone(), named.clone());
                let mut cnst = true;
                for &id in unnamed.iter().chain(named.values()) {
                    cnst &= simplify!(id);
                }
                if !cnst { return false }

                let unnamed = unnamed.into_iter()
                    .map(|id| self.take(id)).collect();
                let named = named.into_iter()
                    .map(|(key, id)| (key, self.take(id))).collect();
                Object::new(Map {unnamed, named})
            },

            Inner::Var(Var {name, evaling, target}) => if evaling.get() {
                eval_err!("Circular dependence from variable {}", name)
            } else if let Some(target) = *target {
                evaling.set(true);
                let cnst = self.simplify(target);
                evaling.set(false);
                return cnst;
            } else { eval_err!("Unresolved name \"{}\"", name) },

            &Inner::Unary(op, arg) => if simplify!(arg) {
                self.take(arg).unary(op)
            } else { return false },

            &Inner::Binary(op, arg1, arg2) =>
            if simplify!(arg1) & simplify!(arg2) {
                self.take(arg1).binary(op, self.take(arg2))
            } else { return false },

            Inner::Call(func, args) => {
                let (func, args) = (*func, args.clone());
                let mut cnst = simplify!(func);
                for &id in args.iter() { cnst &= simplify!(id); }
                if !cnst { return false }

                let args = args.into_iter()
                    .map(|id| self.take(id)).collect();
                let node = self.get_node(func);

                if let Some(obj) = node.value.take() {
                    let res = obj.call(args);
                    node.value.set(Some(obj));
                    res
                } else { panic!("No value found for function") }
            },

            Inner::Arg(_) => return false,
            &Inner::Func(_, _, body) => {
                self.simplify(body);
                let mut arena = ExprArena::new();
                let func = self.clone_into(&mut arena, exp);
                if let Inner::Func(name, args, body) = &mut arena.0[func].inner {
                    let name = std::mem::take(name);
                    let args = std::mem::replace(args, Vec::with_capacity(0));
                    Func::new(name, args, *body, arena)
                } else { unreachable!() }
            },
        };

        self.0[exp].value.set(Some(obj));
        true
    }

    pub fn eval(&self, exp: ExprId) -> Object {
        if self.simplify(exp) { self.take(exp) }
        else { panic!("Cannot evaluate expression; Depends on non-constant values") }
    }
}


impl ExprArena {
    pub fn clear_cache(&self) {
        for (_, node) in self.0.iter() { node.value.set(None); }
    }

    pub fn set_arg(&self, arg: ArgId, value: Object) {
        let node = &self.0[arg];
        if let Inner::Arg(_) = node.inner {
            node.value.set(Some(value));
        } else { panic!("ArgId doesn't refer to an argument node") }
    }

    fn clone_into(&self, arena: &mut ExprArena, exp: ExprId) -> ExprId {
        if self.has_value(exp) {
            return arena.from_obj(self.take(exp));
        }

        match &self.0[exp].inner {
            Inner::Const(obj) => arena.from_obj(obj.clone()),
            Inner::Array(elems) => {
                let elems = elems.iter().map(|id|
                    self.clone_into(arena, *id)
                ).collect();
                arena.create_array(elems).unwrap()
            },
            Inner::Map(unnamed, named) => {
                let unnamed = unnamed.iter().map(|id|
                    self.clone_into(arena, *id)
                ).collect();
                let named = named.iter().map(|(key, id)|
                    (key.clone(), self.clone_into(arena, *id))
                ).collect();
                arena.create_map(unnamed, named).unwrap()
            },

            Inner::Var(Var {name, ..}) => arena.create_var(name.clone()),
            &Inner::Unary(op, arg) => {
                let arg = self.clone_into(arena, arg);
                arena.create_unary(op, arg).unwrap()
            },
            &Inner::Binary(op, arg1, arg2) => {
                let arg1 = self.clone_into(arena, arg1);
                let arg2 = self.clone_into(arena, arg2);
                arena.create_binary(op, arg1, arg2).unwrap()
            },

            Inner::Call(func, args) => {
                let func = self.clone_into(arena, *func);
                let args = args.iter().map(|&id|
                    self.clone_into(arena, id)
                ).collect();
                arena.create_call(func, args).unwrap()
            },

            Inner::Arg(name) => arena.create_node(Vec::with_capacity(0),
                Inner::Arg(name.clone())
            ),
            Inner::Func(name, args, body) => {
                let body = self.clone_into(arena, *body);
                let args = args.iter().map(|&id|
                    self.get_argname(id).to_owned()
                ).collect();
                arena.create_func(name.clone(), args, body).unwrap()
            },
        }
    }
}

