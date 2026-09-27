pub trait BuildTuple {
    type Bigger<N>;
    fn into_bigger<N>(self, n: N) -> Self::Bigger<N>;
}

pub trait AsTuple {
    const IS_STRUCT: bool = true;
    const NAMES: &'static [&'static str];
    type Tuple;
    fn into_tuple(self) -> Self::Tuple;
    fn from_tuple(tuple: Self::Tuple) -> Self;
}

pub trait TupleIsNotEmpty {}

pub trait Tuple<TupleSpec> {
    type Output;
    fn own_tuple_mut_spec(self, tuple_spec: TupleSpec) -> Self::Output;
}

/// Like [`Tuple`], but over `&self`.
pub trait RefTuple<TupleSpec> {
    type Output;
    fn ref_tuple_mut_spec(&self, tuple_spec: TupleSpec) -> Self::Output;
}

pub trait TupleSpec<Member> {
    type Output;
    fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
        &mut self,
        member: Member,
    ) -> Self::Output;
}

/// Like [`TupleSpec`], but always borrows each member (`&Member`), so `Member`
/// may be `?Sized` (e.g. `str`).
pub trait TupleSpecRef<Member: ?Sized> {
    type Output;
    fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
        &mut self,
        member: &Member,
    ) -> Self::Output;
}

pub trait TupleLast<TupleLastSpec> {
    type Output;
    fn own_tuple_own_spec(self, tuple_spec: TupleLastSpec) -> Self::Output;
}

pub trait TupleLastSpec<Member, Last> {
    type Output;
    type LastOutput;
    fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
        &mut self,
        member: Member,
    ) -> Self::Output;
    fn on_last<const INDEX: usize>(self, member: Last) -> Self::LastOutput;
}

pub trait TupleAsRef<'a> {
    type Output;
    fn tuple_as_ref(&'a self) -> Self::Output;
}

pub trait TupleAsMut<'a> {
    type Output: 'a;
    fn tuple_as_mut(&'a mut self) -> Self::Output;
}

pub struct Info<I, O> {
    pub input_info: I,
    pub output_info: O,
}

pub trait FnInfo<Input, Output> {
    type OutputInfo;
    type InputInfo;
    fn info(self) -> Info<Self::InputInfo, Self::OutputInfo>;
}

#[allow(non_camel_case_types)]
pub struct as_last_spec<S>(pub S);

impl<M, L, S> TupleLastSpec<M, L> for as_last_spec<S>
where
    S: TupleSpec<M>,
    S: TupleSpec<L>,
{
    type LastOutput = <S as TupleSpec<L>>::Output;
    type Output = <S as TupleSpec<M>>::Output;
    fn on_last<const INDEX: usize>(mut self, member: L) -> Self::LastOutput {
        TupleSpec::on_each::<INDEX, INDEX>(&mut self.0, member)
    }
    fn on_each<const LEN: usize, const INDEX: usize>(&mut self, member: M) -> Self::Output {
        TupleSpec::on_each::<LEN, INDEX>(&mut self.0, member)
    }
}

macro_rules! implt {
    ($($num:literal)* size $size:literal) => {
        paste::paste!(
            impl<$([<T $num>],)*> AsTuple for ($([<T $num>],)*)
            {
                type Tuple = Self;
                const IS_STRUCT: bool = false;
                const NAMES: &'static [&'static str] = &[$(stringify!($num),)*];
                fn into_tuple(self) -> Self::Tuple {
                    self
                }
                fn from_tuple(tuple: Self::Tuple) -> Self {
                    tuple
                }
            }
            impl<$([<T $num>],)*> BuildTuple for ($([<T $num>],)*)
            {
                type Bigger<N> = ($([<T $num>],)* N,);
                fn into_bigger<N>(self, n: N) -> Self::Bigger<N> {
                    ($(paste::paste!(self.$num),)* n,)
                }
            }

            impl<'a, $([<T $num>]:'a,)*> TupleAsRef<'a> for ($([<T $num>],)*)
            {
                type Output = (
                    $(&'a [<T $num>],)*
                );
                fn tuple_as_ref(&'a self) -> Self::Output {
                    (
                        $(&self.$num,)*
                    )
                }
            }

            impl<'a, $([<T $num>]:'a,)*> TupleAsMut<'a> for ($([<T $num>],)*)
            {
                type Output = (
                    $(&'a mut [<T $num>],)*
                );
                fn tuple_as_mut(&'a mut self) -> Self::Output {
                    (
                        $(&mut self.$num,)*
                    )
                }
            }

            impl<$([<T $num>],)* TS> Tuple<TS> for ($([<T $num>],)*)
            where
                $(TS: TupleSpec<[<T $num>]>,)*
            {
                type Output = (
                    $(<TS as TupleSpec<[<T $num>]> >::Output,)*
                );
                fn own_tuple_mut_spec(self, #[allow(unused)] mut tuple_spec: TS) -> Self::Output {
                    (
                        $(<TS as TupleSpec<[<T $num>]> >::on_each::<$size,$num>(&mut tuple_spec, self.$num),)*
                    )
                }
            }

            // Owned members must be Sized. (`?Sized` is for pointees of `(&T,…)`,
            // which would overlap this impl — see concept_6.)
            impl<$([<T $num>],)* TS> RefTuple<TS> for ($(&[<T $num>],)*)
            where
                $([<T $num>]: ?Sized,)*
                $(TS: TupleSpecRef<[<T $num>]>,)*
            {
                type Output = (
                    $(<TS as TupleSpecRef<[<T $num>]>>::Output,)*
                );
                fn ref_tuple_mut_spec(
                    &self,
                    #[allow(unused)] mut tuple_spec: TS,
                ) -> Self::Output {
                    (
                        $(<TS as TupleSpecRef<[<T $num>]>>::on_each::<$size, $num>(
                            &mut tuple_spec,
                            &self.$num,
                        ),)*
                    )
                }
            }

            impl<F, O, $([<I $num>],)*> FnInfo<($([<I $num>],)*), O> for F
            where
                F: FnOnce($([<I $num>],)*) -> O,
            {
                type InputInfo = (
                    $(std::marker::PhantomData<[<I $num>]>,)*
                );
                type OutputInfo = std::marker::PhantomData<O>;
                fn info(self) -> Info<Self::InputInfo, Self::OutputInfo> {
                    Info {
                        input_info: (
                            $(std::marker::PhantomData::<[<I $num>]>,)*
                        ),
                        output_info: std::marker::PhantomData,
                    }
                }
            }

        );

    };
    ($($num:literal)* last $last:literal) => {
        implt!($($num)* $last size $last);

        paste::paste!(
            impl<$([<T $num>],)* [<T $last>],> TupleIsNotEmpty for ($([<T $num>],)* [<T $last>],) {}
            impl<$([<T $num>],)* [<T $last>], TS> TupleLast<TS> for ($([<T $num>],)* [<T $last>],)
            where
                $(TS: TupleLastSpec<[<T $num>], [<T $last>]>,)*
                TS: TupleLastSpec<[<T $last>], [<T $last>]>,
            {
                type Output = (
                    $(<TS as TupleLastSpec<[<T $num>], [<T $last>]>>::Output,)*
                    <TS as TupleLastSpec<[<T $last>], [<T $last>]>>::LastOutput,
                );
                fn own_tuple_own_spec(self, #[allow(unused)] mut tuple_spec: TS) -> Self::Output {
                    (
                        $(<TS as TupleLastSpec<[<T $num>], [<T $last>]> >::on_each::<$last,$num>(&mut tuple_spec, self.$num),)*
                        <TS as TupleLastSpec<[<T $last>], [<T $last>]> >::on_last::<$last>(tuple_spec, self.$last),
                    )
                }
            }


        );
    };
}

impl<TS> TupleLast<TS> for () {
    type Output = ();
    fn own_tuple_own_spec(self, _: TS) -> Self::Output {}
}

implt!(size 0);
implt!(last 0);
implt!(0 last 1);
implt!(0 1 last 2);
implt!(0 1 2 last 3);
implt!(0 1 2 3 last 4);
implt!(0 1 2 3 4 last 5);
implt!(0 1 2 3 4 5 last 6);
implt!(0 1 2 3 4 5 6 last 7);
implt!(0 1 2 3 4 5 6 7 last 8);
implt!(0 1 2 3 4 5 6 7 8 last 9);
implt!(0 1 2 3 4 5 6 7 8 9 last 10);
implt!(0 1 2 3 4 5 6 7 8 9 10 last 11);
implt!(0 1 2 3 4 5 6 7 8 9 10 11 last 12);
implt!(0 1 2 3 4 5 6 7 8 9 10 11 12 last 13);
implt!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 last 14);
implt!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 last 15);

#[cfg(test)]
mod calling_fn_test {
    use std::marker::PhantomData;

    use crate::tuple_trait::{Info, Tuple, TupleSpec};

    fn example(f0: i32, f1: bool) -> String {
        format!("{} {}", f0, f1)
    }

    pub struct DefaultFromPhantom;

    impl<T: Default> TupleSpec<PhantomData<T>> for DefaultFromPhantom {
        type Output = T;
        fn on_each<const LEN: usize, const INDEX: usize>(
            &mut self,
            _: PhantomData<T>,
        ) -> Self::Output {
            T::default()
        }
    }

    trait FnOnceStable<Args> {
        type Output;
        fn call_once(self, args: Args) -> Self::Output;
    }
    impl<F, T0, T1, O> FnOnceStable<(T0, T1)> for F
    where
        F: FnOnce(T0, T1) -> O,
    {
        type Output = F::Output;
        fn call_once(self, args: (T0, T1)) -> Self::Output {
            self(args.0, args.1)
        }
    }

    #[test]
    fn example_test() {
        let Info {
            input_info,
            output_info: _,
        } = super::FnInfo::info(example);
        let populated = input_info.own_tuple_mut_spec(DefaultFromPhantom);
        let output = FnOnceStable::call_once(example, populated);
        assert_eq!(output, "0 false");
    }
}

#[cfg(test)]
mod basic_example {
    #![allow(unused)]
    #![deny(unused_must_use)]
    #![allow(non_camel_case_types)]

    use crate::tuple_trait::{BuildTuple, RefTuple, Tuple, TupleAsRef, TupleSpec, TupleSpecRef};
    use core::fmt;
    use std::marker::PhantomData;

    struct join<'q>(&'q mut String);

    impl<M> TupleSpec<&M> for join<'_>
    where
        M: fmt::Display + ?Sized,
    {
        type Output = ();
        fn on_each<const LEN: usize, const INDEX: usize>(&mut self, member: &M) -> Self::Output {
            self.0.push_str(format!("{}", member).as_str());
            if LEN != INDEX {
                self.0.push_str(", ");
            }
        }
    }

    impl<M> TupleSpecRef<M> for join<'_>
    where
        M: fmt::Display + ?Sized,
    {
        type Output = ();
        fn on_each<const LEN: usize, const INDEX: usize>(&mut self, member: &M) -> Self::Output {
            self.0.push_str(format!("{}", member).as_str());
            if LEN != INDEX {
                self.0.push_str(", ");
            }
        }
    }

    #[test]
    fn use_spec() {
        let mut str = String::new();

        let tuple = (3, "hello world", true);

        TupleAsRef::tuple_as_ref(&tuple).own_tuple_mut_spec(join(&mut str));

        // join didnt hold any lifetimes as I can move tuple
        let _ = tuple;

        assert_eq!(str.as_str(), "3, hello world, true")
    }

    #[test]
    fn use_ref_tuple() {
        use crate::tuple_trait::RefTuple;

        let mut str = String::new();
        let key = String::from("key");
        // concept_6 pattern: tuple of refs, no HRTB / 'static
        (key.as_str(), "todo", "tag").ref_tuple_mut_spec(join(&mut str));
        assert_eq!(str.as_str(), "key, todo, tag")
    }

    struct phantomize;
    impl<M> TupleSpec<&M> for phantomize {
        type Output = PhantomData<M>;
        fn on_each<const LEN: usize, const INDEX: usize>(&mut self, _: &M) -> Self::Output {
            PhantomData
        }
    }

    #[test]
    fn phantom_exampe() {
        let tuple = (3, "hello world", true);

        let phantoms: (
            // phantoms types
            PhantomData<usize>,
            PhantomData<&str>,
            PhantomData<bool>,
        ) = tuple.tuple_as_ref().own_tuple_mut_spec(phantomize);
    }

    struct Ctx<'b>(&'b mut String);
    struct double_lifetime<'shallow, 'deep>(&'shallow mut Ctx<'deep>);

    impl<'a, 'b> TupleSpec<&'b str> for double_lifetime<'a, 'b> {
        type Output = ();

        fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
            &mut self,
            member: &'b str,
        ) -> Self::Output {
            self.0.0.push_str(member);
        }
    }

    fn use_double_lifetime<'deep, T>(
        //into: &'shllow mut Next<'deep> ,
        ctx: &mut Ctx<'deep>,
        this: &'deep str,
    ) where
        T: BuildTuple,
        // eradicate one lifetime and abstract over the other.
        //
        // because not always that I can specify the lifetime
        // of shallow (inside traits you cannot change signature of method)
        // impl<'deep> Trait<'deep> for &'deep str {
        //     fn method(self, ctx: &mut Ctx<'deep>)
        // }
        (&'static str,): for<'shllow> Tuple<double_lifetime<'shllow, 'deep>>,
    {
        let ctx = double_lifetime(ctx);
        ("hello",).into_bigger(this).own_tuple_mut_spec(ctx);
    }
}
