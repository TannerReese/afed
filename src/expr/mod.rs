use std::cell::Cell;
use std::slice::Iter;
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

    Var(String, Option<ExprId>),
    Unary(opers::Unary, ExprId),
    Binary(opers::Binary, ExprId, ExprId),
    Access(ExprId, Vec<String>, Vec<ExprId>),
    Arg(String),
    Func(String, Vec<ArgId>, ExprId),
}

pub struct Node {
    evaling: Cell<bool>,
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
            evaling: Cell::new(false),
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
    fn get_argname(&self, id: ArgId) -> &str {
        if let Inner::Arg(argname) = &self.0[id].inner { argname }
        else { panic!("ArgId doesn't refer to an argument node") }
    }

    pub fn resolve<F>(&mut self, target: ExprId, mut resolver: F)
    where F: FnMut(&ExprArena, &str) -> Option<ExprId> {
        let vars = std::mem::replace(&mut self.0[target].vars, Vec::with_capacity(0));

        let unresolved = vars.into_iter().filter(|&id| {
            let self_imm = &*self;
            let tgt;
            if let Inner::Var(name, _) = &self_imm.0[id].inner {
                tgt = resolver(self_imm, name);
            } else { panic!("VarId doesn't refer to a variable node") }

            if let Inner::Var(_, target) = &mut self.0[id].inner {
                *target = tgt;
            } else { unreachable!() }

            if let Some(tgt) = tgt { self.0[tgt].saved = true; }
            tgt.is_none()
        }).collect();
        self.0[target].vars = unresolved;
    }

    pub fn resolve_builtins(&mut self, root: ExprId, bltns: HashMap<String, Object>) {
        let bltns: HashMap<String, ExprId> = bltns.into_iter()
            .map(|(key, obj)| (key, self.from_obj(obj))).collect();

        self.resolve(root, |ar, key| {
            bltns.get(key)
            .or_else(|| bltns.values().filter_map(|&pkg|
                if let Inner::Map(_, named) = &ar.0[pkg].inner {
                    named.get(key)
                } else { None }
            ).next()).cloned()
        });
    }
}


impl ExprArena {
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }

    fn create_node(&mut self, vars: Vec<VarId>, inner: Inner) -> ExprId {
        self.0.alloc(Node {
            evaling: Cell::new(false),
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
            self.resolve(id, |_, path| named.get(path).cloned());
            self.take_vars(id)
        }).flatten().collect();
        Some(self.create_node(vars, Inner::Map(unnamed, named)))
    }


    pub fn create_var(&mut self, name: String) -> VarId {
        self.0.alloc_with_id(|id| Node {
            evaling: Cell::new(false),
            vars: vec![id], inner: Inner::Var(name, None),
            saved: true, value: Cell::new(None),
        })
    }

    pub fn create_unary(&mut self, op: opers::Unary, arg: ExprId) -> Option<ExprId> {
        let vars = self.take_vars(arg);
        Some(self.create_node(vars, Inner::Unary(op, arg)))
    }

    pub fn create_binary(&mut self,
        op: opers::Binary, arg1: ExprId, arg2: ExprId
    ) -> Option<ExprId> {
        let mut vars = self.take_vars(arg1);
        vars.append(&mut self.take_vars(arg2));
        Some(self.create_node(vars, Inner::Binary(op, arg1, arg2)))
    }

    pub fn create_access(&mut self,
        exp: ExprId, path: Vec<String>, args: Vec<ExprId>
    ) -> Option<ExprId> {
        let mut vars = self.take_vars(exp);
        vars.extend(args.iter().map(|&id| self.take_vars(id)).flatten());
        Some(self.create_node(vars, Inner::Access(exp, path, args)))
    }

    pub fn create_func(&mut self,
        name: String, args: Vec<String>, body: ExprId
    ) -> Option<ExprId> {
        //let old_args = args.clone();
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
            evaling: Cell::new(false),
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
            } else if let Inner::Var(_, Some(id)) = &node.inner {
                exp = *id;
            } else { return node; }
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

    fn access(&self,
        mut exp: ExprId, path: &mut Iter<'_, String>
    ) -> ExprId {
        while let Some(key) = path.as_slice().get(0) {
            match &self.0[exp].inner {
                Inner::Map(_, named) => if let Some(&id) = named.get(key) {
                    path.next();
                    exp = id;
                } else { break },
                Inner::Var(_, Some(target)) => { exp = *target; },
                _ => break,
            }
        }
        exp
    }

    fn simplify(&self, args_used: &mut Vec<ArgId>, exp: ExprId) -> bool {
        if self.has_value(exp) { return true }
        let node = &self.0[exp];
        if node.evaling.get() {
            node.value.set(Some(eval_err!("Circular dependency")));
            return true;
        } else { node.evaling.set(true) }

        macro_rules! simplify { ($child:ident) => {{
            let is_const = self.simplify(args_used, $child);
            let child_node = self.get_node($child);
            if let Some(obj) = child_node.value.take() {
                if obj.is_err() {
                    node.value.set(Some(obj));
                    node.evaling.set(false);
                    return true;
                }
                child_node.value.set(Some(obj));
            }
            is_const
        }};}

        let obj = match &node.inner {
            Inner::Const(obj) => Some(obj.clone()),
            Inner::Array(elems) => {
                let elems = elems.clone();
                let mut cnst = true;
                for &id in elems.iter() { cnst &= simplify!(id); }

                if !cnst { None } else {
                    let elems = elems.iter().map(|&id| self.take(id)).collect();
                    Some(Object::new(Array(elems)))
                }
            },

            Inner::Map(unnamed, named) => {
                let (unnamed, named) = (unnamed.clone(), named.clone());
                let mut cnst = true;
                for &id in unnamed.iter().chain(named.values()) {
                    cnst &= simplify!(id);
                }

                if !cnst { None } else {
                    let unnamed = unnamed.into_iter()
                        .map(|id| self.take(id)).collect();
                    let named = named.into_iter()
                        .map(|(key, id)| (key, self.take(id))).collect();
                    Some(Object::new(Map {unnamed, named}))
                }
            },

            Inner::Var(name, target) => if let Some(target) = *target {
                let res = self.simplify(args_used, target);
                node.evaling.set(false);
                return res;
            } else { Some(eval_err!("Unresolved name \"{}\"", name)) },

            &Inner::Unary(op, arg) => if simplify!(arg) {
                Some(self.take(arg).unary(op))
            } else { None },

            &Inner::Binary(op, arg1, arg2) =>
            if simplify!(arg1) & simplify!(arg2) {
                Some(self.take(arg1).binary(op, self.take(arg2)))
            } else { None },

            Inner::Access(target, path, args) => {
                let (target, mut path, args) = (*target, path.iter(), args.clone());
                let target = self.access(target, &mut path);

                let mut cnst = simplify!(target);
                for &id in args.iter() { cnst &= simplify!(id); }
                if !cnst { None } else {
                    let args: Vec<Object> = args.into_iter()
                        .map(|id| self.take(id)).collect();
                    let tgt_node = self.get_node(target);

                    if let Some(obj) = tgt_node.value.take() {
                        let path: Vec<&str> = path.map(|s| s.as_str()).collect();
                        let res = if args.len() == 0 && path.len() == 0 {
                            obj.clone()
                        } else { obj.call_path(path, args) };

                        tgt_node.value.set(Some(obj));
                        Some(res)
                    } else { panic!("No value found for function") }
                }
            },

            Inner::Arg(_) => { args_used.push(exp); None },
            Inner::Func(_, args, body) => {
                let mut used = Vec::new();
                self.simplify(&mut used, *body);
                let cnst = used.iter().all(|id| args.contains(id));

                if !cnst { None } else {
                    let mut arena = ExprArena::new();
                    let func = self.clone_into(&mut arena, exp);
                    if let Inner::Func(
                        name, args, body
                    ) = &mut arena.0[func].inner {
                        let name = std::mem::take(name);
                        let args = std::mem::replace(args, Vec::with_capacity(0));
                        Some(Func::new(name, args, *body, arena))
                    } else { unreachable!() }
                }
            },
        };

        node.evaling.set(false);
        let is_cnst = obj.is_some();
        node.value.set(obj);
        is_cnst
    }

    pub fn eval(&self, exp: ExprId) -> Object {
        let mut args_used = Vec::new();
        if self.simplify(&mut args_used, exp) { self.take(exp) }
        else {
            let mut argnames = String::new();
            let mut is_first = true;
            for &id in args_used.iter() {
                if !is_first { argnames += ", "; }
                is_first = false;
                argnames += self.get_argname(id);
            }

            eval_err!(
                "Depends on non-constant argument{} {}",
                if args_used.len() == 1 { "" } else { "s" },
                argnames,
            )
        }
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

            Inner::Var(name, _) => arena.create_var(name.clone()),
            &Inner::Unary(op, arg) => {
                let arg = self.clone_into(arena, arg);
                arena.create_unary(op, arg).unwrap()
            },
            &Inner::Binary(op, arg1, arg2) => {
                let arg1 = self.clone_into(arena, arg1);
                let arg2 = self.clone_into(arena, arg2);
                arena.create_binary(op, arg1, arg2).unwrap()
            },

            Inner::Access(exp, path, args) => {
                let exp = self.clone_into(arena, *exp);
                let args = args.iter().map(|&id|
                    self.clone_into(arena, id)
                ).collect();
                arena.create_access(exp, path.clone(), args).unwrap()
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

