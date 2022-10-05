use std::mem;
use std::borrow::Borrow;
use std::hash::Hash;
use std::cmp::Eq;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;
use id_arena::{Arena, Id};

use super::object::{Object, Objectish, EvalError, EvalResult};
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
    
    Cache(Expr, Option<EvalResult>),
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
    fn set_cache(&mut self, target: Expr, res: EvalResult) -> &EvalResult {
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
                let name = mem::replace(name, Path(Vec::new()));
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
            if let Some(nms) = mem::take(names) { nms } else { return }
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
            if let Some(mut child_names) = mem::take(names) {
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
            mem::take(names)
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
    
    fn from_obj_raw(&mut self, mut obj: Object, owned: bool) -> Expr {
        let inner = if let Some(Map {unnamed, named}) = obj.downcast_mut::<Map>() {
            let unnamed = mem::take(unnamed);
            let named = mem::take(named);
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
        } else if let Some(Array(elems)) = obj.downcast_mut::<Array>() {
            let elems = mem::take(elems);
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
    
    fn eval_ref(&mut self, mut target: Expr) -> Result<Option<&Object>, EvalError> {
        if let Some(Node {inner, ..}) = self.0.get_mut(target) { match inner {
            Inner::Cache(body, value) => if value.is_none() {
                let body = *body;
                let res = self.eval(body);
                self.set_cache(target, res);
            },
            Inner::Var(name, evaling, body) => if *evaling {
                return Err(eval_err!("Circular dependence from variable {}", name));
            } else {
                *evaling = true;
                let body = *body;
                self.eval_ref(body)?;
                self.set_not_evaling(target);
                target = body;
            },
            _ => {},
        }} else { return Err(eval_err!("Unknown Node ID")); }
        
        if let Some(Node {inner, ..}) = self.0.get(target) { match inner {
            Inner::Const(obj) => Ok(Some(obj)),
            Inner::Cache(_, value) => match value {
                None => unreachable!(),
                Some(Ok(obj)) => Ok(Some(obj)),
                Some(Err(err)) => Err(err.clone()),
            },
            _ => Ok(None),
        }} else { unreachable!() }
    }
    
    pub fn eval(&mut self, target: Expr) -> EvalResult {
        if let Some(Node {inner, ..}) = self.0.get_mut(target) { match inner {
            Inner::Const(obj) => Ok(obj.clone()),
            Inner::Array(elems) => Ok(Object::new(Array({
                let elems = elems.clone();
                elems.into_iter().map(|id| self.eval(id))
                .collect::<Result<Vec<Object>, EvalError>>()?
            }))),
            Inner::Map(unnamed, named) => {
                let unnamed = unnamed.clone();
                let named = named.clone();
                
                Ok(Object::new(Map {
                    unnamed: unnamed.into_iter().map(|val|
                        self.eval(val)
                    ).collect::<Result<Vec<Object>, EvalError>>()?,
                    named: named.into_iter().map(|(key, val)| {
                        self.eval(val).map(|obj| (key, obj))
                    }).collect::<Result<HashMap<String, Object>, EvalError>>()?,
                }))
            },
            
            Inner::Cache(body, value) => if let Some(res) = value { res } else {
                let body = *body;
                let res = self.eval(body);
                self.set_cache(target, res)
            }.clone(),
            Inner::Name(path) => Err(eval_err!("Unresolved name \"{}\"", path)),
            Inner::Var(name, evaling, body) => if *evaling {
                Err(eval_err!("Circular dependence from variable {}", name))
            } else {
                *evaling = true;
                let body = *body;
                let res = self.eval(body);
                self.set_not_evaling(target);
                res
            },
            
            &mut Inner::Unary(op, arg) => self.eval(arg)?.apply_unary(op),
            &mut Inner::Binary(op, arg1, arg2) => {
                self.eval(arg1)?.apply_binary(op, self.eval(arg2)?)
            },
            
            Inner::Call(func, args) => {
                let (func, args) = (*func, args.clone());
                let args = args.into_iter().map(|x| self.eval(x))
                .collect::<Result<Vec<Object>, EvalError>>()?;
                
                if let Some(func_ref) = self.eval_ref(func)? {
                    func_ref.apply_call(args)
                } else {
                    self.eval(func)?.apply_call(args)
                }
            },
        }} else { Err(eval_err!("Unknown Node ID")) }
    }
}

