use crate::sqlx_query_builder::{Expression, Join, OpExpression, combinators::OptionalExpression};

pub struct InsertStatement<TableName, Identifiers, Values, Returning> {
    pub table_name: TableName,
    pub identifiers: Identifiers,
    pub values: Values,
    pub returning: Returning,
}

impl<TableName, Identifiers, Values, Returning> OpExpression
    for InsertStatement<TableName, Identifiers, Values, Returning>
{
}

/// used to avoid conflicting implementaion with Vec<T> in generic implementations
pub struct One<T>(pub T);

impl<'q, S, TableName, Identifiers, Values, Returning> Expression<'q, S>
    for InsertStatement<TableName, Identifiers, One<Values>, Returning>
where
    S: crate::database_extention::DatabaseExt,
    TableName: Expression<'q, S> + OpExpression,
    Identifiers: Expression<'q, S> + OptionalExpression,
    Values: Expression<'q, S> + OptionalExpression,
    Join<Returning>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut crate::sqlx_query_builder::StatementBuilder<'q, S>)
    where
        S: crate::database_extention::DatabaseExt,
    {
        if !self.identifiers.is_oper() || !self.values.0.is_oper() {
            panic!("insert_statment with empty values")
        }
        ctx.syntax("INSERT INTO ");
        self.table_name.expression(ctx);
        ctx.syntax(" ");
        ctx.syntax("(");
        self.identifiers.expression(ctx);
        ctx.syntax(")");
        ctx.syntax(" VALUES ");
        ctx.syntax("(");
        self.values.0.expression(ctx);
        ctx.syntax(")");

        Join {
            start: " RETURNING ",
            separator: ", ",
            items: self.returning,
        }
        .expression(ctx);

        ctx.syntax(";");
    }
}

impl<'q, S, TableName, Identifiers, Values, Returning> Expression<'q, S>
    for InsertStatement<TableName, Identifiers, Vec<Values>, Returning>
where
    S: crate::database_extention::DatabaseExt,
    TableName: Expression<'q, S>,
    Identifiers: Expression<'q, S>,
    Values: Expression<'q, S>,
    Join<Returning>: Expression<'q, S>,
{
    fn expression(mut self, ctx: &mut crate::sqlx_query_builder::StatementBuilder<'q, S>)
    where
        S: crate::database_extention::DatabaseExt,
    {
        ctx.syntax("INSERT INTO ");
        self.table_name.expression(ctx);
        ctx.syntax(" ");
        ctx.syntax("(");
        self.identifiers.expression(ctx);
        ctx.syntax(")");
        ctx.syntax(" VALUES ");

        let pop = self.values.pop();
        for each in self.values {
            ctx.syntax("(");
            each.expression(ctx);
            ctx.syntax(")");
            ctx.syntax(", ");
        }

        if let Some(last) = pop {
            ctx.syntax("(");
            last.expression(ctx);
            ctx.syntax(")");
        }

        Join {
            start: " RETURNING ",
            separator: ", ",
            items: self.returning,
        }
        .expression(ctx);

        ctx.syntax(";");
    }
}

pub struct IteratorSpec<T>(pub T);

impl<'q, S, TableName, Identifiers, Values, Returning> Expression<'q, S>
    for InsertStatement<TableName, Identifiers, IteratorSpec<Values>, Returning>
where
    S: crate::database_extention::DatabaseExt,
    TableName: Expression<'q, S>,
    Identifiers: Expression<'q, S>,
    Values: IntoIterator<Item: Expression<'q, S>>,
    Join<Returning>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut crate::sqlx_query_builder::StatementBuilder<'q, S>)
    where
        S: crate::database_extention::DatabaseExt,
    {
        ctx.syntax("INSERT INTO ");
        self.table_name.expression(ctx);
        ctx.syntax(" ");
        ctx.syntax("(");
        self.identifiers.expression(ctx);
        ctx.syntax(")");
        ctx.syntax(" VALUES ");

        let mut iter = self.values.0.into_iter();

        let first = iter
            .next()
            .expect("should have at least one item to insert for a valid insert statement");

        ctx.syntax("(");
        first.expression(ctx);
        ctx.syntax(")");

        for each in iter {
            ctx.syntax(", ");
            ctx.syntax("(");
            each.expression(ctx);
            ctx.syntax(")");
        }

        Join {
            start: " RETURNING ",
            separator: ", ",
            items: self.returning,
        }
        .expression(ctx);

        ctx.syntax(";");
    }
}
