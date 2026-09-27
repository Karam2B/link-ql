//! Types to compose expressions
//!
//! ## Many expressions -- Join
//!
//! Join<ManyExpressions> implements Expression
//! where ManyExpressions selects for
//! - T where T: Expression
//! - tuples with each member implement expression
//! - tuples with each member is a Nest<ManyExpressionsDeep>, look at the nesting section for help
//! - Vec<T> where T: Expression
//! - Option<T> where T: Expression
//! - () -- empty expression
//!
//! ## Zero or one expression -- Prefixed
//!
//! Prefixed<OptionalExpression> implements Expression
//! where OptionalExpression selects for
//! - T where T: Expression
//! - Option<T> where T: Expression
//! - () -- empty expression
//!
//! ## Using for trait bounds
//!
//! these types can be used in trait bounds to select for ManyExpressions
//! and OptionalExpressions, like this
//! ```ignore
//! impl<'a, S, SelectItems> Expression<'a, S> for BasicSelectStatement<SelectItems>
//! where
//!     Join<SelectItems>: Expression<'a, S>
//! {
//!     fn expression(self, ctx: &mut StatementBuilder<'a, S>) {
//!         Join {
//!             start: "SELECT",
//!             separator: ", ",
//!             items: self.select_items,
//!         }
//!         .expression(ctx);
//!     }
//! }
//! ```
//!
//! Constructing these types is done internally by types that
//! implement Expression, usually constructing them outside of
//! Expression implementations is a mistake
//!
//! ## Nesting
//! If you already selected many ManyExpressions you can nest each
//! inside a Nest and put them together inside a tuple
//! pass that tuple where ManyExpressions was expected, like
//! ```ignore
//! impl<S, Tuple1, Tuple2> SqlString<S, Tuple1, Tuple2>
//! where
//!     Join<Tuple1>: for<'q> Expression<'q, S>,
//!     Join<Tuple2>: for<'q> Expression<'q, S>
//! {
//!     fn sql_string(self) -> String {
//!         StatementBuilder::<S>::new(BasicSelectStatement {
//!             select_items: (
//!                 Nest(tuple_1),
//!                 Nest(tuple_2),
//!             )
//!         }).unwrap().0
//!     }
//! }
//!
//! #[test]
//! fn using_example() {
//!     let sql_string = SqlString::<Sqlite, _, _>::new(
//!         (
//!             Nest(("column_1", "column_2")),
//!             Nest(("column_3", "column_4", "column_5")),
//!         )
//!     );
//!     assert_eq!(sql_string, "SELECT column_1, column_2, column_3, column_4, column_5");
//! }
//!
//! ```
use crate::database_extention::DatabaseExt;

use super::{Expression, OpExpression, StatementBuilder};

pub trait OptionalExpression {
    fn is_oper(&self) -> bool;
}

impl OptionalExpression for () {
    fn is_oper(&self) -> bool {
        false
    }
}

impl<T: OptionalExpression> OptionalExpression for Option<T> {
    fn is_oper(&self) -> bool {
        self.as_ref().is_some_and(|item| item.is_oper())
    }
}

impl<T: OptionalExpression> OptionalExpression for Vec<T> {
    fn is_oper(&self) -> bool {
        self.iter().any(|item| item.is_oper())
    }
}

impl<T: OptionalExpression, const N: usize> OptionalExpression for [T; N] {
    fn is_oper(&self) -> bool {
        self.iter().any(|item| item.is_oper())
    }
}

impl<T: OpExpression> OptionalExpression for T {
    fn is_oper(&self) -> bool {
        self.is_expression_present()
    }
}

#[derive(Clone)]
pub struct Join<Items> {
    pub start: &'static str,
    pub separator: &'static str,
    pub items: Items,
}

#[derive(Clone)]
pub struct Prefixed<T> {
    pub prefix: &'static str,
    pub inner: T,
}

#[derive(Clone)]
pub struct Nest<T>(pub T);

impl<T: OptionalExpression> OptionalExpression for Nest<T> {
    fn is_oper(&self) -> bool {
        self.0.is_oper()
    }
}

impl<Items> OpExpression for Join<Items> {}

impl<OptionalExpression> OpExpression for Prefixed<OptionalExpression> {}

impl<'q, S, T> Expression<'q, S> for Prefixed<T>
where
    S: DatabaseExt,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax(self.prefix);
        self.inner.expression(ctx);
    }
}

impl<'q, S, T> Expression<'q, S> for Join<T>
where
    S: DatabaseExt,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax(self.start);
        self.items.expression(ctx);
    }
}

impl<'q, S> Expression<'q, S> for Prefixed<()>
where
    S: DatabaseExt,
{
    fn expression(self, _ctx: &mut StatementBuilder<'q, S>) {}
}

impl<'q, S: DatabaseExt> Expression<'q, S> for Join<()> {
    fn expression(self, _ctx: &mut StatementBuilder<'q, S>) {}
}

impl<'q, S, T> Expression<'q, S> for Prefixed<Option<T>>
where
    S: DatabaseExt,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        if let Some(inner) = self.inner {
            ctx.syntax(self.prefix);
            inner.expression(ctx);
        }
    }
}

impl<'q, S, T> Expression<'q, S> for Join<Option<T>>
where
    S: DatabaseExt,
    T: OptionalExpression,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        let Some(item) = self.items else {
            return;
        };
        ctx.syntax(self.start);
        item.expression(ctx);
    }
}

impl<'q, S, T> Expression<'q, S> for Join<Vec<T>>
where
    S: DatabaseExt,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        let mut started = false;
        for item in self.items {
            if !item.is_oper() {
                continue;
            }
            if started {
                ctx.syntax(self.separator);
            } else {
                ctx.syntax(self.start);
                started = true;
            }
            item.expression(ctx);
        }
    }
}

impl<'q, S, T, const N: usize> Expression<'q, S> for Join<[T; N]>
where
    S: DatabaseExt,
    T: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        let mut started = false;
        for item in self.items {
            if !item.is_oper() {
                continue;
            }
            if started {
                ctx.syntax(self.separator);
            } else {
                ctx.syntax(self.start);
                started = true;
            }
            item.expression(ctx);
        }
    }
}

macro_rules! impl_tuple_join {
    ($($idx:tt: $T:ident),+) => {
        impl<$($T),+> OptionalExpression for ($($T,)+)
        where
            $($T: OptionalExpression,)+
        {
            fn is_oper(&self) -> bool {
                false $(|| self.$idx.is_oper())+
            }
        }


        impl<'q, S, $($T),+> Expression<'q, S>
            for Join< ($($T,)+)>
        where
            S: DatabaseExt,
            $(
                $T: Expression<'q, S>,
                $T: OptionalExpression,
            )+
        {
            #[allow(non_snake_case, unused_assignments)]
            fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
                let mut started = false;
                let ($($T,)+) = self.items;
                $(
                    if $T.is_oper() {
                        if started {
                            ctx.syntax(self.separator);
                        } else {
                            ctx.syntax(self.start);
                            started = true;
                        }
                        $T.expression(ctx);
                    }
                )+
            }
        }

        impl<'q, S, $($T),+> Expression<'q, S>
            for Join<($(Nest<$T>,)+)>
        where
            S: DatabaseExt,
            $(
                $T: OptionalExpression,
                Join<$T>: Expression<'q, S>,
            )+
        {
            fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
                let mut started = false;
                $(
                    if self.items.$idx.0.is_oper() {
                        if started {
                            Join {
                                start: self.separator,
                                separator: self.separator,
                                items: self.items.$idx.0,
                            }
                            .expression(ctx);
                        } else {
                            Join {
                                start: self.start,
                                separator: self.separator,
                                items: self.items.$idx.0,
                            }
                            .expression(ctx);
                        }
                        #[allow(unused_assignments)]
                        {
                            started = true;
                        }
                    }
                )+
            }
        }
    };
}

impl_tuple_join!(0: T0);
impl_tuple_join!(0: T0, 1: T1);
impl_tuple_join!(0: T0, 1: T1, 2: T2);
impl_tuple_join!(0: T0, 1: T1, 2: T2, 3: T3);
impl_tuple_join!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_tuple_join!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
impl_tuple_join!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
impl_tuple_join!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);

#[cfg(test)]
mod nesting {
    use sqlx::Sqlite;

    use crate::sqlx_query_builder::{
        Bind, StatementBuilder, basic_expressions::ColumnEqual, combinators::Nest,
    };

    use super::Join;

    #[test]
    fn test_nesting() {
        let join = Join {
            start: "SELECT ",
            separator: ", ",
            items: (
                "column",
                ColumnEqual {
                    col: "column",
                    eq: Bind(34),
                },
                ColumnEqual {
                    col: "column",
                    eq: Bind(34),
                },
                ColumnEqual {
                    col: "column",
                    eq: Bind(34),
                },
            ),
        };

        let stmt = StatementBuilder::<Sqlite>::new(join).unwrap().0;
        assert_eq!(
            stmt,
            r#"SELECT "column", "column" = $1, "column" = $2, "column" = $3"#
        );

        let join = Join {
            start: "SELECT ",
            separator: ", ",
            items: ("column_1", "column_2"),
        };

        let stmt = StatementBuilder::<Sqlite>::new(join).unwrap().0;
        assert_eq!(stmt, r#"SELECT "column_1", "column_2""#);

        // if you have already selected more than one tuple
        // you need to constructing tuple of Nests
        // (Nest<Tuple1>, Nest<Tuple2>)
        // and pass it to where Tuple is expected

        let join = Join {
            start: "SELECT ",
            separator: ", ",
            items: (
                Nest(("column_1", "column_2")),
                Nest(("column_3", "column_4", "column_5")),
            ),
        };

        let stmt = StatementBuilder::<Sqlite>::new(join).unwrap().0;
        assert_eq!(
            stmt,
            r#"SELECT "column_1", "column_2", "column_3", "column_4", "column_5""#
        );

        // Expression for Join<(T0, T1)>
        // the way we select only for the tuple:
        // where Join<Tuple>: Expression<'q, S>
        // if we have Tuple1, Tuple2 what implement Expression?
        // Expression for Join<(Nest<(T0, T1)>, Nest<(T2, )>)>
    }
}
