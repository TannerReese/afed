
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
    (@una #[unary($name:ident)] $(#$m:tt)*, (), $vars:tt) =>
        { impl_operable!{@una $(#$m)*, ($name), $vars} };
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
    (@method #[unary $_:tt] $(#[$m:meta])*, $c:tt, $vs:tt) => {};
    (@method #[binary $_:tt] $(#[$m:meta])*, $c:tt, $vs:tt) => {};
    (@method #[exclude $_:tt] $(#[$m:meta])*, $c:tt, $vs:tt) => {};
    (@method #[$_:meta] $(#$meta:tt)*, $is_call:tt, $vars:tt) =>
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


    (@strip #[call]) => {};
    (@strip #[unary $_:tt]) => {};
    (@strip #[binary $_:tt]) => {};
    (@strip #[exclude $_:tt]) => {};
    (@strip #[$meta:meta]) => { #[$meta] };
    (@method_impl $_:pat, (
        $(#$meta:tt)*
        $vis:vis $func:ident $args:tt -> $ret:ty $block:block
    )) => {
        $(impl_operable!{@strip #$meta})*
        $vis fn $func $args -> $ret $block
    };


    ($Self:ty : $(
        $(#$meta:tt)*
        $vis:vis fn $func:tt $args:tt -> $ret:ty $block:block
    )*) => {
        impl $Self { $(impl_operable!{@method $(#$meta)*, (), (
            @method_impl $func, (
                $(#$meta)*
                $vis $func $args -> $ret $block
            )
        )})* }

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
}

