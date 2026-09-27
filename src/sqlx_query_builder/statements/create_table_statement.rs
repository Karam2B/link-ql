use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{Expression, Join, OpExpression, StatementBuilder},
};

pub struct CreateTable<Init, TableName, ColDefs> {
    pub init: Init,
    pub name: TableName,
    pub col_defs: ColDefs,
}

pub mod expressions {
    #![allow(non_camel_case_types)]

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
    };

    pub struct CreateTableInit;

    impl OpExpression for CreateTableInit {
    }

    impl<'q, S> Expression<'q, S> for CreateTableInit {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>)
        where
            S: DatabaseExt,
        {
            ctx.syntax(&"CREATE TABLE");
        }
    }

    pub struct CreateTableInitIfNotExist;

    impl OpExpression for CreateTableInitIfNotExist {
    }

    impl<'q, S> Expression<'q, S> for CreateTableInitIfNotExist {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>)
        where
            S: DatabaseExt,
        {
            ctx.syntax(&"CREATE TABLE IF NOT EXISTS");
        }
    }
}

impl<Header, Table, Columns> OpExpression for CreateTable<Header, Table, Columns> {
}

impl<'q, S, Header, Table, Columns> Expression<'q, S> for CreateTable<Header, Table, Columns>
where
    S: DatabaseExt,
    Header: Expression<'q, S>,
    Table: Expression<'q, S>,
    Join<Columns>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        let open_b = "(";
        let close_b = ");";
        self.init.expression(ctx);
        ctx.syntax(&" ");
        self.name.expression(ctx);
        ctx.syntax(&" ");
        ctx.syntax(&open_b);
        Join {
            start: "",
            separator: ", ",
            items: self.col_defs,
        }
        .expression(ctx);
        ctx.syntax(&close_b);
    }
}

pub struct ColumnDefinition<C>(pub C);

impl<C> OpExpression for ColumnDefinition<C> {}

impl<'q, S, C> Expression<'q, S> for ColumnDefinition<C>
where
    Join<C>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        Join {
            start: "",
            separator: " ",
            items: self.0,
        }
        .expression(ctx);
    }
}

pub struct NotNull;
impl OpExpression for NotNull {
}
impl<'a, S> Expression<'a, S> for NotNull {
    fn expression(self, ctx: &mut crate::sqlx_query_builder::StatementBuilder<'a, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax("NOT NULL");
    }
}

#[cfg(feature = "skip_without_comments")]
mod old_dynamic_statement {
    #![allow(unused)]
    use std::{marker::PhantomData, ops::Not};

    use crate::{Buildable, ColumPositionConstraint, ExpressionToFragment, QueryBuilder};

    #[derive(Debug)]
    pub struct CreateTableSt<S: StatementBuilder> {
        pub(crate) header: String,
        pub(crate) ident: (Option<String>, String),
        pub(crate) columns: Vec<(String, S::Fragment)>,
        pub(crate) constraints: Vec<S::Fragment>,
        pub(crate) verbatim: Vec<String>,
        pub(crate) ctx: S,
        pub(crate) _sqlx: PhantomData<S>,
    }

    #[allow(non_upper_case_globals)]
    pub mod header {
        pub const create: &'static str = "CREATE TABLE";
        pub const create_temp: &'static str = "CREATE TEMP";
        pub const create_temp_if_not_exists: &'static str = "CREATE TEMP IF NOT EXISTS";
        pub const create_table_if_not_exists: &'static str = "CREATE TABLE IF NOT EXISTS";
    }

    impl<S: StatementBuilder> Buildable for CreateTableSt<S> {
        type QueryBuilder = S;

        fn build(self) -> (String, S::Output) {
            S::to_output(self.ctx, |ctx| {
                let mut str = String::from(&self.header);
                str.push(' ');

                if let Some(schema) = self.ident.0 {
                    str.push_str(&schema);
                }

                str.push_str(self.ident.1.as_ref());

                str.push_str(" (");

                let mut clauses = Vec::new();
                for (mut col, constrain) in self.columns {
                    let constrain = S::fragment_to_string(ctx, constrain);
                    if constrain.is_empty().not() {
                        col.push_str(&format!(" {}", constrain))
                    }
                    clauses.push(col);
                }
                for constraint in self.constraints {
                    let item = S::fragment_to_string(ctx, constraint);
                    clauses.push(item);
                }

                for verbatim in self.verbatim {
                    clauses.push(verbatim);
                }
                if clauses.is_empty() {
                    panic!("columns is empty");
                }
                str.push_str(&clauses.join(", "));
                str.push_str(");");
                str
            })
        }
    }

    impl<Q: StatementBuilder + Default> CreateTableSt<Q> {
        pub fn init(header: &str, table: &str) -> Self {
            Self {
                header: header.to_string(),
                ident: (None, table.to_string()),
                columns: Default::default(),
                constraints: Default::default(),
                verbatim: Default::default(),
                ctx: Default::default(),
                _sqlx: PhantomData,
            }
        }
        pub fn column_def<C>(&mut self, name: &str, constraint: C)
        where
            Q: ExpressionToFragment<'static, C> + ColumPositionConstraint,
        {
            let item = Q::expression_to_fragment(&mut self.ctx, constraint);
            self.columns.push((name.to_string(), item));
        }
    }
}
