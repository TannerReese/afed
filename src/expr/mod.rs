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
    Map(HashMap<String, ExprId>),

    Var(String, bool, Option<ExprId>),
    Unary(opers::Unary, ExprId),
    Binary(opers::Binary, ExprId, ExprId),
    Access(ExprId, Vec<String>, Vec<ExprId>),
    Arg(String),
    Func(Option<String>, Vec<ArgId>, ExprId),
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

    fn take_vars(&mut self, child: ExprId) -> Vec<VarId> {
        std::mem::replace(&mut self.0[child].vars, Vec::with_capacity(0))
    }

    pub fn resolve<F>(&mut self, target: ExprId, mut resolver: F)
    where F: FnMut(&ExprArena, &str) -> Option<ExprId> {
        let vars = self.take_vars(target);

        let unresolved = vars.into_iter().filter(|&id| {
            let self_imm = &*self;
            let tgt;
            if let Inner::Var(name, is_defn, _) = &self_imm.0[id].inner {
                if *is_defn { true } else {
                    tgt = resolver(self_imm, name);
                    if let Inner::Var(_, _, target) = &mut self.0[id].inner {
                        *target = tgt;
                    } else { unreachable!() }
                    if let Some(tgt) = tgt { self.0[tgt].saved = true; }
                    tgt.is_none()
                }
            } else { panic!("VarId doesn't refer to a variable node") }
        }).collect();
        self.0[target].vars = unresolved;
    }

    pub fn resolve_builtins(&mut self, root: ExprId, bltns: HashMap<String, Object>) {
        let bltns: HashMap<String, ExprId> = bltns.into_iter()
            .map(|(key, obj)| (key, self.from_obj(obj))).collect();

        self.resolve(root, |ar, key| {
            bltns.get(key)
            .or_else(|| bltns.values().filter_map(|&pkg|
                if let Inner::Map(elems) = &ar.0[pkg].inner {
                    elems.get(key)
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

    fn sift_defns(&mut self, exp: ExprId) -> Vec<VarId> {
        let mut defns = Vec::new();
        let vars = self.take_vars(exp)
        .into_iter().filter(|&id| {
            if let Inner::Var(_, is_defn, _) = &self.0[id].inner {
                if *is_defn { defns.push(id); }
                !*is_defn
            } else { panic!("VarId doesn't refer to a variable node") }
        }).collect();
        self.0[exp].vars = vars;
        defns
    }

    pub fn get_defns(&self, exp: ExprId) -> Vec<String> {
        self.0[exp].vars.iter().filter_map(|&id|
            if let Inner::Var(nm, is_defn, _) = &self.0[id].inner {
                if *is_defn { Some(nm.clone()) } else { None }
            } else { panic!("VarId doesn't refer to a variable node") }
        ).collect()
    }

    pub fn create_array(&mut self, elems: Vec<ExprId>) -> ExprId {
        let vars = elems.iter().map(|&id| self.take_vars(id)).flatten().collect();
        self.create_node(vars, Inner::Array(elems))
    }

    pub fn create_map(&mut self, elems: Vec<ExprId>) -> ExprId {
        let defns = elems.iter().map(|&id|
            self.sift_defns(id)
        ).flatten().collect::<Vec<VarId>>();

        let mut named = HashMap::new();
        for id in defns.into_iter() {
            if let Inner::Var(
                nm, is_defn, Some(target)
            ) = &mut self.0[id].inner { if *is_defn {
                if named.insert(nm.clone(), *target).is_some() {
                    panic!("Redefinition of label in map");
                }
                *is_defn = false;
                continue;
            }}
            panic!("VarId doesn't refer to a definition");
        }

        let vars = elems.iter().map(|&id| {
            self.resolve(id, |_, path| named.get(path).cloned());
            self.take_vars(id)
        }).flatten().collect();
        self.create_node(vars, Inner::Map(named))
    }


    pub fn create_var(&mut self, name: String) -> VarId {
        self.0.alloc_with_id(|id| Node {
            evaling: Cell::new(false), vars: vec![id],
            inner: Inner::Var(name, false, None),
            saved: true, value: Cell::new(None),
        })
    }

    pub fn create_defn(&mut self, name: String, body: ExprId) -> VarId {
        let mut vars = self.take_vars(body);
        self.0.alloc_with_id(|id| {
            vars.push(id);
            Node {
                evaling: Cell::new(false), vars: vars,
                inner: Inner::Var(name, true, Some(body)),
                saved: true, value: Cell::new(None),
            }
        })
    }

    pub fn create_unary(&mut self, op: opers::Unary, arg: ExprId) -> ExprId {
        let vars = self.take_vars(arg);
        self.create_node(vars, Inner::Unary(op, arg))
    }

    pub fn create_binary(&mut self,
        op: opers::Binary, arg1: ExprId, arg2: ExprId
    ) -> ExprId {
        let mut vars = self.take_vars(arg1);
        vars.append(&mut self.take_vars(arg2));
        self.create_node(vars, Inner::Binary(op, arg1, arg2))
    }

    pub fn create_access(&mut self,
        exp: ExprId, path: Vec<String>, args: Vec<ExprId>
    ) -> ExprId {
        let mut vars = self.take_vars(exp);
        vars.extend(args.iter().map(|&id| self.take_vars(id)).flatten());
        self.create_node(vars, Inner::Access(exp, path, args))
    }

    pub fn create_func(&mut self,
        name: Option<String>, args: Vec<String>, body: ExprId
    ) -> ExprId {
        let args = args.into_iter().map(|nm|
            self.create_node(Vec::with_capacity(0), Inner::Arg(nm))
        ).collect::<Vec<ArgId>>();

        self.resolve(body, |ar, nm| {
            args.iter().filter_map(|&id|
                if nm == ar.get_argname(id) { Some(id) }
                else { None }
            ).next()
        });
        let vars = self.take_vars(body);
        self.create_node(vars, Inner::Func(name, args, body))
    }


    pub fn from_obj(&mut self, obj: Object) -> ExprId {
        let inner = if obj.is_a::<Map>() {
            let Map(elems) = obj.cast::<Map>().unwrap();
            Inner::Map(elems.into_iter().map(|(key, child)|
                (key, self.from_obj(child))
            ).collect())
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
            } else if let Inner::Var(_, _, Some(id)) = &node.inner {
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
                Inner::Map(named) => if let Some(&id) = named.get(key) {
                    path.next();
                    exp = id;
                } else { break },
                Inner::Var(_, _, Some(target)) => { exp = *target; },
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

            Inner::Map(named) => {
                let named = named.clone();
                let mut cnst = true;
                for &id in named.values() { cnst &= simplify!(id); }

                if !cnst { None } else {
                    let named = named.into_iter()
                        .map(|(key, id)| (key, self.take(id))).collect();
                    Some(Object::new(Map(named)))
                }
            },

            Inner::Var(name, _, target) => if let Some(target) = *target {
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
                arena.create_array(elems)
            },
            Inner::Map(named) => {
                let named = named.iter().map(|(key, id)|
                    (key.clone(), self.clone_into(arena, *id))
                ).collect::<HashMap<String, ExprId>>();
                let vars = named.values().map(|&id| {
                    arena.resolve(id, |_, path| named.get(path).cloned());
                    arena.take_vars(id)
                }).flatten().collect();
                arena.create_node(vars, Inner::Map(named))
            },

            Inner::Var(name, _, _) => arena.create_var(name.clone()),
            &Inner::Unary(op, arg) => {
                let arg = self.clone_into(arena, arg);
                arena.create_unary(op, arg)
            },
            &Inner::Binary(op, arg1, arg2) => {
                let arg1 = self.clone_into(arena, arg1);
                let arg2 = self.clone_into(arena, arg2);
                arena.create_binary(op, arg1, arg2)
            },

            Inner::Access(exp, path, args) => {
                let exp = self.clone_into(arena, *exp);
                let args = args.iter().map(|&id|
                    self.clone_into(arena, id)
                ).collect();
                arena.create_access(exp, path.clone(), args)
            },

            Inner::Arg(name) => arena.create_node(Vec::with_capacity(0),
                Inner::Arg(name.clone())
            ),
            Inner::Func(name, args, body) => {
                let body = self.clone_into(arena, *body);
                let args = args.iter().map(|&id|
                    self.get_argname(id).to_owned()
                ).collect();
                arena.create_func(name.clone(), args, body)
            },
        }
    }
}

