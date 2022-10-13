use std::mem::{take, replace};
use std::borrow::Borrow;
use std::hash::Hash;
use std::cmp::Eq;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;
use id_arena::{Arena, Id};

use super::object::{Object, Objectish, EvalError};
use super::object::opers;
use super::object::array::Array;
use super::object::map::Map;

pub struct ExprArena(Arena<Node>);
pub type Expr = Id<Node>;
type Name = Expr;

#[derive(Debug, Clone)]
struct Path(Vec<String>);

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0[0])?;
        for part in self.0[1..].iter() { write!(f, ".{}", part)?; }
        Ok(())
    }
}

fn bltn_find<'a, 'b>(bltns: &'a HashMap<String, Object>, path: &'b Path) -> Option<&'a Object> {
    if let Some(obj) = bltns.get(&path.0[0]) {
        if let Some(res) = obj.find(path.0[1..].iter()) {
            return Some(res);
        }
    }
    
    for pkg in bltns.values() {
        if let Some(obj) = pkg.find(path.0.iter()) {
            return Some(obj);
        }
    }
    None
}

#[derive(Debug, Clone)]
enum Inner {
    Const(Object),
    Array(Vec<Expr>),
    Map(Vec<Expr>, HashMap<String, Expr>),
    
    Cache(Expr, Option<Object>),
    Name(Path),
    Var(Path, bool, Expr),
    Unary(opers::Unary, Expr),
    Binary(opers::Binary, Expr, Expr),
    Call(Expr, Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct Node {
    owned: bool,
    names: Option<Vec<Name>>,
    inner: Inner,
}

impl ExprArena {
    fn set_cache(&mut self, target: Expr, res: Object) -> &Object {
        if let Some(Node {inner: Inner::Cache(_, value), ..}) = self.0.get_mut(target) {
            *value = Some(res);
            if let Some(res) = value { res } else { unreachable!() }
        } else { panic!("Node ID doesn't refer to Cache node") }
    }
    
    fn set_not_evaling(&mut self, target: Expr) {
        if let Some(Node {inner: Inner::Var(_, evaling, _), ..}) = self.0.get_mut(target) {
            *evaling = false;
        } else { panic!("Node ID doesn't refer to Var node") }
    }
    
    fn make_const(&mut self, name_id: Name, val: &Object) {
        if let Some(Node {inner, ..}) = self.0.get_mut(name_id) {
            if let Inner::Name(_) = inner {
                *inner = Inner::Const(val.clone());
                return;
            }
        }
        panic!("Name ID doesn't refer to Name node")
    }
    
    // Convert Name-type Node into Var-type Node by resolving the name
    fn make_var(&mut self, name_id: Name, tgt_id: Expr) {
        if let Some(Node {inner, ..}) = self.0.get_mut(name_id) {
            if let Inner::Name(name) = inner {
                let name = replace(name, Path(Vec::new()));
                *inner = Inner::Var(name, false, tgt_id);
                return;
            }
        }
        panic!("Name ID doesn't refer to name")
    }
    
    // Resolve names, merge remaining names, and set ownership
    // Returns unresolved names
    fn resolve_names(&mut self, target: Expr, map: &HashMap<String, Expr>) {
        let names = if let Some(Node {names, ..}) = self.0.get_mut(target) {
            if let Some(nms) = take(names) { nms } else { return }
        } else { return };
        
        let unresolved = names.into_iter().filter(|&name_id| {
            if let Some(Node {inner: Inner::Name(name), ..}) = self.0.get(name_id) {
                if let Some(tgt_id) = map.get(&name.0[0])
                .and_then(|&id| self.find(id, name.0[1..].iter())) {
                    self.make_var(name_id, tgt_id);
                    false
                } else { true }
            } else { panic!("Name ID doesn't refer to name") }
        }).collect::<Vec<Name>>();
        
        if unresolved.len() > 0 {
            if let Some(Node {names, ..}) = self.0.get_mut(target) {
                *names = Some(unresolved);
            } else { unreachable!() }
        }
    }
    
    fn set_owned(&mut self, child: Expr, parent_names: &mut Option<Vec<Name>>) {
        if let Some(Node {owned, names, ..}) = self.0.get_mut(child) {
            *owned = true;
            if let Some(mut child_names) = take(names) {
                if let Some(parent_names) = parent_names {
                    parent_names.append(&mut child_names);
                } else {
                    *parent_names = Some(child_names);
                }
            }
        } else { panic!("Unknown Node ID") }
    }
    
    pub fn resolve_builtins(&mut self, root: Expr, bltns: &HashMap<String, Object>) -> bool {
        let names = if let Some(Node {names: Some(names), ..}) = self.0.get_mut(root) {
            take(names)
        } else { return false; };
        
        let unresolved = names.into_iter().filter(|&name_id|
            if let Some(Node {inner: Inner::Name(name), ..}) = self.0.get(name_id) {
                if let Some(obj) = bltn_find(bltns, name) {
                    self.make_const(name_id, obj);
                    false
                } else { true }
            } else { panic!("Name ID doesn't refer to name") }
        ).collect::<Vec<Name>>();
        
        if unresolved.len() > 0 {
            if let Some(Node {names, ..}) = self.0.get_mut(root) {
                *names = Some(unresolved);
            } else { unreachable!() }
        }
        return true;
    }
    
    
    
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }
    
    pub fn create_array(&mut self, elems: Vec<Expr>) -> Option<Expr> {
        if !elems.iter().all(|&id| self.is_ownable(id)) { return None; }
        
        let mut names: Option<Vec<Name>> = None;
        for &id in elems.iter() { self.set_owned(id, &mut names); }
        
        Some(self.0.alloc(Node {owned: false, names, inner: Inner::Array(elems)}))
    }
    
    pub fn create_map(&mut self, unnamed: Vec<Expr>, mut named: HashMap<String, Expr>) -> Option<Expr> {
        if !unnamed.iter().all(|&id| self.is_ownable(id)) { return None; }
        if !named.iter().all(|(_, &id)| self.is_ownable(id)) { return None; }
        
        let mut map_nms: Option<Vec<Name>> = None;
        
        // Merge name list and resolve names for unnamed members
        for &id in unnamed.iter() {
            self.resolve_names(id, &named);
            self.set_owned(id, &mut map_nms);
        }
        
        // Merge name list and resolve names for named members
        for (_, &id) in named.iter() {
            self.resolve_names(id, &named);
            self.set_owned(id, &mut map_nms);
        }
        
        // Wrap the named in Cache nodes
        // so their results can be reused
        for (_, id) in named.iter_mut() {
            *id = self.0.alloc(Node {
                owned: true, names: None, inner: Inner::Cache(*id, None)
            });
        }
        
        Some(self.0.alloc(Node {
            owned: false, names: map_nms,
            inner: Inner::Map(unnamed, named)
        }))
    }
       
    pub fn from_obj(&mut self, obj: Object) -> Expr { self.from_obj_raw(obj, false) }
    
    fn from_obj_raw(&mut self, obj: Object, owned: bool) -> Expr {
        let inner = if obj.is_a::<Map>() {
            let Map {unnamed, named} = obj.cast::<Map>().unwrap();
            Inner::Map(
                unnamed.into_iter().map(|child|
                    self.from_obj_raw(child, true)
                ).collect(),
                named.into_iter().map(|(key, child)| {
                    let id = self.from_obj_raw(child, true);
                    (key, self.0.alloc(Node {
                        owned: true, names: None,
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
        self.0.alloc(Node {owned, names: None, inner})
    }
    
    pub fn create_obj<T>(&mut self, obj: T) -> Expr where T: Objectish {
        self.from_obj(Object::new(obj))
    }
    
    
    
    pub fn create_name(&mut self, name: String) -> Expr {
        let name = Path(name.split('.').map(|s| s.to_owned()).collect());
        self.0.alloc_with_id(|id| {
            Node {owned: false, names: Some(vec![id]), inner: Inner::Name(name)}
        })
    }
    
    pub fn create_unary(&mut self, op: opers::Unary, arg: Expr) -> Option<Expr> {
        if !self.is_ownable(arg) { return None; }
        
        let mut names: Option<Vec<Name>> = None;
        self.set_owned(arg, &mut names);
        
        Some(self.0.alloc(Node {
            owned: false, names,
            inner: Inner::Unary(op, arg),
        }))
    }
    
    pub fn create_binary(&mut self, op: opers::Binary, arg1: Expr, arg2: Expr) -> Option<Expr> {
        if !self.is_ownable(arg1) || !self.is_ownable(arg2) { return None; }
        
        let mut names: Option<Vec<Name>> = None;
        self.set_owned(arg1, &mut names);
        self.set_owned(arg2, &mut names);
        
        Some(self.0.alloc(Node {
            owned: false, names,
            inner: Inner::Binary(op, arg1, arg2),
        }))
    }
    
    pub fn create_call(&mut self, func: Expr, args: Vec<Expr>) -> Option<Expr> {
        if !self.is_ownable(func) { return None; }
        if !args.iter().all(|&id| self.is_ownable(id)) { return None; }
        
        let mut names: Option<Vec<Name>> = None;
        self.set_owned(func, &mut names);
        for &id in args.iter() { self.set_owned(id, &mut names); }
        
        Some(self.0.alloc(Node {
            owned: false, names,
            inner: Inner::Call(func, args)
        }))
    }
    
    
    
    pub fn is_ownable(&self, target: Expr) -> bool {
        self.0.get(target).map_or(false, |expr| !expr.owned)
    }
    
    pub fn get<B>(&self, target: Expr, key: &B) -> Option<Expr>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    {
        if let Some(Node {
            inner: Inner::Map(_, named), ..
        }) = self.0.get(target) {
            if let Some(&elem_id) = named.get(key) {
                if let Some(&Node {
                    inner: Inner::Cache(body, _), ..
                }) = self.0.get(elem_id) {
                    Some(body)
                } else { None }
            } else { None }
        } else { None }
    }
    
    pub fn find<'a, I, B>(&self, mut target: Expr, path: I) -> Option<Expr>
    where
        I: Iterator<Item = &'a B>,
        B: Hash + Eq + 'a,
        String: Borrow<B>,
    {
        for nm in path {
            if let Some(new_target) = self.get(target, nm) {
                target = new_target;
            } else { return None; }
        }
        return Some(target);
    }
    
    fn eval_ref(&mut self, mut target: Expr) -> Option<&Object> {
        if let Some(Node {inner, ..}) = self.0.get_mut(target) { match inner {
            Inner::Cache(body, value) => if value.is_none() {
                let body = *body;
                let res = self.eval(body);
                self.set_cache(target, res);
            },
            Inner::Var(_, evaling, body) => if *evaling {
                return None
            } else {
                *evaling = true;
                let body = *body;
                let is_none = self.eval_ref(body).is_none();
                self.set_not_evaling(target);
                if is_none { return None; }
                target = body;
            },
            _ => {},
        }} else { return None; }
        
        if let Some(Node {inner, ..}) = self.0.get(target) { match inner {
            Inner::Const(obj) => Some(obj),
            Inner::Cache(_, value) => value.as_ref(),
            _ => None,
        }} else { unreachable!() }
    }
    
    pub fn eval(&mut self, target: Expr) -> Object {
        if let Some(Node {inner, ..}) = self.0.get_mut(target) { match inner {
            Inner::Const(obj) => obj.clone(),
            Inner::Array(elems) => {
                let elems = elems.clone();
                let mut new_elems = Vec::new();
                for id in elems.into_iter() {
                    new_elems.push(try_ok!(self.eval(id)))
                }
                Object::new(Array(new_elems))
            },
            Inner::Map(unnamed, named) => {
                let old_unnamed = unnamed.clone();
                let old_named = named.clone();
                
                let mut unnamed = Vec::new();
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
                self.set_cache(target, res)
            }.clone(),
            Inner::Name(path) => eval_err!("Unresolved name \"{}\"", path),
            Inner::Var(name, evaling, body) => if *evaling {
                eval_err!("Circular dependence from variable {}", name)
            } else {
                *evaling = true;
                let body = *body;
                let res = self.eval(body);
                self.set_not_evaling(target);
                res
            },
            
            &mut Inner::Unary(op, arg) => try_ok!(self.eval(arg)).unary(op),
            &mut Inner::Binary(op, arg1, arg2) => {
                try_ok!(self.eval(arg1)).binary(op, try_ok!(self.eval(arg2)))
            },
            
            Inner::Call(func, args) => {
                let (func, args) = (*func, args.clone());
                let mut obj_args = Vec::new();
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

