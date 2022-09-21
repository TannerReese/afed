use std::mem;
use std::borrow::Borrow;
use std::hash::Hash;
use std::cmp::Eq;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;
use id_arena::{Arena, Id};

use super::opers;
use super::object::{Object, EvalError, EvalResult};

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

#[derive(Debug, Clone)]
enum Inner {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Expr>),
    Map(Vec<Expr>, HashMap<String, Expr>),
    
    Cache(Expr, Option<EvalResult>),
    Name(Path),
    Var(Path, bool, Expr),
    Unary(opers::Unary, Expr),
    Binary(opers::Binary, Expr, Expr),
}

#[derive(Debug, Clone)]
pub struct Node {
    owned: bool,
    names: Option<Vec<Name>>,
    inner: Inner,
}

impl ExprArena {
    fn set_cache(&mut self, target: Expr, res: EvalResult) -> bool {
        if let Some(Node {inner: Inner::Cache(_, value), ..}) = self.0.get_mut(target) {
            *value = Some(res);
            true
        } else { false }
    }
    
    fn set_not_evaling(&mut self, target: Expr) -> bool {
        if let Some(Node {inner: Inner::Var(_, evaling, _), ..}) = self.0.get_mut(target) {
            *evaling = false;
            true
        } else { false }
    }
    
    // Convert Name-type Node into Var-type Node by resolving the name
    fn make_var(&mut self, name_id: Expr, tgt_id: Expr) -> bool {
        if let Some(Node {inner, ..}) = self.0.get_mut(name_id) {
            if let Inner::Name(name) = inner {
                let name = mem::replace(name, Path(Vec::new()));
                *inner = Inner::Var(name, false, tgt_id);
                true
            } else { false }
        } else { false }
    }
    
    // Resolve names, merge remaining names, and set ownership
    fn resolve(&mut self, map: &HashMap<String, Expr>, names: Option<Vec<Name>>) -> Option<Vec<Name>> {
        let names = if let Some(names) = names { names } else { return None };
        let names: Vec<Name> = names.into_iter().filter(|&name_id| {
            if let Some(Node {inner: Inner::Name(name), ..}) = self.0.get(name_id) {
                if let Some(tgt_id) = map.get(&name.0[0])
                .and_then(|&id| self.find(id, name.0[1..].iter())) {
                    if !self.make_var(name_id, tgt_id) { unreachable!() }
                    false
                } else { true }
            } else { panic!("Name ID doesn't refer to name") }
        }).collect();
        
        if names.len() == 0 { None } else { Some(names) }
    }
    
    fn merge_names(names1: &mut Option<Vec<Name>>, names2: Option<Vec<Name>>) {
        if let Some(mut ns2) = names2 {
            if let Some(ns1) = names1 {
                ns1.append(&mut ns2);
            } else {
                *names1 = Some(ns2);
            }
        }
    }
    
    
    
    pub fn new() -> ExprArena { ExprArena(Arena::new()) }
    
    #[allow(dead_code)]
    pub fn new_null(&mut self) -> Expr {
        self.0.alloc(Node {owned: false, names: None, inner: Inner::Null})
    }
    
    #[allow(dead_code)]
    pub fn new_bool(&mut self, b: bool) -> Expr {
        self.0.alloc(Node {owned: false, names: None, inner: Inner::Bool(b)})
    }
    
    pub fn new_num(&mut self, num: f64) -> Expr {
        self.0.alloc(Node {owned: false, names: None, inner: Inner::Num(num)})
    }
    
    pub fn new_str(&mut self, s: String) -> Expr {
        self.0.alloc(Node {owned: false, names: None, inner: Inner::Str(s)})
    }
    
    pub fn new_arr(&mut self, elems: Vec<Expr>) -> Option<Expr> {
        if elems.iter().any(|&id| self.is_owned(id)) { return None; }
        
        let mut arr_names: Option<Vec<Name>> = None;
        for &id in elems.iter() {
            if let Some(Node {owned, names, ..}) = self.0.get_mut(id) {
                *owned = true;
                ExprArena::merge_names(&mut arr_names, mem::take(names));
            }
        }
        
        Some(self.0.alloc(Node {owned: false, names: arr_names, inner: Inner::Arr(elems)}))
    }
    
    pub fn new_map(&mut self, free_elems: Vec<Expr>, mut elems: HashMap<String, Expr>) -> Option<Expr> {
        if free_elems.iter().any(|&id| self.is_owned(id)) { return None; }
        if elems.iter().any(|(_, &id)| self.is_owned(id)) { return None; }
        
        let mut map_names: Option<Vec<Name>> = None;
        
        // Merge name list and resolve names for unnamed members
        for &id in free_elems.iter() {
            if let Some(Node {owned, names, ..}) = self.0.get_mut(id) {
                *owned = true;
                let names = mem::take(names);
                ExprArena::merge_names(&mut map_names, self.resolve(&elems, names));
            } else { panic!("Unknown Expression ID") }
        }
        
        // Merge name list and resolve names for named members
        for (_, &id) in elems.iter() {
            if let Some(Node {owned, names, ..}) = self.0.get_mut(id) {
                *owned = true;
                let names = mem::take(names);
                ExprArena::merge_names(&mut map_names, self.resolve(&elems, names));
            } else { panic!("Unknown Expression ID") }
        }
        
        // Wrap the elems in Cache nodes
        // so their results can be reused
        for (_, id) in elems.iter_mut() {
            *id = self.0.alloc(Node {
                owned: true, names: None, inner: Inner::Cache(*id, None)
            });
        }
        
        Some(self.0.alloc(Node {
            owned: false, names: map_names,
            inner: Inner::Map(free_elems, elems)
        }))
    }
    
    pub fn from_obj(&mut self, obj: &Object) -> Expr {
        let inner = match obj {
            Object::Null => Inner::Null,
            Object::Bool(b) => Inner::Bool(*b),
            Object::Num(r) => Inner::Num(*r),
            Object::Str(s) => Inner::Str(s.clone()),
            Object::Arr(elems) => Inner::Arr(
                elems.iter().map(|child| self.from_obj(child)).collect()
            ),
            Object::Map(free_elems, elems) => Inner::Map(
                free_elems.iter().map(|child|
                    self.from_obj(child)
                ).collect(),
                elems.iter().map(|(key, child)|
                    (key.clone(), self.from_obj(child))
                ).collect(),
            ),
        };
        self.0.alloc(Node {owned: false, names: None, inner})
    }
    
    
    
    pub fn new_name(&mut self, name: String) -> Expr {
        let name = Path(name.split('.').map(|s| s.to_owned()).collect());
        let mut name_ids = Vec::new();
        self.0.alloc_with_id(|id| {
            name_ids.push(id);
            Node {owned: false, names: Some(name_ids), inner: Inner::Name(name)}
        })
    }
    
    pub fn new_unary(&mut self, op: opers::Unary, arg: Expr) -> Option<Expr> {
        if self.is_owned(arg) { return None; }
        
        let names = if let Some(Node {owned, names, ..}) = self.0.get_mut(arg) {
            *owned = true;
            mem::take(names)
        } else { unreachable!() };
        
        Some(self.0.alloc(Node {owned: false, names, inner: Inner::Unary(op, arg)}))
    }
    
    pub fn new_binary(&mut self, op: opers::Binary, arg1: Expr, arg2: Expr) -> Option<Expr> {
        if self.is_owned(arg1) || self.is_owned(arg2) { return None; }
        
        let mut names = if let Some(Node {owned, names: ns, ..}) = self.0.get_mut(arg1) {
            *owned = true;
            mem::take(ns)
        } else { unreachable!() };
        
        if let Some(Node {owned, names: ns, ..}) = self.0.get_mut(arg2) {
            *owned = true;
            ExprArena::merge_names(&mut names, mem::take(ns))
        } else { unreachable!() };
        
        Some(self.0.alloc(Node {owned: false, names, inner: Inner::Binary(op, arg1, arg2)}))
    }
    
    
    
    pub fn is_owned(&self, target: Expr) -> bool {
        self.0.get(target).map_or(true, |expr| expr.owned)
    }
    
    pub fn get<B>(&self, target: Expr, key: &B) -> Option<Expr>
    where
        B: Hash + Eq,
        String: Borrow<B>,
    {
        if let Some(Node {
            inner: Inner::Map(_, elems), ..
        }) = self.0.get(target) {
            if let Some(&elem_id) = elems.get(key) {
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
        I:Iterator<Item = &'a B>,
        B: Hash + Eq + std::fmt::Debug + 'a,
        String: Borrow<B>,
    {
        for nm in path {
            if let Some(new_target) = self.get(target, nm) {
                target = new_target;
            } else { return None; }
        }
        return Some(target);
    }
    
    pub fn eval(&mut self, target: Expr) -> EvalResult {
        if let Some(Node {inner, ..}) = self.0.get_mut(target) { match inner {
            Inner::Null => Ok(Object::Null),
            Inner::Bool(b) => Ok(Object::Bool(*b)),
            Inner::Num(r) => Ok(Object::Num(*r)),
            Inner::Str(s) => Ok(Object::Str(s.clone())),
            Inner::Arr(elems) => Ok(Object::Arr({
                let elems = elems.clone();
                elems.iter().map(|id| self.eval(*id))
                .collect::<Result<Vec<Object>, EvalError>>()?
            })),
            Inner::Map(free_elems, elems) => {
                let free_elems = free_elems.clone();
                let elems = elems.clone();
                
                Ok(Object::Map(
                    free_elems.into_iter().map(|val|
                        self.eval(val)
                    ).collect::<Result<Vec<Object>, EvalError>>()?,
                    elems.into_iter().map(|(key, val)| {
                        self.eval(val).map(|obj| (key, obj))
                    }).collect::<Result<HashMap<String, Object>, EvalError>>()?,
                ))
            },
            
            Inner::Cache(body, value) => if let Some(res) = value {
                res.clone()
            } else {
                let body = *body;
                let res = self.eval(body);
                if !self.set_cache(target, res.clone()) { unreachable!() }
                res
            },
            Inner::Name(_) => Err(eval_err!("Unresolved name")),
            Inner::Var(name, evaling, body) => if *evaling {
                Err(eval_err!("Circular dependence from variable {}", name))
            } else {
                *evaling = true;
                let body = *body;
                let res = self.eval(body);
                if !self.set_not_evaling(target) { unreachable!() } 
                res
            },
            
            &mut Inner::Unary(op, arg) => self.eval(arg).and_then(|val| val.apply_unary(op)),
            &mut Inner::Binary(op, arg1, arg2) => {
                self.eval(arg1).and_then(|val1|
                    self.eval(arg2).and_then(|val2|
                        val1.apply_binary(op, val2)
                    )
                )
            },
        }} else { Err(eval_err!("Unknown Thunk ID")) }
    }
}

