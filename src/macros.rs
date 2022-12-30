
macro_rules! eval_err {
    ($($arg:tt)*) => { EvalError::new(format!($($arg)*)) };
}

macro_rules! count_tt {
    () => { 0 };
    ($fst:tt $($item:tt)*) => {1 + count_tt!($($item)*)};
}

macro_rules! call {
    (($obj:expr)($($arg:expr),*)) =>
        { $obj.call(None, vec![$($arg.into()),*]) };
    ($obj:ident ($($arg:expr),*)) =>
        { $obj.call(None, vec![$($arg.into()),*]) };

    (($obj:expr).$method:ident ($($arg:expr),*)) =>
        { $obj.call(Some(stringify!($method)), vec![$($arg.into()),*]) };
    ($obj:ident.$method:ident ($($arg:expr),*)) =>
        { $obj.call(Some(stringify!($method)), vec![$($arg.into()),*]) };
}

macro_rules! name_type {
    ($name:ident: $tp:ty) => { name_type!{stringify!($name), $tp} };
    ($name:literal: $tp:ty) => { name_type!{$name, $tp} };
    ($name:expr, $tp:ty) =>{
        impl NamedType for $tp { fn type_name() -> &'static str { $name }}
    };
}


macro_rules! impl_operable {
    (@una #[unary($name:ident)] $(#$meta:tt)*, (), $vars:tt) =>
        { impl_operable!{@una $(#$meta)*, ($name), $vars} };
    (@una #[$_:meta] $(#$meta:tt)*, $una:tt, $vars:tt) =>
        { impl_operable!{@una $(#$meta)*, $una, $vars} };
    (@una , (), $vars:tt) => {};
    (@una , ($name:ident), ($self:ident, $op:ident,
        ($id:ident : $tp:ty) -> $ret:ty $block:block
    )) => { if let Unary::$name = $op {
        let ret: $ret = (|$id: $tp| $block)($self.into());
        return Some(ret.into());
    }};



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
            match $self.try_into() {
                Ok(self_) => match $other.try_cast() {
                    Ok(other) => {
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


    (@arity $attr_pat:pat, ($attr:ident,
        ($_:pat $(, $arg:ident : $tp:ty)*)
    )) => { if let $attr_pat = $attr {
        return Some(count_tt!($($arg)*));
    }};

    (@get_self_ref $self:ident, (self $($_:tt)*)) => { $self.clone() };
    (@get_self_ref $self:ident, (&self $($_:tt)*)) => { $self };
    (@call $attr_pat:pat, ($self:ident, $attr:ident,
        $argvec:ident, $func_args:tt,
        $func:ident ($_:pat $(, $arg:ident : $tp:ty)*) -> $ret:ty $block:block
    )) => { if let $attr_pat = $attr {
        $(let $arg = match $argvec.remove(0).cast() {
            Ok(value) => value,
            Err(err) => return err,
        };)*
        return impl_operable!(@get_self_ref $self, $func_args)
            .$func($($arg),*).into()
    }};


    (@help #[doc=$doc:literal] $(#$meta:tt)*,
        $is_oper:tt, ($help:expr), $vars:tt
    ) => { impl_operable!{@help $(#$meta)*,
        $is_oper, (concat!($help, "\n", $doc)), $vars
    }};
    (@help #[call] $(#$meta:tt)*, (), $help:tt, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #[unary $_:tt] $(#$meta:tt)*, (), $help:tt, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #[binary $_:tt] $(#$meta:tt)*, (), $help:tt, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, (oper), $help, $vars} };
    (@help #$_:tt $(#$meta:tt)*, $is_oper:tt, $help:tt, $vars:tt) =>
        { impl_operable!{@help $(#$meta)*, $is_oper, $help, $vars} };
    (@help , (), ($help:expr), (
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
    (@help , (oper), ($help:expr), (
        $opers:ident, $methods:ident, $attr:ident, $func:tt
    )) => { if $attr == None { if $help.trim().len() > 0 {
        $opers += "\n";  $opers += $help;
    }}};



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

                $(impl_operable!{@help $(#$meta)*, (), (""),
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

