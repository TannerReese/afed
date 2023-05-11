// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use id_arena::{Arena, Id};
use std::cell::Cell;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::fmt::{Debug, Error, Formatter};
use std::hash::Hasher;
use std::slice::Iter;

use afed_objects::{array::Array, eval_err, map::Map, pkg::Pkg, Binary, Object, Unary};

extern crate id_arena;

pub mod func;
pub mod pattern;

use func::Func;
pub use pattern::Pattern;

#[derive(Debug, Clone)]
pub struct ExprArena(Arena<Node>, HashSet<String>);
pub type ExprId = Id<Node>;
type VarId = ExprId;
pub type ArgId = ExprId;

// Node within the Abstract Syntax Tree defined by `ExprArena`
pub struct Node {
    /* NOTE: `Cell` is needed for `simplifying` and `value`
     * since simplification by ref needs to be possible.
     */

    /* Whether the node is currently being simplified.
     * `Cell` is needed since simplification by ref needs to be possible.
     */
    simplifying: Cell<bool>,

    /* List of unbound variable definitions and unresolved
     * variable references in this node and its descendants
     */
    vars: Vec<VarId>,

    /* Cached value of node after evaluation
     * `Cell` is needed since simplification by ref needs to be possible.
     */
    value: Cell<Option<Object>>,

    /* If saved==false then the node will give up its value when requested.
     * Otherwise, the value will be cloned when requested.  This is set to
     * true for variables and equals statements since they may be used by
     * more nodes than just their parent.
     */
    saved: bool,

    inner: Inner,
}

// Indicate type and main contents of Node
#[derive(Debug, Clone)]
enum Inner {
    Const(Object),
    Array(Vec<ExprId>),
    Map {
        // If `Some(id)` then the value of this node is that of `target`
        target: Option<ExprId>,
        elems: HashMap<String, ExprId>,
    },

    // Effectively a pointer node to other nodes
    Var {
        name: String,
        // Reference to other node whose value this one takes
        target: Option<ExprId>,
        // Whether this variable node is the definition of the variable
        is_defn: bool,
    },

    // Destructure value and store the values in variable nodes
    // Ids for Pattern are the IDs of the associated DestructArgs
    Destruct(Pattern<String>, ExprId),

    Unary(Unary, ExprId),
    Binary(Binary, ExprId, ExprId),
    Access {
        // Reference to node whose value will be accessed / called
        caller: ExprId,
        // List of method names to call
        path: Vec<String>,
        // Arguments to method call
        args: Vec<ExprId>,
    },
    // Node whose cached value will be substituted by a function argument
    Arg(String),

    Func {
        // `None` when function is declared anonymously
        name: Option<String>,
        // Patterns used to match each argument
        pats: Vec<Pattern<ArgId>>,
        // Subtree that will be evaluted when this function is called
        // NOTE: The subtree can still have external references to the AST
        body: ExprId,
    },
}

impl Clone for Node {
    fn clone(&self) -> Self {
        let val = self.value.take();
        let cloned = val.clone();
        self.value.set(val);
        Node {
            simplifying: Cell::new(false),
            vars: self.vars.clone(),
            inner: self.inner.clone(),
            saved: self.saved,
            value: Cell::new(cloned),
        }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let val = self.value.take();
        let cloned = val.clone();
        self.value.set(val);
        write!(
            f,
            concat!(
                "Node {{ ",
                "vars: {:?}, inner: {:?}, ",
                "saved: {}, value: Cell({:?}) }}",
            ),
            self.vars, self.inner, self.saved, cloned,
        )
    }
}

impl ExprArena {
    fn get_argname(&self, id: ArgId) -> &str {
        if let Inner::Arg(argname) = &self.0[id].inner {
            argname
        } else {
            panic!("ArgId doesn't refer to an argument node")
        }
    }

    fn take_vars(&mut self, child: ExprId) -> Vec<VarId> {
        std::mem::take(&mut self.0[child].vars)
    }

    /* Fills in the target `ExprId` for every
     * variable node where `resolver` returns `Some(id)`
     */
    fn resolve<F>(&mut self, target: ExprId, mut resolver: F)
    where
        F: FnMut(&ExprArena, &str) -> Option<ExprId>,
    {
        let vars = self.take_vars(target);

        let unresolved = vars
            .into_iter()
            .filter(|&id| {
                let self_imm = &*self;
                let tgt;
                // Only variable nodes that aren't definitions can be resolved
                if let Inner::Var { name, is_defn, .. } = &self_imm.0[id].inner {
                    if *is_defn {
                        true
                    } else {
                        tgt = resolver(self_imm, name);
                        // `resolver` is mutating so we have to reacquire the ref
                        if let Inner::Var { target, .. } = &mut self.0[id].inner {
                            *target = tgt;
                        } else {
                            unreachable!()
                        } // It has to be a variable node
                        if let Some(tgt) = tgt {
                            self.0[tgt].saved = true;
                        }
                        tgt.is_none()
                    }
                } else {
                    panic!("VarId doesn't refer to a variable node")
                }
            })
            .collect();
        self.0[target].vars = unresolved;
    }

    /* Resolves variable nodes in `root` by looking up names in `bltns`.
     * Also, the elements of `bltns` are converted to objects and added to
     * the arena.
     */
    pub fn resolve_pkgs(&mut self, root: ExprId, pkgs: Pkg) {
        let mut globals = HashMap::new();
        match pkgs {
            Pkg::Const(_) => panic!("Cannot resolve against constant"),
            Pkg::Map(bltns) => {
                for (key, (_, pkg)) in bltns.into_iter() {
                    let id = self.expr_from_bltn(pkg, &mut globals);
                    if globals.insert(key, id).is_some() {
                        panic!("Redefinition of builtin package")
                    }
                }
            }
        };

        self.resolve(root, |_, key| globals.get(key).cloned());
    }
}

impl ExprArena {
    // Convert `bltn` to objects and add them to the arena
    fn expr_from_bltn(&mut self, bltn: Pkg, globals: &mut HashMap<String, ExprId>) -> ExprId {
        match bltn {
            Pkg::Const(obj) => self.from_obj(obj),
            Pkg::Map(elems) => {
                let elems = elems
                    .into_iter()
                    .map(|(key, (is_global, elem))| {
                        let id = self.expr_from_bltn(elem, globals);
                        // When an entry in `bltn` is global add it to the `globals`
                        if is_global && globals.insert(key.clone(), id).is_some() {
                            panic!("Redefinition of global builtin '{}'", &key)
                        }
                        (key, id)
                    })
                    .collect();
                self.create_node(
                    vec![],
                    Inner::Map {
                        target: None,
                        elems,
                    },
                )
            }
        }
    }
}

impl Default for ExprArena {
    fn default() -> Self {
        Self::new()
    }
}

/* Abstract Syntax Tree for the parsed code.  For example, references to
 * variables or function arguments can create references between branches
 * of the tree.  Because of this, all nodes of the tree must be maintained
 * in memory.
 */
impl ExprArena {
    pub fn new() -> Self {
        Self(Arena::new(), HashSet::new())
    }

    // Helper method for other create methods
    fn create_node(&mut self, vars: Vec<VarId>, inner: Inner) -> ExprId {
        self.0.alloc(Node {
            simplifying: Cell::new(false),
            vars,
            inner,
            saved: false,
            value: Cell::new(None),
        })
    }

    // Used by `create_map` to remove and return the unbound defns from `vars`
    fn sift_defns(&mut self, exp: ExprId) -> Vec<VarId> {
        let mut defns = Vec::new();
        let vars = self
            .take_vars(exp)
            .into_iter()
            .filter(|&id| {
                if let Inner::Var { is_defn, .. } = &self.0[id].inner {
                    if *is_defn {
                        defns.push(id);
                    }
                    !*is_defn
                } else {
                    panic!("VarId doesn't refer to a variable node")
                }
            })
            .collect();
        self.0[exp].vars = vars;
        defns
    }

    // Used for parsing map members in `ParsingContext::map_member`
    pub fn get_defns(&self, exp: ExprId) -> Vec<String> {
        self.0[exp]
            .vars
            .iter()
            .filter_map(|&id| {
                if let Inner::Var { name, is_defn, .. } = &self.0[id].inner {
                    if *is_defn {
                        Some(name.clone())
                    } else {
                        None
                    }
                } else {
                    panic!("VarId doesn't refer to a variable node")
                }
            })
            .collect()
    }

    // Create a string not used before for a variable
    fn make_unique_name(&self) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write_usize(self.1.len());
        loop {
            hasher.write_usize(0); // Make sure the hash changes every loop
            let hash = hasher.finish();
            let unique = format!("UNIQUE<{}>", hash);
            if !self.1.contains(&unique) {
                break unique;
            }
        }
    }

    pub fn create_array(&mut self, elems: Vec<ExprId>) -> ExprId {
        let vars = elems.iter().flat_map(|&id| self.take_vars(id)).collect();
        self.create_node(vars, Inner::Array(elems))
    }

    // Collect all unbound definitions from `elems` and make into a map node
    // If a definition with an empty name is present, that becomes the target
    pub fn create_map(&mut self, elems: Vec<ExprId>) -> ExprId {
        // Collect unbound definitions into `entries` map
        let mut entries = HashMap::new();
        let mut target = None;
        for &id in elems.iter() {
            for var_id in self.sift_defns(id) {
                if let Inner::Var {
                    name,
                    is_defn,
                    target: Some(id),
                } = &mut self.0[var_id].inner
                {
                    if *is_defn {
                        // Optional map target is indicated using empty variable name
                        if name.is_empty() {
                            if target.is_none() {
                                target = Some(*id);
                            } else {
                                panic!("Redefinition of map target");
                            }
                        } else if entries.insert(name.clone(), *id).is_some() {
                            panic!("Redefinition of label in map");
                        } else {
                            *is_defn = false;
                        }
                        continue;
                    }
                }
                panic!("VarId doesn't refer to a definition");
            }
        }

        // Resolve the unresolved variables in every element
        let vars = elems
            .iter()
            .flat_map(|&id| {
                self.resolve(id, |_, path| entries.get(path).cloned());
                // Collect unresolved variables into this node
                self.take_vars(id)
            })
            .collect();
        self.create_node(
            vars,
            Inner::Map {
                target,
                elems: entries,
            },
        )
    }

    pub fn create_var(&mut self, name: String) -> VarId {
        self.1.insert(name.clone());
        self.0.alloc_with_id(|id| Node {
            simplifying: Cell::new(false),
            vars: vec![id],
            saved: true,
            value: Cell::new(None),
            inner: Inner::Var {
                name,
                is_defn: false,
                target: None,
            },
        })
    }

    pub fn create_defn(&mut self, name: String, body: ExprId) -> VarId {
        let mut vars = self.take_vars(body);
        self.1.insert(name.clone());
        self.0.alloc_with_id(|id| {
            vars.push(id);
            Node {
                simplifying: Cell::new(false),
                vars,
                saved: true,
                value: Cell::new(None),
                inner: Inner::Var {
                    name,
                    is_defn: true,
                    target: Some(body),
                },
            }
        })
    }

    fn create_destruct(&mut self, pat: Pattern<String>, body: ExprId) -> ExprId {
        let vars = self.take_vars(body);
        self.create_node(vars, Inner::Destruct(pat, body))
    }

    pub fn create_defn_with_pat(&mut self, pat: Pattern<String>, body: ExprId) -> VarId {
        let arg_set = pat.arg_set();
        let dstr = self.create_destruct(pat, body);
        let unique_name = self.make_unique_name();
        let dstr_defn = self.create_defn(unique_name.clone(), dstr);

        for arg in arg_set.into_iter() {
            let dstr_ref = self.create_var(unique_name.clone());
            let access = self.create_access(dstr_ref, vec![arg.clone()]);
            let var_id = self.create_defn(arg, access);

            let mut vars = self.take_vars(var_id);
            self.0[dstr_defn].vars.append(&mut vars);
        }
        dstr_defn
    }

    pub fn create_unary(&mut self, op: Unary, arg: ExprId) -> ExprId {
        let vars = self.take_vars(arg);
        self.create_node(vars, Inner::Unary(op, arg))
    }

    pub fn create_binary(&mut self, op: Binary, arg1: ExprId, arg2: ExprId) -> ExprId {
        let mut vars = self.take_vars(arg1);
        vars.append(&mut self.take_vars(arg2));
        self.create_node(vars, Inner::Binary(op, arg1, arg2))
    }

    pub fn create_access(&mut self, exp: ExprId, path: Vec<String>) -> ExprId {
        self.create_call(exp, path, vec![])
    }

    pub fn create_call(&mut self, caller: ExprId, path: Vec<String>, args: Vec<ExprId>) -> ExprId {
        if path.is_empty() && args.is_empty() {
            return caller;
        }
        let mut vars = self.take_vars(caller);
        vars.extend(args.iter().flat_map(|&id| self.take_vars(id)));
        self.create_node(vars, Inner::Access { caller, path, args })
    }

    pub fn create_func(
        &mut self,
        name: Option<String>,
        pats: Vec<Pattern<String>>,
        body: ExprId,
    ) -> ExprId {
        // Convert patterns into argument nodes
        let mut args = Vec::new();
        let pats = pats
            .into_iter()
            .map(|p| {
                p.into_map(|nm| {
                    let id = self.create_node(vec![], Inner::Arg(nm));
                    args.push(id);
                    id
                })
            })
            .collect::<Vec<Pattern<ArgId>>>();

        // Resolve variables in body of function
        self.resolve(body, |ar, nm| {
            args.iter()
                .filter_map(|&id| {
                    if nm == ar.get_argname(id) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .next()
        });
        let vars = self.take_vars(body);
        self.create_node(vars, Inner::Func { name, pats, body })
    }

    // Add node to arena for `obj`
    // Special treatment for `Map`s and `Array`s. They are destructured
    pub fn from_obj(&mut self, obj: Object) -> ExprId {
        let inner = if obj.is_a::<Map>() {
            let Map(elems) = obj.cast::<Map>().unwrap();
            Inner::Map {
                target: None,
                elems: elems
                    .into_iter()
                    .map(|(key, child)| (key, self.from_obj(child)))
                    .collect(),
            }
        } else if obj.is_a::<Array>() {
            let Array(elems) = obj.cast::<Array>().unwrap();
            Inner::Array(
                elems
                    .into_iter()
                    .map(|child| self.from_obj(child))
                    .collect(),
            )
        } else {
            Inner::Const(obj)
        };

        self.0.alloc(Node {
            simplifying: Cell::new(false),
            vars: vec![],
            inner,
            saved: false,
            value: Cell::new(None),
        })
    }

    pub fn create_obj<T>(&mut self, obj: T) -> ExprId
    where
        T: Into<Object>,
    {
        self.from_obj(obj.into())
    }

    pub fn set_saved(&mut self, exp: ExprId) {
        self.0[exp].saved = true;
    }
}

impl ExprArena {
    // Iteratively follow variable references to find first non-variable node
    fn get_node(&self, mut exp: ExprId) -> &Node {
        // Track visited Var nodes to prevent circular dependency
        let mut var_ids = HashSet::new();
        var_ids.insert(exp);
        loop {
            let node = &self.0[exp];
            if let Some(obj) = node.value.take() {
                node.value.set(Some(obj));
                return node;
            } else if let Inner::Var {
                target: Some(id), ..
            } = &node.inner
            {
                exp = *id;
                if !var_ids.insert(exp) {
                    node.value.set(Some(eval_err!("Circular dependency")));
                    return node;
                }
            } else {
                return node;
            }
        }
    }

    // Only copy when value needs to be saved to avoid extraneous allocation
    fn take(&self, exp: ExprId) -> Object {
        let node = self.get_node(exp);
        let obj = node.value.take();
        if !node.saved {
            obj
        } else {
            let cloned = obj.clone();
            node.value.set(obj);
            cloned
        }
        .unwrap()
    }

    fn has_value(&self, exp: ExprId) -> bool {
        let node = self.get_node(exp);
        if let Some(obj) = node.value.take() {
            node.value.set(Some(obj));
            true
        } else {
            false
        }
    }

    /* Iteratively follow path through map nodes.  This prevents costly
     * copies of an entire map to retrieve one element of it.
     */
    fn access(&self, mut exp: ExprId, path: &mut Iter<'_, String>) -> ExprId {
        while let Some(key) = path.as_slice().get(0) {
            match &self.0[exp].inner {
                Inner::Map {
                    target: None,
                    elems,
                } => {
                    if let Some(&id) = elems.get(key) {
                        path.next();
                        exp = id;
                    } else {
                        break;
                    }
                }
                Inner::Var {
                    target: Some(id), ..
                } => {
                    exp = *id;
                }
                _ => break,
            }
        }
        exp
    }

    /* Try to simplify a node and all of its descendants into constants.
     * Full evaluation is not always possible when unassigned arg nodes
     * are present.  This happens when we are trying to simplify the body
     * of a function that hasn't been called yet.
     *
     * If successfully simplified to a constant then true is returned.
     * This value is stored in the `value` field of the `Node` struct.
     */
    fn simplify(&self, args_used: &mut Vec<ArgId>, exp: ExprId) -> bool {
        // If node is already simplified leave
        if self.has_value(exp) {
            return true;
        }
        let node = &self.0[exp];
        // Check for cycles in the call stack
        if node.simplifying.get() {
            node.value.set(Some(eval_err!("Circular dependency")));
            return true;
        } else {
            node.simplifying.set(true)
        }

        // Call `simplify` and return if the node simplifies to an error
        macro_rules! simplify {
            ($child:ident) => {{
                let is_const = self.simplify(args_used, $child);
                let child_node = self.get_node($child);
                if let Some(obj) = child_node.value.take() {
                    if obj.is_err() {
                        node.value.set(Some(obj));
                        node.simplifying.set(false);
                        return true;
                    }
                    child_node.value.set(Some(obj));
                }
                is_const
            }};
        }

        // `obj` is `Some` when the node simplifies to a constant
        let obj = match &node.inner {
            /* Most of these cases try to simplify the children nodes.
             * If all simplify successfully, the values are all taken with
             * `take` and combined to create the result.
             */
            Inner::Const(obj) => Some(obj.clone()),
            // Try to simplify ever element in the array
            Inner::Array(elems) => {
                let elems = elems.clone();
                let mut cnst = true;
                for &id in elems.iter() {
                    cnst &= simplify!(id);
                }

                if !cnst {
                    None
                } else {
                    let elems = elems.iter().map(|&id| self.take(id)).collect();
                    Some(Object::new(Array(elems)))
                }
            }

            // Try to simplify every value in the map
            Inner::Map {
                target: None,
                elems,
            } => {
                let elems = elems.clone();
                let mut cnst = true;
                for &id in elems.values() {
                    cnst &= simplify!(id);
                }

                if !cnst {
                    None
                } else {
                    let elems = elems
                        .into_iter()
                        .map(|(key, id)| (key, self.take(id)))
                        .collect();
                    Some(Object::new(Map(elems)))
                }
            }

            // Only need to simplify the target expression
            &Inner::Map {
                target: Some(id), ..
            } => {
                if simplify!(id) {
                    Some(self.take(id))
                } else {
                    None
                }
            }

            /* Var nodes are treated differently because of the `get_node`
             * method.  The `value` field of `target` isn't taken, because
             * when `take` is called on the top variable it will be iterate
             * down through all the variables and take the value from
             * the bottom.  Because var nodes are saved if this wasn't
             * done, many needless copies of the value would be created
             * while simplifying.
             */
            Inner::Var { name, target, .. } => {
                if let Some(target) = *target {
                    let res = self.simplify(args_used, target);
                    node.simplifying.set(false);
                    return res;
                } else {
                    Some(eval_err!("Unresolved name \"{}\"", name))
                }
            }

            // Try to simplify argument and then destructure it and make a map
            // Its value isn't set. Instead it sets the values for its DestructArgs
            Inner::Destruct(pattern, target) => {
                let target = *target;
                if simplify!(target) {
                    let mut args = HashMap::new();
                    if let Err(err) = pattern.match_args(
                        &mut |name, obj| {
                            args.insert(name.clone(), obj);
                        },
                        self.take(target),
                    ) {
                        Some(err)
                    } else {
                        Some(Object::new(Map(args)))
                    }
                } else {
                    None
                }
            }

            // Try to simplify argument and apply unary operator
            &Inner::Unary(op, arg) => {
                if simplify!(arg) {
                    Some(self.take(arg).unary(op))
                } else {
                    None
                }
            }

            // Try to simplify arguments and apply binary operator
            &Inner::Binary(op, arg1, arg2) => {
                if simplify!(arg1) & simplify!(arg2) {
                    Some(self.take(arg1).binary(op, self.take(arg2)))
                } else {
                    None
                }
            }

            Inner::Access { caller, path, args } => {
                let (caller, mut path) = (*caller, path.iter());
                let args = args.clone();
                // Try to resolve the path as far as possible before evaling
                // To avoid needless copies of maps
                let caller = self.access(caller, &mut path);

                // Try to simplify caller and arguments
                let mut cnst = simplify!(caller);
                for &id in args.iter() {
                    cnst &= simplify!(id);
                }

                if !cnst {
                    None
                } else {
                    // Take values from the caller and arguments
                    let args: Vec<Object> = args.into_iter().map(|id| self.take(id)).collect();
                    let tgt_node = self.get_node(caller);

                    // Call the caller with the remaining path
                    if let Some(obj) = tgt_node.value.take() {
                        let path: Vec<&str> = path.map(|s| s.as_str()).collect();
                        let res = if args.is_empty() && path.is_empty() {
                            obj.clone()
                        } else {
                            obj.call_path(path, args)
                        };

                        tgt_node.value.set(Some(obj));
                        Some(res)
                    } else {
                        panic!("No value found for function")
                    }
                }
            }

            // If it had a value it would be returned at the top
            Inner::Arg(_) => {
                args_used.push(exp);
                None
            }

            /* When function is simplified it creates a new `expr::Func`
             * object with its own `ExprArena`
             */
            Inner::Func { pats, body, .. } => {
                let mut used = Vec::new();
                // Simplify the `body` to remove external variable references
                self.simplify(&mut used, *body);
                /* If the body still relies on unassigned arguments (i.e. to
                 * functions outside this one) then the function still has
                 * outside connections and can't be simplified.
                 */
                let cnst = used.iter().all(|id| pats.iter().any(|p| p.contains(id)));

                if !cnst {
                    None
                } else {
                    // Create arena and clone the structure of `body` into it
                    let mut new_arena = ExprArena::new();
                    let func = self.clone_into(&mut new_arena, exp);
                    if let Inner::Func { name, pats, body } = &mut new_arena.0[func].inner {
                        let name = std::mem::take(name);
                        let pats = std::mem::take(pats);
                        Some(Func::create(name, pats, *body, new_arena))
                    } else {
                        unreachable!()
                    }
                }
            }
        };

        node.simplifying.set(false);
        let is_cnst = obj.is_some();
        node.value.set(obj);
        is_cnst
    }

    // Call `simplify` on `exp` and make sure its simplifies successfully
    pub fn eval(&self, exp: ExprId) -> Object {
        let mut args_used = Vec::new();
        if self.simplify(&mut args_used, exp) {
            self.take(exp)
        } else {
            let mut argnames = String::new();
            let mut is_first = true;
            for &id in args_used.iter() {
                if !is_first {
                    argnames += ", ";
                }
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
    // Clear the cached value of every node
    pub fn clear_cache(&self) {
        for (_, node) in self.0.iter() {
            node.value.set(None);
        }
    }

    // Used when calling `expr::Func`  to set the value of arg nodes
    pub fn set_arg(&self, arg: ArgId, value: Object) {
        let node = &self.0[arg];
        if let Inner::Arg(_) = node.inner {
            node.value.set(Some(value));
        } else {
            panic!("ArgId doesn't refer to an argument node")
        }
    }

    // Used by the `Inner::Func` case in `simplify` to construct a new arena
    fn clone_into(&self, arena: &mut ExprArena, exp: ExprId) -> ExprId {
        if self.has_value(exp) {
            return arena.from_obj(self.take(exp));
        }

        match &self.0[exp].inner {
            /* Most cases reconstruct their children using `clone_into`
             * recursively and then create a copy of themselves providing the
             * children IDs. During this variable and argument nodes need to
             * be re-resolved since those connections cannot be easily copied.
             */
            Inner::Const(obj) => arena.from_obj(obj.clone()),
            Inner::Array(elems) => {
                let elems = elems.iter().map(|id| self.clone_into(arena, *id)).collect();
                arena.create_array(elems)
            }
            Inner::Map { target, elems } => {
                let target = target.map(|id| self.clone_into(arena, id));
                let elems = elems
                    .iter()
                    .map(|(key, id)| (key.clone(), self.clone_into(arena, *id)))
                    .collect::<HashMap<String, ExprId>>();

                // Re-resolve the unresolved variables
                let vars = target
                    .iter()
                    .chain(elems.values())
                    .flat_map(|&id| {
                        arena.resolve(id, |_, path| elems.get(path).cloned());
                        arena.take_vars(id)
                    })
                    .collect();
                // We don't need to re-separate definitions
                // So we don't need to use `create_map`
                arena.create_node(vars, Inner::Map { target, elems })
            }

            // The `target` is no longer valid in the new arena
            Inner::Var { name, .. } => arena.create_var(name.clone()),
            Inner::Destruct(pattern, target) => {
                let target = self.clone_into(arena, *target);
                arena.create_destruct(pattern.clone(), target)
            }

            &Inner::Unary(op, arg) => {
                let arg = self.clone_into(arena, arg);
                arena.create_unary(op, arg)
            }
            &Inner::Binary(op, arg1, arg2) => {
                let arg1 = self.clone_into(arena, arg1);
                let arg2 = self.clone_into(arena, arg2);
                arena.create_binary(op, arg1, arg2)
            }

            Inner::Access { caller, path, args } => {
                let args = args.iter().map(|&id| self.clone_into(arena, id)).collect();

                // Check if cached value exists for caller
                let mut iter = path.iter();
                let new_caller = self.access(*caller, &mut iter);
                if self.has_value(new_caller) {
                    let new_caller = arena.from_obj(self.take(new_caller));
                    let new_path = iter.cloned().collect();
                    arena.create_call(new_caller, new_path, args)
                } else {
                    let new_caller = self.clone_into(arena, *caller);
                    arena.create_call(new_caller, path.clone(), args)
                }
            }

            Inner::Arg(name) => arena.create_var(name.clone()),
            Inner::Func { name, pats, body } => {
                let body = self.clone_into(arena, *body);
                // Convert Arg IDs beack into string slices
                let pats = pats
                    .iter()
                    .map(|p| p.map(|&id| self.get_argname(id).to_owned()))
                    .collect();
                arena.create_func(name.clone(), pats, body)
            }
        }
    }
}
