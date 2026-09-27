use std::marker::PhantomData;

use sqlx::Database;

use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{
        Expression, OpExpression, RefExpression, StatementBuilder,
        combinators::{Join, OptionalExpression},
    },
    update_mod::Update,
};

pub struct Bind<T>(pub T);

impl<T: Clone> Clone for Bind<T> {
    fn clone(&self) -> Self {
        Bind(self.0.clone())
    }
}

impl<T> OpExpression for Bind<T> {}
impl<'q, S, T> Expression<'q, S> for Bind<T>
where
    S: sqlx::Database,
    T: 'q + sqlx::Type<S> + sqlx::Encode<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.bind(self.0);
    }
}

pub struct TypeAsSyntax<T>(pub PhantomData<T>);

impl<T> Clone for TypeAsSyntax<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T> OpExpression for TypeAsSyntax<T> {}
impl<'q, S, T> Expression<'q, S> for TypeAsSyntax<T>
where
    S: DatabaseExt,
    T: sqlx::Type<S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: Database,
    {
        ctx.type_as_syntax::<T>();
    }
}

#[derive(Clone)]
pub struct AliasedScopedColumn<T, C, A> {
    pub table: T,
    pub column: C,
    pub alias: A,
}

impl<T, C, A> OpExpression for AliasedScopedColumn<T, C, A> {}
impl<'q, T, C, A, S> Expression<'q, S> for AliasedScopedColumn<T, C, A>
where
    S: DatabaseExt,
    C: Expression<'q, S>,
    T: Expression<'q, S>,
    A: Expression<'q, S>,
{
    fn expression(self, arg: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.table.expression(arg);
        arg.syntax(".");
        self.column.expression(arg);
        arg.syntax(" AS ");
        self.alias.expression(arg);
    }
}

#[derive(Clone)]
pub struct ScopedColumn<T, C> {
    pub table: T,
    pub col: C,
}

impl<T, C> OpExpression for ScopedColumn<T, C> {}
impl<'q, T, C, S> Expression<'q, S> for ScopedColumn<T, C>
where
    T: Expression<'q, S>,
    C: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.table.expression(ctx);
        ctx.syntax(".");
        self.col.expression(ctx);
    }
}

#[derive(Clone)]
pub struct UpdatingColumn<C, T> {
    pub col: C,
    pub set: T,
}

impl<C, T> OpExpression for UpdatingColumn<C, Bind<T>> {}

impl<'a, S, C, T> Expression<'a, S> for UpdatingColumn<C, Bind<T>>
where
    S: DatabaseExt,
    T: sqlx::Type<S> + sqlx::Encode<'a, S> + 'a,
    C: Expression<'a, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'a, S>) {
        self.col.expression(ctx);
        ctx.syntax(&" = ");
        ctx.bind(self.set.0);
    }
}

impl<C, T> OpExpression for UpdatingColumn<C, Option<T>> {}
impl<'a, C, T, S> Expression<'a, S> for UpdatingColumn<C, Option<T>>
where
    S: DatabaseExt,
    T: Expression<'a, S>,
    C: Expression<'a, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'a, S>) {
        self.col.expression(ctx);
        ctx.syntax(&" = ");
        match self.set {
            Some(value) => {
                value.expression(ctx);
            }
            None => {
                ctx.syntax("NULL");
            }
        }
    }
}

impl<C, T> OpExpression for UpdatingColumn<C, Update<T>> {}

impl<'a, C, T, S> Expression<'a, S> for UpdatingColumn<C, Update<Bind<T>>>
where
    S: DatabaseExt,
    T: sqlx::Type<S> + sqlx::Encode<'a, S> + 'a,
    C: for<'b> RefExpression<'b, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'a, S>) {
        match self.set {
            Update::Set(value) => {
                self.col.ref_expression(ctx);
                ctx.syntax(&" = ");
                ctx.bind(value.0);
            }
            Update::Keep => {}
        }
    }
}

macro_rules! column_compare {
    ($name:ident, $field_name:ident, $op:literal) => {
        #[derive(Clone)]
        pub struct $name<Col, Val> {
            pub col: Col,
            pub $field_name: Val,
        }

        impl<Col, Val> OpExpression for $name<Col, Val> {}

        impl<'q, S, Col, Val> Expression<'q, S> for $name<Col, Val>
        where
            S: DatabaseExt,
            Col: Expression<'q, S>,
            Val: Expression<'q, S>,
        {
            fn expression(self, arg: &mut StatementBuilder<'q, S>) {
                self.col.expression(arg);
                arg.syntax($op);
                self.$field_name.expression(arg);
            }
        }
    };
}

macro_rules! column_is {
    ($name:ident, $op:literal) => {
        #[derive(Clone)]
        pub struct $name<Col> {
            pub col: Col,
        }

        impl<Col> OpExpression for $name<Col> {}

        impl<'q, S, Col> Expression<'q, S> for $name<Col>
        where
            S: DatabaseExt,
            Col: Expression<'q, S>,
        {
            fn expression(self, arg: &mut StatementBuilder<'q, S>) {
                self.col.expression(arg);
                arg.syntax($op);
            }
        }
    };
}

macro_rules! group_with {
    ($name:ident, $op:literal) => {
        #[derive(Clone)]
        pub struct $name<T>(pub T);

        impl<T> OpExpression for $name<T> {}

        impl<'q, S, T> Expression<'q, S> for $name<T>
        where
            S: DatabaseExt,
            T: OptionalExpression,
            Join<T>: Expression<'q, S>,
        {
            fn expression(self, arg: &mut StatementBuilder<'q, S>) {
                if self.0.is_oper() {
                    Join {
                        start: "(",
                        separator: $op,
                        items: self.0,
                    }
                    .expression(arg);
                    arg.syntax(")");
                }
            }
        }
    };
}

column_compare!(ColumnEqual, eq, " = ");
column_compare!(ColumnNotEqual, ne, " != ");
column_compare!(ColumnGreaterThan, gt, " > ");
column_compare!(ColumnGreaterThanOrEqual, ge, " >= ");
column_compare!(ColumnLessThan, lt, " < ");
column_compare!(ColumnLessThanOrEqual, le, " <= ");
column_compare!(ColumnContains, like, " LIKE ");

column_is!(ColumnIsNotNull, " IS NOT NULL");
column_is!(ColumnIsNull, " IS NULL");

group_with!(ExpressionsWithAnd, " AND ");
group_with!(ExpressionsWithOr, " OR ");

#[derive(Clone)]
pub struct ColumnIn<Col, V> {
    pub col: Col,
    pub values: V,
}

impl<Col, V> OpExpression for ColumnIn<Col, V> {}

impl<'q, S, Col, V> Expression<'q, S> for ColumnIn<Col, V>
where
    S: DatabaseExt,
    Col: Expression<'q, S>,
    Join<V>: Expression<'q, S>,
    V: OptionalExpression,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        if self.values.is_oper() {
            self.col.expression(ctx);
            Join {
                start: " IN (",
                separator: ", ",
                items: self.values,
            }
            .expression(ctx);
            ctx.syntax(")");
        }
    }
}

#[derive(Clone)]
pub struct ManyColumnsLargerOrEqual<Ids, Values> {
    pub ids: Ids,
    pub values: Values,
}

impl<Ids, Values> OpExpression for ManyColumnsLargerOrEqual<Ids, Values> {}

impl<'q, S, Ids, Values> Expression<'q, S> for ManyColumnsLargerOrEqual<Ids, Values>
where
    S: DatabaseExt,
    Join<Ids>: Expression<'q, S>,
    Join<Values>: Expression<'q, S>,
{
    fn expression(self, arg: &mut StatementBuilder<'q, S>) {
        arg.syntax("(");
        Join {
            start: "",
            separator: ",",
            items: self.ids,
        }
        .expression(arg);
        arg.syntax(")");
        arg.syntax(" >= ");
        arg.syntax("(");
        Join {
            start: "",
            separator: ",",
            items: self.values,
        }
        .expression(arg);
        arg.syntax(")");
    }
}

pub struct ForeignKey<Table, Column, Ons> {
    pub references_table: Table,
    pub references_col: Column,
    pub ons: Ons,
}

impl<Table, Column, Ons> OpExpression for ForeignKey<Table, Column, Ons> {}

mod imp_foriegn_key_for_sqlite {
    use sqlx::Sqlite;

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, Join, StatementBuilder},
    };

    impl<'q, Table, Column, Ons> Expression<'q, Sqlite> for super::ForeignKey<Table, Column, Ons>
    where
        Join<Ons>: Expression<'q, Sqlite>,
        Table: Expression<'q, Sqlite>,
        Column: Expression<'q, Sqlite>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax("REFERENCES ");
            self.references_table.expression(ctx);
            ctx.syntax("(");
            self.references_col.expression(ctx);
            ctx.syntax(")");
            Join {
                start: " ",
                separator: ", ",
                items: self.ons,
            }
            .expression(ctx);
        }
    }
}

pub struct JoinExpression<ForeignTable, ForeignColumn, LocalTable, LocalColumn> {
    pub join_type: &'static str,
    pub foreign_table: ForeignTable,
    pub foreign_column: ForeignColumn,
    pub local_table: LocalTable,
    pub local_column: LocalColumn,
}

impl<ForeignTable, ForeignColumn, LocalTable, LocalColumn> OpExpression
    for JoinExpression<ForeignTable, ForeignColumn, LocalTable, LocalColumn>
{
}

impl<'q, S, Ft, Fc, Lt, Lc> Expression<'q, S> for JoinExpression<Ft, Fc, Lt, Lc>
where
    S: DatabaseExt,
    Ft: for<'a> RefExpression<'a, S>,
    Fc: Expression<'q, S>,
    Lt: Expression<'q, S>,
    Lc: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax(&self.join_type);
        ctx.syntax(" ");
        self.foreign_table.ref_expression(ctx);
        ctx.syntax(" ON ");
        self.local_table.expression(ctx);
        ctx.syntax(".");
        self.local_column.expression(ctx);
        ctx.syntax(" = ");
        self.foreign_table.ref_expression(ctx);
        ctx.syntax(".");
        self.foreign_column.expression(ctx);
    }
}

pub struct OnDeleteSetNull;

impl OpExpression for OnDeleteSetNull {}

mod imp_on_delete_set_null_for_sqlite {
    use sqlx::Sqlite;

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, StatementBuilder},
    };

    impl<'q> Expression<'q, Sqlite> for super::OnDeleteSetNull {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax(&"ON DELETE SET NULL");
        }
    }
}

pub struct OnDeleteCascase;

impl OpExpression for OnDeleteCascase {}

mod imp_on_delete_cascase_for_sqlite {
    use sqlx::Sqlite;

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, StatementBuilder},
    };

    impl<'q> Expression<'q, Sqlite> for super::OnDeleteCascase {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax(&"ON DELETE CASCADE");
        }
    }
}

pub struct CompositePrimaryKey<Cols>(pub Cols);

impl<Cols> OpExpression for CompositePrimaryKey<Cols> {}

impl<'q, S, Cols> Expression<'q, S> for CompositePrimaryKey<Cols>
where
    S: DatabaseExt,
    Cols: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("PRIMARY KEY (");
        self.0.expression(ctx);
        ctx.syntax(")");
    }
}

#[cfg(test)]
#[linked_sql_macros::skip]
/// skipped waiting RefExpression refactoring
mod tests {
    use crate::sqlx_query_builder::{Expression, Join, OpExpression, StatementBuilder};
    use crate::sqlx_query_builder::{Expression, Join, OpExpression, StatementBuilder};
    use sqlx::Sqlite;

    struct ManyToImplExpression<T>(pub T);
    impl<T> OpExpression for ManyToImplExpression<T> {}
    impl<'q, T> Expression<'q, Sqlite> for ManyToImplExpression<T>
    where
        Join<T>: Expression<'q, Sqlite>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>) {
            Join {
                start: "START ",
                separator: ", ",
                items: self.0,
            }
            .expression(ctx);
        }
    }
    #[test]
    fn test_many_flat() {
        let stmt = StatementBuilder::<Sqlite>::new(Join {
            start: "START ",
            separator: ", ",
            items: (
                vec!["id", "email"],
                vec!["name", "age"],
                vec!["job", "job_description"],
            ),
        });

        pretty_assertions::assert_eq!(
            stmt.stmt().replace("\"", "'"),
            "START 'id', 'email', 'name', 'age', 'job', 'job_description'"
        );
        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: (
                vec!["id", "email"],
                vec!["name", "age"],
                vec!["job", "job_description"],
            ),
        }));
        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: (
                vec!["id", "email"],
                vec!["name", "age"],
                vec!["job", "job_description"],
            ),
        }));
        pretty_assertions::assert_eq!(
            stmt.stmt().replace("\"", "'"),
            "START 'id', 'email', 'name', 'age', 'job', 'job_description'"
        );
        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: (vec!["id", "email"], vec!["name", "age"]),
        }));
        pretty_assertions::assert_eq!(
            stmt.stmt().replace("\"", "'"),
            "START 'id', 'email', 'name', 'age'"
        );

        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: (vec!["id", "email"],),
        }));
        pretty_assertions::assert_eq!(stmt.stmt().replace("\"", "'"), "START 'id', 'email'");

        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: vec![
                vec!["id", "email"],
                vec!["name", "age"],
                vec!["job", "job_description"],
            ],
        }));
        let stmt = StatementBuilder::<'_, Sqlite>::new(ManyToImplExpression(Join {
            start: "",
            separator: ", ",
            items: vec![
                vec!["id", "email"],
                vec!["name", "age"],
                vec!["job", "job_description"],
            ],
        }));
        pretty_assertions::assert_eq!(
            stmt.stmt().replace("\"", "'"),
            "START 'id', 'email', 'name', 'age', 'job', 'job_description'"
        );
    }
}

pub struct DefaultExpression<T> {
    pub value: T,
}

impl<T> OpExpression for DefaultExpression<T> {}
impl<'q, S, T> Expression<'q, S> for DefaultExpression<T>
where
    T: Expression<'q, S>,
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax("DEFAULT ");
        self.value.expression(ctx);
    }
}

pub struct CurrentTimestamp;

impl OpExpression for CurrentTimestamp {}

mod current_timestamp_for_sqlite {
    use sqlx::Sqlite;

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, StatementBuilder},
    };

    impl<'q> Expression<'q, Sqlite> for super::CurrentTimestamp {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax("CURRENT_TIMESTAMP");
        }
    }
}

pub mod sub_query_expressions {
    use crate::sqlx_query_builder::OpExpression;

    pub struct CurrentTable;
    pub struct ColumnMatchNew<C>(pub C);
    pub struct ColumnNewEqualsOld<C>(pub C);

    impl OpExpression for CurrentTable {}
    impl<C> OpExpression for ColumnMatchNew<C> {}
    impl<C> OpExpression for ColumnNewEqualsOld<C> {}

    mod impl_for_sqlite {
        use sqlx::Sqlite;

        use crate::{
            database_extention::DatabaseExt,
            sqlx_query_builder::{Expression, RefExpression, StatementBuilder},
        };

        impl<'q> Expression<'q, Sqlite> for super::CurrentTable {
            fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
            where
                Sqlite: DatabaseExt,
            {
                ctx.syntax("{table}");
            }
        }

        impl<'q, C> Expression<'q, Sqlite> for super::ColumnMatchNew<C>
        where
            C: for<'b> RefExpression<'b, Sqlite>,
        {
            fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
            where
                Sqlite: DatabaseExt,
            {
                // $col = NEW.$col
                self.0.ref_expression(ctx);
                ctx.syntax(" = NEW.");
                self.0.ref_expression(ctx);
            }
        }

        impl<'q, C> Expression<'q, Sqlite> for super::ColumnNewEqualsOld<C>
        where
            C: for<'b> RefExpression<'b, Sqlite>,
        {
            fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
            where
                Sqlite: DatabaseExt,
            {
                ctx.syntax("NEW.");
                self.0.ref_expression(ctx);
                ctx.syntax(" = OLD.");
                self.0.ref_expression(ctx);
            }
        }
    }
}

pub struct SetExpression<NameTuple, Value> {
    pub name: NameTuple,
    pub value: Value,
}

impl<NameTuple, Value> OpExpression for SetExpression<NameTuple, Value> {}
impl<'q, S, NameTuple, Value> Expression<'q, S> for SetExpression<NameTuple, Value>
where
    S: DatabaseExt,
    NameTuple: for<'b> RefExpression<'b, S>,
    Value: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.name.ref_expression(ctx);
        ctx.syntax(" = ");
        self.value.expression(ctx);
    }
}

pub struct ManyStatmenets<T>(pub T);

impl<T0> OpExpression for ManyStatmenets<(T0,)> {}
impl<'q, S, T0> Expression<'q, S> for ManyStatmenets<(T0,)>
where
    T0: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.0.0.expression(ctx);
    }
}

impl<T0, T1> OpExpression for ManyStatmenets<(T0, T1)> {}

impl<'q, S, T0, T1> Expression<'q, S> for ManyStatmenets<(T0, T1)>
where
    T0: Expression<'q, S>,
    T1: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.0.0.expression(ctx);
        self.0.1.expression(ctx);
    }
}

impl<T0, T1, T2> OpExpression for ManyStatmenets<(T0, T1, T2)> {}
impl<'q, S, T0, T1, T2> Expression<'q, S> for ManyStatmenets<(T0, T1, T2)>
where
    T0: Expression<'q, S>,
    T1: Expression<'q, S>,
    T2: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.0.0.expression(ctx);
        self.0.1.expression(ctx);
        self.0.2.expression(ctx);
    }
}
