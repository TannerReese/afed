use std::mem::{take, replace};
use std::borrow::Borrow;
use std::hash::Hash;
use std::cmp::Eq;
use std::collections::HashMap;
use id_arena::{Arena, Id};

use super::object::{Object, Objectish, EvalError};
use super::object::opers;
use super::object::array::Array;
use super::object::map::Map;

pub struct ExprArena(Arena<Node>);
pub type ExprId = Id<Node>;
type VarId = ExprId;

#[derive(Debug, Clone)]
enum Inner {
    Const(Object),
    Array(Vec<ExprId>),
    Map(Vec<ExprId>, HashMap<String, ExprId>),
    
    Cache(ExprId, Option<Object>),
    Var(Var),
    Unary(opers::Unary, ExprId),
    Binary(opers::Binary, ExprId, ExprId),
    Call(ExprId, Vec<ExprId>),
}

#[derive(Debug, Clone)]
struct Var {
    name: String,
    evaling: bool,
    target: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct Node {
    owned: bool,
    vars: Vec<VarId>,
    inner: Inner,
}

impl ExprArena {
    fn get_var(&mut self, id: VarId) -> &mut Var {
        if let Inner::Var(var) = &mut self.0[id].inner { var }
        else { panic!("Variable ID doesn't refer a variable node") }
    }
    
    fn set_cache(&mut self, target: ExprId, res: Object) -> &Object {
        if let Inner::Cache(_, result) = &mut self.0[target].inner {
            *result = Some(res);
            if let Some(res) = result { res } else { unreachable!() }
        } else { panic!("Node ID doesn't refer to Cache node") }
    }
    
    pub fn resolve<F>(&mut self, target: ExprId, mut resolver: F)
    where F: FnMut(&mut ExprArena, &str) -> Option<ExprId> {
        let vars = replace(&mut self.0[target].vars, Vec::with_capacity(0));
        
        let unresolved = vars.into_iter().filter(|&id| {
            let name = take(&mut self.get_var(id).name);
            let tgt = resolver(self, &name);
            
            let vr = self.get_var(id);
            vr.name = name;
            vr.target = tgt;
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
    
    fn set_owned(&mut self, child: ExprId) -> Vec<VarId> {
        let node = &mut self.0[child];
        node.owned = true;
        replace(&mut node.vars, Vec::with_capacity(0))
    }
    
    
    
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }
    
    pub fn create_array(&mut self, elms: Vec<ExprId>) -> Option<ExprId> {
        if !elms.iter().all(|&id| self.is_ownable(id)) { return None; }
        let vars = elms.iter().map(|&id| self.set_owned(id)).flatten().collect();
        Some(self.0.alloc(Node {owned: false, vars, inner: Inner::Array(elms)}))
    }
    
    pub fn create_map(&mut self,
        unnamed: Vec<ExprId>, mut named: HashMap<String, ExprId>
    ) -> Option<ExprId> {
        if !unnamed.iter().chain(named.values()).all(|&id| self.is_ownable(id)) { return None; }
        
        let vars = unnamed.iter().chain(named.values()).map(|&id| {
            self.resolve(id, |ar: &mut ExprArena, path: &str| {
                let mut comps = path.split('.');
                comps.next()
                .and_then(|fst| named.get(fst))
                .and_then(|&tgt| ar.find(tgt, comps))
            });
            self.set_owned(id)
        }).flatten().collect();
        
        for id in named.values_mut() {
            let inner = Inner::Cache(*id, None);
            *id = self.0.alloc(Node {
                owned: true, vars: Vec::with_capacity(0), inner
            });
        }
        
        Some(self.0.alloc(Node {
            owned: false, vars,
            inner: Inner::Map(unnamed, named)
        }))
    }
       
    pub fn from_obj(&mut self, obj: Object) -> ExprId { self.from_obj_raw(obj, false) }
    
    fn from_obj_raw(&mut self, obj: Object, owned: bool) -> ExprId {
        let inner = if obj.is_a::<Map>() {
            let Map {unnamed, named} = obj.cast::<Map>().unwrap();
            Inner::Map(
                unnamed.into_iter().map(|child|
                    self.from_obj_raw(child, true)
                ).collect(),
                named.into_iter().map(|(key, child)| {
                    let id = self.from_obj_raw(child, true);
                    (key, self.0.alloc(Node {
                        owned: true, vars: Vec::with_capacity(0),
                        inner: Inner::Cache(id, None)
                    }))
                }).collect(),
            )
        } else if obj.is_a::<Array>() {
            let Array(elems) = obj.cast::<Array>().unwrap();
            Inner::Array(elems.into_iter().map(|child|
                self.from_obj_raw(child, true)
            ).collect())
        } else { Inner::Const(obj) };
        self.0.alloc(Node {owned, vars: Vec::with_capacity(0), inner})
    }
    
    pub fn create_obj<T>(&mut self, obj: T) -> ExprId where T: Objectish {
        self.from_obj(Object::new(obj))
    }
    
    
    
    pub fn create_var(&mut self, name: String) -> VarId {
        self.0.alloc_with_id(|id| Node {
            owned: false, vars: vec![id],
            inner: Inner::Var(Var {
                name, evaling: false, target: None
            })
        })
    }
    
    pub fn create_unary(&mut self, op: opers::Unary, arg: ExprId) -> Option<ExprId> {
        if !self.is_ownable(arg) { return None; }
        let vars = self.set_owned(arg);
        
        Some(self.0.alloc(Node {
            owned: false, vars,
            inner: Inner::Unary(op, arg),
        }))
    }
    
    pub fn create_binary(&mut self, op: opers::Binary, arg1: ExprId, arg2: ExprId) -> Option<ExprId> {
        if !self.is_ownable(arg1) || !self.is_ownable(arg2) { return None; }
        let mut vars = self.set_owned(arg1);
        vars.append(&mut self.set_owned(arg2));
        
        Some(self.0.alloc(Node {
            owned: false, vars,
            inner: Inner::Binary(op, arg1, arg2),
        }))
    }
    
    pub fn create_call(&mut self, func: ExprId, args: Vec<ExprId>) -> Option<ExprId> {
        if !self.is_ownable(func) { return None; }
        if !args.iter().all(|&id| self.is_ownable(id)) { return None; }
        let mut vars = self.set_owned(func);
        vars.extend(args.iter().map(|&id| self.set_owned(id)).flatten());
        
        Some(self.0.alloc(Node {
            owned: false, vars,
            inner: Inner::Call(func, args)
        }))
    }
    
    
    
    pub fn is_ownable(&self, target: ExprId) -> bool {
        self.0.get(target).map_or(false, |expr| !expr.owned)
    }
    
    pub fn get<B>(&self, target: ExprId, key: &B) -> Option<ExprId>
    where
        B: Hash + Eq + ?Sized,
        String: Borrow<B>,
    {
        self.0.get(target)
        .and_then(|node|
            if let Inner::Map(_, named) = &node.inner { Some(named) }
            else { None }
        ).and_then(|named| named.get(key))
        .and_then(|&id| self.0.get(id))
        .and_then(|node|
            if let Inner::Cache(body, _) = node.inner { Some(body) }
            else { None }
        )
    }
    
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
    
    fn eval_ref(&mut self, mut exp: ExprId) -> Option<&Object> {
        match &mut self.0[exp].inner {
            Inner::Cache(body, value) => if value.is_none() {
                let body = *body;
                let res = self.eval(body);
                self.set_cache(exp, res);
            },
            Inner::Var(Var {evaling, target, ..}) => if *evaling {
                return None
            } else if let Some(target) = *target {
                *evaling = true;
                let is_none = self.eval_ref(target).is_none();
                self.get_var(exp).evaling = false;
                if is_none { return None; }
                exp = target;
            } else { return None },
            _ => {},
        }
        
        match &self.0[exp].inner {
            Inner::Const(obj) => Some(obj),
            Inner::Cache(_, value) => value.as_ref(),
            _ => None,
        }
    }
    
    pub fn eval(&mut self, exp: ExprId) -> Object {
        if let Some(Node {inner, ..}) = self.0.get_mut(exp) { match inner {
            Inner::Const(obj) => obj.clone(),
            Inner::Array(elems) => {
                let old_elems = elems.clone();
                let mut elems = Vec::with_capacity(old_elems.len());
                for id in old_elems.into_iter() {
                    elems.push(try_ok!(self.eval(id)))
                }
                Object::new(Array(elems))
            },
            Inner::Map(unnamed, named) => {
                let old_unnamed = unnamed.clone();
                let old_named = named.clone();
                
                let mut unnamed = Vec::with_capacity(old_unnamed.len());
                for id in old_unnamed.into_iter() {
                    unnamed.push(try_ok!(self.eval(id)));
                }
                
                let mut named = HashMap::new();
                for (key, id) in old_named.into_iter() {
                    if let Some(_) = named.insert(key, try_ok!(self.eval(id))) {
                        unreachable!()
                    }
                }
                Object::new(Map { unnamed, named })
            },
            
            Inner::Cache(body, value) => if let Some(res) = value { res } else {
                let body = *body;
                let res = self.eval(body);
                self.set_cache(exp, res)
            }.clone(),
            Inner::Var(Var {name, evaling, target}) => if *evaling {
                eval_err!("Circular dependence from variable {}", name)
            } else if let Some(target) = *target {
                *evaling = true;
                let res = self.eval(target);
                self.get_var(exp).evaling = false;
                res
            } else { eval_err!("Unresolved name \"{}\"", name) },
            
            &mut Inner::Unary(op, arg) => try_ok!(self.eval(arg)).unary(op),
            &mut Inner::Binary(op, arg1, arg2) => {
                try_ok!(self.eval(arg1)).binary(op, try_ok!(self.eval(arg2)))
            },
            
            Inner::Call(func, args) => {
                let (func, args) = (*func, args.clone());
                let mut obj_args = Vec::with_capacity(args.len());
                for id in args.into_iter() {
                    obj_args.push(try_ok!(self.eval(id)));
                }
                
                if let Some(func_ref) = self.eval_ref(func) {
                    func_ref.call(obj_args)
                } else {
                    try_ok!(self.eval(func)).call(obj_args)
                }
            },
        }} else { eval_err!("Unknown Node ID") }
    }
}

