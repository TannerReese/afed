macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError::create(format!($($arg)*)) };
}

macro_rules! count_tt {
    () => { 0 };
    ($fst:tt $($item:tt)*) => {1 + count_tt!($($item)*)};
}

// Calls `Object` with given method
macro_rules! call {
    ($obj:ident ($($arg:expr),*)) => { call!(($obj)($($arg),*)) };
    (($obj:expr)($($arg:expr),*)) =>
        { $obj.call(None, vec![$($arg.into()),*]) };

    ($obj:ident.$attr:ident) => { call!(($obj).$attr()) };
    (($obj:expr).$attr:ident) => { call!(($obj).$attr()) };
    ($obj:ident.$method:ident ($($arg:expr),*)) =>
        { call!(($obj).$method($($arg),*)) };
    (($obj:expr).$method:ident ($($arg:expr),*)) =>
        { $obj.call(Some(stringify!($method)), vec![$($arg.into()),*]) };
}

// Quickly give name to a type so `Objectish` can be implemented
macro_rules! name_type {
    ($name:ident: $tp:ty) => {
        name_type! {stringify!($name), $tp}
    };
    ($name:literal: $tp:ty) => {
        name_type! {$name, $tp}
    };
    ($name:expr, $tp:ty) => {
        impl NamedType for $tp {
            fn type_name() -> &'static str {
                $name
            }
        }
    };
}

/* Ergonomically implement the unary and binary operations as well as
 * method calls on a type so `Objectish` can be implemented.
 *
 * An impl block is created for the type containing all of the method
 * declarations (including those labelled with `#[call]`).  All meta
 * attributes (besides those used by impl_operable!)  and visibility modifiers
 * are preserved.  As such, methods in `impl_operable!` can collide with
 * other methods declared on the type.  There may be at most one method
 * declaration with `#[call]`.  It may have any valid identifier as a name.
 *
 * Operator declarations are identified by a `#[unary(<Unary>)]` or
 * `#[binary(<Binary>)]` attribute. Here <Unary> and <Binary> are valid
 * cases of the enums `Unary` and `Binary`, respectively.  Further,
 * binary operator declarations may optionally include `rev` or `comm`
 * (e.g. `#[binary(Add,comm)]`). `rev` indicates that this declaration
 * accepts only arguments in the reverse order.  `comm` indicates that this
 * declaration accepts either normal or reverse order arguments.  Multiple
 * declarations can be given for a single operator.  The first one matching
 * the types are order will be used.  The attribute `#[exclude(<type-1>,...)]`
 * can be used to prevent the second argument from matching against any of
 * the types <type-1>, ...  The name used in the signature of an operator
 * declaration is irrelevant and need only be a valid token-tree (e.g. `_`).
 *
 * Example:
 * Here we are assuming `MyType` can be cast to and from `u8`.
 * ```
 *   impl_operable!{MyType:
 *       // Operator declarations
 *       #[binary(Neg)]
 *       #[exclude(Number)]
 *       fn _(x: u8) -> u8 { -x }
 *       #[binary(Add)]
 *       fn _(x: u8, y: u8) -> u8 { x + y }
 *
 *       // Method declarations
 *       #[call]
 *       fn call__(self, shf: u8) -> u8 { u8::from(self) << shf }
 *       pub fn times_three(&self) -> u8 { u8::from(self) * 3 }
 *   }
 * ```
 */
macro_rules! impl_operable {
    /* Create cold to handle a unary operator in `unary` method
     * of `Operable` trait for operable declarations
     */
    (@una #[unary($name:ident)] $(#$meta:tt)*, (), $vars:tt) =>
        { impl_operable!{@una $(#$meta)*, ($name), $vars} };
    (@una #[$_:meta] $(#$meta:tt)*, $una:tt, $vars:tt) =>
        { impl_operable!{@una $(#$meta)*, $una, $vars} };
    (@una , (), $vars:tt) => {};
    (@una , ($name:ident), ($self:ident, $op:ident,
        ($id:ident : $tp:ty) -> $ret:ty $block:block
    )) => {
        #[allow(clippy::redundant_closure_call)]
        if let Unary::$name = $op {
            let ret: $ret = (|$id: $tp| $block)($self.into());
            return Some(ret.into());
        }
    };



    /* Create code to handle a binary operator in `binary` method
     * of `Operable` trait for operator declarations
     */
    (@bin #[binary(comm, $n:ident)] $(#$meta:tt)*, (), $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, ($n, true, true), $e, $vars} };
    (@bin #[binary(rev, $n:ident)] $(#$meta:tt)*, (), $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, ($n, false, true), $e, $vars} };
    (@bin #[binary($n:ident, comm)] $(#$meta:tt)*, (), $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, ($n, true, true), $e, $vars} };
    (@bin #[binary($n:ident, rev)] $(#$meta:tt)*, (), $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, ($n, false, true), $e, $vars} };
    (@bin #[binary($n:ident)] $(#$meta:tt)*, (), $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, ($n, true, false), $e, $vars} };

    (@bin #[exclude $ts:tt] $(#$meta:tt)*, $b:tt, (), $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, $b, $ts, $vars} };

    (@bin #[$_:meta] $(#$meta:tt)*, $b:tt, $e:tt, $vars:tt) =>
        { impl_operable!{@bin $(#$meta)*, $b, $e, $vars} };

    (@bin , (), (), $vars:tt) => {};
    (@bin ,
        ($name:ident, $allow_unrev:literal, $allow_rev:literal),
        ($($excl_tp:ty),*),
        ($self:ident, $rev:ident, $op:ident, $other:ident,
            ($v1:ident : $t1:ty, $v2:ident : $t2:ty) -> $ret:ty $block:block
        )
    ) => { if let Binary::$name = $op {
        if (!$rev && $allow_unrev) || ($rev && $allow_rev) {
            $(match $other.try_cast::<$excl_tp>() {
                Ok(other) => return Err((Object::new($self), other.into())),
                Err(other) => { $other = other },
            })*
            #[allow(clippy::redundant_closure_call)]
            match $self.try_into() {
                Ok(self_) => match $other.try_cast() {
                    Ok(other) => {
                        // Closure must be used to make 'return's work inside $block
                        let ret: $ret = (|$v1: $t1, $v2: $t2| $block)(
                            self_, other
                        );
                        return Ok(ret.into());
                    },
                    Err(other) => {
                        $other = other;
                        $self = self_.into();
                    },
                },
                Err(self_) => { $self = self_.into() },
            }
        }
    }};



    // Helper sub-macro for calling `@arity` and `@call`
    // Filters out all operator declarations
    (@method #[call] $(#$meta:tt)*, (), $vars:tt) =>
        { impl_operable!{@method $(#$meta)*, (call), $vars} };
    (@method #[unary $_:tt] $(#$meta:tt)*, $c:tt, $vs:tt) => {};
    (@method #[binary $_:tt] $(#$meta:tt)*, $c:tt, $vs:tt) => {};
    (@method #[exclude $_:tt] $(#$meta:tt)*, $c:tt, $vs:tt) => {};
    (@method #$_:tt $(#$meta:tt)*, $is_call:tt, $vars:tt) =>
        { impl_operable!{@method $(#$meta)*, $is_call, $vars} };
    (@method , (), (@$method:ident $func:ident, $vars:tt)) =>
        { impl_operable!{@$method Some(stringify!($func)), $vars} };
    (@method , (call), (@$method:ident $func:ident, $vars:tt)) =>
        { impl_operable!{@$method None, $vars} };


    // Create code to return arity for a method based on the method header
    (@arity $attr_pat:pat, ($attr:ident,
        ($_:pat $(, $arg:ident : $tp:ty)*)
    )) => { if let $attr_pat = $attr {
        return Some(count_tt!($($arg)*));
    }};

    /* Method calls on `Object`s are always call-by-reference
     * so `@get_self_ref` clones `self` if the method being
     * called was written to take ownership of `self`.
     */
    (@self_from_ref $self:ident, (self $($_:tt)*)) => { $self.clone() };
    (@self_from_ref $self:ident, (&self $($_:tt)*)) => { $self };

    /* Create code to cast arguments and
     * call a method for all method declarations
     */
    (@call $attr_pat:pat, ($self:ident, $attr:ident,
        $argvec:ident, $func_args:tt,
        $func:ident ($_:pat $(, $arg:ident : $tp:ty)*) -> $ret:ty $block:block
    )) => { if let $attr_pat = $attr {
        $(let $arg = match $argvec.remove(0).cast() {
            Ok(value) => value,
            Err(err) => return err,
        };)*
        return impl_operable!(@self_from_ref $self, $func_args)
            .$func($($arg),*).into()
    }};


    /* `@help` creates code to generate the help messages
     * for operator and method headers
     */
    (@help #[doc=$doc:literal] $(#$meta:tt)*,
        $is_oper:tt, $help:expr, $vars:tt
    ) => { impl_operable!{@help $(#$meta)*,
        $is_oper, concat!($help, "\n", $doc), $vars
    }};
    (@help #[call] $(#$meta:tt)*, (), $help:expr, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #[unary $_:tt] $(#$meta:tt)*, (), $help:expr, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #[binary $_:tt] $(#$meta:tt)*, (), $help:expr, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #$_:tt $(#$meta:tt)*, $is_oper:tt, $help:expr, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, $is_oper, $help, $vars} };
    (@help , (), $help:expr, (
        $opers:ident, $methods:ident, $attr:ident, $func:tt
    )) => {
        if $attr == None { if let Some(sig) = $help.trim().lines().next() {
            if sig.trim().len() > 0 {
                $methods += "\n";  $methods += sig;
            }
        }}

        if $attr == Some(stringify!($func)) {
            return Some($help.trim().to_owned());
        }
    };
    (@help , (oper), $help:expr, (
        $opers:ident, $methods:ident, $attr:ident, $func:tt
    )) => { if $attr == None { if $help.trim().len() > 0 {
        $opers += "\n";  $opers += $help;
    }}};



    /* `@method_impl` strips meta attributes used by `impl_operable!`
     * and filter out operator declarations
     * before they are placed into the impl block
     */
    (@method_impl #[call] $(#$meta:tt)*, $mlist:tt, $vars:tt) =>
        { impl_operable!{@method_impl $(#$meta)*, $mlist, $vars} };
    (@method_impl #[unary $_:tt] $(#$meta:tt)*, $mlist:tt, $vars:tt) => {};
    (@method_impl #[binary $_:tt] $(#$meta:tt)*, $mlist:tt, $vars:tt) => {};
    (@method_impl #[exclude $_:tt] $(#$meta:tt)*, $mlist:tt, $vars:tt) => {};
    (@method_impl #$any:tt $(#$meta:tt)*, $mlist:tt, $vars:tt) =>
        { impl_operable!{@method_impl $(#$meta)*, (#$any $mlist), $vars} };
    (@method_impl , (#$next:tt $mlist:tt), ($decl:tt $(#$meta:tt)*)) =>
        { impl_operable!{@method_impl , $mlist, ($decl $(#$meta)* #$next)} };
    (@method_impl , (), (
        ( $vis:vis fn $func:ident $args:tt -> $ret:ty $block:block )
        $(#$meta:tt)*
    )) => { $(#$meta)* $vis fn $func $args -> $ret $block };



    ($Self:ty , $desc:expr , $(
        $(#$meta:tt)*
        $vis:vis fn $func:tt $args:tt -> $ret:ty $block:block
    )*) => {
        impl $Self { $(impl_operable!{@method_impl $(#$meta)*, (), ((
            $vis fn $func $args -> $ret $block
        ))})* }

        impl From<::std::convert::Infallible> for $Self {
            fn from(_: ::std::convert::Infallible) -> Self { panic!() }
        }

        impl Operable for $Self {
            fn arity(&self, _attr: Option<&str>) -> Option<usize> {
                $(impl_operable!{@method $(#$meta)*, (), (
                    @arity $func, (_attr, $args)
                )})*
                return None
            }

            fn help(&self, attr: Option<&str>) -> Option<String> {
                let mut _methods = String::new();
                let mut _opers = String::new();

                $(impl_operable!{@help $(#$meta)*, (), "",
                    (_opers, _methods, attr, $func)
                })*

                return if let None = attr { Some(format!(concat!(
                    "{}:\n", $desc, "\n\nOperators:{}\n\nMethods:{}"
                ), <$Self>::type_name(), _opers, _methods))} else { None }
            }

            #[allow(unused_mut)]
            fn call(&self,
                _attr: Option<&str>, mut _args: Vec<Object>
            ) -> Object {
                $(impl_operable!{@method $(#$meta)*, (), (
                    @call $func, (self, _attr,
                        _args, $args,
                        $func $args -> $ret $block
                    )
                )})*
                panic!()
            }


            #[allow(unused_mut)]
            fn unary(mut self, _op: Unary) -> Option<Object> {
                $(impl_operable!{@una $(#$meta)*, (),
                    (self, _op, $args -> $ret $block)
                })*
                return None
            }

            #[allow(unused_mut)]
            fn binary(mut self,
                _rev: bool, _op: Binary, mut _other: Object
            ) -> Result<Object, (Object, Object)> {
                $(impl_operable!{@bin $(#$meta)*, (), (), (
                    self, _rev, _op, _other,
                        $args -> $ret $block
                )})*
                Err((Object::new(self), _other))
            }
        }
    };

    ($Self:ty: #![doc=$desc:expr] $($rest:tt)*) =>
        { impl_operable!{$Self, $desc, $($rest)*} };
    ($Self:ty, $desc:expr, #![doc=$desc2:expr] $($rest:tt)*) =>
        { impl_operable!{$Self, concat!($desc, "\n", $desc2), $($rest)*} };

    ($Self:ty : $($(#$meta:tt)*
        $vis:vis fn $func:tt $args:tt -> $ret:ty $block:block
    )*) => { impl_operable!{$Self, "", $(
        $(#$meta)* $vis fn $func $args -> $ret $block
    )*} };
}

/* Used by library code to create a `Bltn::Map` instance
 * which will be named `$pkg`. It will contain constants and
 * functions corresponding to the function declarations provided.
 *
 * `create_bltns!` will also create a impl block containing
 * declarations for all functions with more than zero arguments.
 * NOTE: Functions cannot be genericized
 *
 * Example:
 * ```
 *   create_bltns!{package_name:
 *       fn my_constant() -> f32 { 0.57 }
 *       fn foo(x: usize) -> usize { x + 1 }
 *       pub fn bar(v: Vec<Object>) -> usize { v.len() }
 *   }
 * ```
 */
macro_rules! create_bltns {
    /* `@func` converts the method headers into
     * `Bltn::Const` if they have no arguments
     *  and `BltnFunc` instances otherwise
     */
    (@func #[global] $(#$m:tt)*, $help:expr, $_:expr, $vars:tt) =>
        { create_bltns!{@func $(#$m)*, $help, true, $vars} };
    (@func #[doc=$h:expr] $(#$m:tt)*, "", $g:expr, $vars:tt) =>
        { create_bltns!{@func $(#$m)*, $h, $g, $vars} };
    (@func #[doc=$h:expr] $(#$m:tt)*, $help:expr, $g:expr, $vars:tt) =>
        { create_bltns!{@func $(#$m)*, concat!($help, "\n", $h), $g, $vars} };
    (@func #[$_:meta] $($rest:tt)*) => { create_bltns!{@func $($rest)*} };

    (@func , $help:expr, $is_global:expr, (
        $pkg:ident, $name:expr, $func:ident () -> $ret:ty $block:block
    )) => { if $pkg.insert(stringify!($func).to_owned(),
        ($is_global, {
            let val: $ret = $block;
            Bltn::Const(val.into())
        })
    ).is_some() { panic!(
        concat!($name, '.', stringify!($func), " redeclared")
    )}};

    (@func , $help:expr, $is_global:expr, ($pkg:ident, $name:expr,
        $func:ident ($($arg:ident : $tp:ty),+) -> $ret:ty $block:block
    )) => { if $pkg.insert(stringify!($func).to_owned(),
        ($is_global, Bltn::Const(BltnFunc::create(
            concat!($name, '.', stringify!($func)), $help,
            |args: [Object; count_tt!($($arg)+)]| {
                let [$($arg),+] = args;
                $func($(match $arg.cast() {
                    Err(err) => return err,
                    Ok(val) => val,
                }),+).into()
            }
        )))
    ).is_some() { panic!(
        concat!($name, '.', stringify!($func), " redeclared")
    )}};


    /* `@strip_meta` removes this macros attributes from
     * the method headers before they're placed in their impl block
     */
    (@strip_meta #[global] $(#$meta:tt)*, $(#$new_meta:tt)*, $vars:tt) =>
        { create_bltns!{@strip_meta $(#$meta)*, $(#$new_meta)*, $vars} };
    (@strip_meta #$m:tt $(#$meta:tt)*, $(#$new_meta:tt)*, $vars:tt) =>
        { create_bltns!{@strip_meta $(#$meta)*, $(#$new_meta)* #$m, $vars} };

    (@strip_meta , $(#$meta:tt)*, ($vis:vis fn
        $func:ident () -> $ret:ty $block:block
    )) => {};
    (@strip_meta , $(#$meta:tt)*, ($vis:vis fn
        $func:ident ($($arg:ident : $tp:ty),+) -> $ret:ty $block:block
    )) => { $(#$meta)* $vis fn $func ($($arg : $tp),+) -> $ret $block };



    ($pkg:ident($name:expr), $make_bltns:ident,
        mod {$(
            $(#[global] $($is_global:literal)?)? $mod:ident : $modval:expr
        ),* $(,)?} :
        $($(#$meta:tt)* $vis:vis fn $func:ident $args:tt -> $ret:ty $block:block)*
    ) => {
        $(create_bltns!{@strip_meta $(#$meta)*, ,
            ($vis fn $func $args -> $ret $block)
        })*

        pub fn $make_bltns() -> Bltn {
            let mut $pkg = ::std::collections::HashMap::new();
            $(
                let mut elems = match $modval {
                    Bltn::Const(_) => panic!("Package must be map"),
                    Bltn::Map(elems) => elems,
                };

                if false $(|| true $($is_global)?)? {
                    for (_, (is_global, _)) in elems.iter_mut() {
                        *is_global = true;
                    }
                }

                if $pkg.insert(
                    stringify!($mod).to_owned(), (false, Bltn::Map(elems))
                ).is_some() { panic!(
                    concat!("Package ", stringify!($mod), " redeclared")
                )}
            )*

            $(create_bltns!(@func $(#$meta)*,
                "", false, ($pkg, $name, $func $args -> $ret $block)
            );)*
            Bltn::Map($pkg)
        }
    };

    ($pkg:ident : $($rest:tt)*) =>
        { create_bltns!{$pkg(stringify!($pkg)), make_bltns: $($rest)*} };
    ($pkg:ident($name:expr) : $($rest:tt)*) =>
        { create_bltns!{$pkg($name), make_bltns: $($rest)*} };
    ($pkg:ident($name:expr), $make_bltns:ident : mod $mods:tt $($rest:tt)*) =>
        { create_bltns!{$pkg($name), $make_bltns, mod $mods: $($rest)*} };
    ($pkg:ident($name:expr), $make_bltns:ident : $($rest:tt)*) =>
        { create_bltns!{$pkg($name), $make_bltns, mod {}: $($rest)*}};

    (mod $mods:tt $(
        $(#$meta:tt)* $vis:vis fn $func:ident $args:tt -> $ret:ty $block:block
    )*) => { create_bltns!{pkg ("pkg"), make_bltns, mod $mods:
        $($(#$meta)* $vis fn $func $args -> $ret $block)*
    }};

    ($(
        $(#$meta:tt)* $vis:vis fn $func:ident $args:tt -> $ret:ty $block:block
    )*) => { create_bltns!{pkg ("pkg"), make_bltns, mod {}:
        $($(#$meta)* $vis fn $func $args -> $ret $block)*
    }};
}
