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
    saved: bool, value: Option<Object>,
}

impl ExprArena {
    fn get_var(&mut self, id: VarId) -> &mut Var {
        if let Inner::Var(var) = &mut self.0[id].inner { var }
        else { panic!("Variable ID doesn't refer a variable node") }
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
    
    fn set_owned(&mut self, child: ExprId) -> Vec<VarId> {
        let node = &mut self.0[child];
        node.owned = true;
        std::mem::replace(&mut node.vars, Vec::with_capacity(0))
    }
    
    
    
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }
    
    fn create_node(&mut self, vars: Vec<VarId>, inner: Inner) -> ExprId {
        self.0.alloc(Node {
            owned: false, vars, inner,
            saved: false, value: None,
        })
    }
    
    pub fn create_array(&mut self, elms: Vec<ExprId>) -> Option<ExprId> {
        if !elms.iter().all(|&id| self.is_ownable(id)) { return None; }
        let vars = elms.iter().map(|&id| self.set_owned(id)).flatten().collect();
        Some(self.create_node(vars, Inner::Array(elms)))
    }
    
    pub fn create_map(&mut self,
        unnamed: Vec<ExprId>, named: HashMap<String, ExprId>
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
        Some(self.create_node(vars, Inner::Map(unnamed, named)))
    }
    
    pub fn from_obj(&mut self, obj: Object) -> ExprId { self.from_obj_raw(obj, false) }
    
    fn from_obj_raw(&mut self, obj: Object, owned: bool) -> ExprId {
        let inner = if obj.is_a::<Map>() {
            let Map {unnamed, named} = obj.cast::<Map>().unwrap();
            Inner::Map(
                unnamed.into_iter().map(|child|
                    self.from_obj_raw(child, true)
                ).collect(),
                named.into_iter().map(|(key, child)|
                    (key, self.from_obj_raw(child, true))
                ).collect(),
            )
        } else if obj.is_a::<Array>() {
            let Array(elems) = obj.cast::<Array>().unwrap();
            Inner::Array(elems.into_iter().map(|child|
                self.from_obj_raw(child, true)
            ).collect())
        } else { Inner::Const(obj) };
        
        self.0.alloc(Node {
            owned, vars: Vec::with_capacity(0), inner,
            saved: false, value: None,
        })
    }
    
    pub fn create_obj<T>(&mut self, obj: T) -> ExprId where T: Objectish {
        self.from_obj(Object::new(obj))
    }
    
    
    
    pub fn create_var(&mut self, name: String) -> VarId {
        self.0.alloc_with_id(|id| Node {
            owned: false, vars: vec![id],
            inner: Inner::Var(Var {
                name, evaling: false, target: None
            }),
            saved: false, value: None,
        })
    }
    
    pub fn create_unary(&mut self, op: opers::Unary, arg: ExprId) -> Option<ExprId> {
        if !self.is_ownable(arg) { return None; }
        let vars = self.set_owned(arg);
        Some(self.create_node(vars, Inner::Unary(op, arg)))
    }
    
    pub fn create_binary(&mut self, op: opers::Binary, arg1: ExprId, arg2: ExprId) -> Option<ExprId> {
        if !self.is_ownable(arg1) || !self.is_ownable(arg2) { return None; }
        let mut vars = self.set_owned(arg1);
        vars.append(&mut self.set_owned(arg2));
        Some(self.create_node(vars, Inner::Binary(op, arg1, arg2)))
    }
    
    pub fn create_call(&mut self, func: ExprId, args: Vec<ExprId>) -> Option<ExprId> {
        if !self.is_ownable(func) { return None; }
        if !args.iter().all(|&id| self.is_ownable(id)) { return None; }
        let mut vars = self.set_owned(func);
        vars.extend(args.iter().map(|&id| self.set_owned(id)).flatten());
        Some(self.create_node(vars, Inner::Call(func, args)))
    }
    
    pub fn set_saved(&mut self, exp: ExprId) { self.0[exp].saved = true; }
    
    
    
    fn is_ownable(&self, target: ExprId) -> bool {
        self.0.get(target).map_or(false, |expr| !expr.owned)
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
    
    
    
    fn take(&mut self, exp: ExprId) -> Object {
        let node = &mut self.0[exp];
        if node.saved { node.value.clone() }
        else { std::mem::take(&mut node.value) }
        .unwrap()
    }
    
    pub fn simplify(&mut self, exp: ExprId) -> bool {
        if self.0[exp].value.is_some() { return true }
        
        macro_rules! simplify { ($child:ident) => {{
            let is_const = self.simplify($child);
            if let Some(obj) = &self.0[$child].value {
                if obj.is_err() {
                    let obj = self.take($child);
                    self.0[exp].value = Some(obj);
                    return true;
                }
            }
            is_const
        }};}
        
        let obj = match &mut self.0[exp].inner {
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
            
            Inner::Var(Var {name, evaling, target}) => if *evaling {
                eval_err!("Circular dependence from variable {}", name)
            } else if let Some(target) = *target {
                *evaling = true;
                let cnst = self.simplify(target);
                self.get_var(exp).evaling = false;
                if !cnst { return false }
                self.take(target)
            } else { eval_err!("Unresolved name \"{}\"", name) },
            
            &mut Inner::Unary(op, arg) => if simplify!(arg) {
                self.take(arg).unary(op)
            } else { return false },
            
            &mut Inner::Binary(op, arg1, arg2) =>
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
                self.0[func].value.as_ref().unwrap().call(args)
            },
        };
        self.0[exp].value = Some(obj);
        true
    }
    
    pub fn eval(&mut self, exp: ExprId) -> Object {
        if self.simplify(exp) { self.take(exp) }
        else { panic!("Cannot evaluate expression; Depends on non-constant values") }
    }
}

