use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{
        Expression, Join, OpExpression, Prefixed, StatementBuilder, combinators::OptionalExpression,
    },
};

pub struct SelectStatement<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit> {
    pub select_items: SelectItems,
    pub from: From,
    pub joins: Joins,
    pub wheres: Wheres,
    pub group_by: GroupBy,
    pub order: Order,
    pub limit: Limit,
}

impl<SelectItems, From, Joins, GroupBy, Wheres, Limit, Order> OpExpression
    for SelectStatement<SelectItems, From, Joins, GroupBy, Wheres, Limit, Order>
{
}

impl<'q, S, SelectItems, From, Joins, Wheres, Limit, Order, GroupBy> Expression<'q, S>
    for SelectStatement<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit>
where
    Join<SelectItems>: Expression<'q, S>,
    SelectItems: OptionalExpression,
    From: Expression<'q, S>,
    Join<Joins>: Expression<'q, S>,
    Join<GroupBy>: Expression<'q, S>,
    Join<Wheres>: Expression<'q, S>,
    Prefixed<Limit>: Expression<'q, S>,
    Join<Order>: Expression<'q, S>,
{
    #[track_caller]
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax("SELECT ");
        let join_select_items = Join {
            start: "",
            separator: ", ",
            items: self.select_items,
        };

        if join_select_items.items.is_oper() {
            join_select_items.expression(ctx);
        } else {
            panic!("empty select item")
        }

        ctx.syntax(" FROM ");
        self.from.expression(ctx);
        Join {
            start: " ",
            separator: ", ",
            items: self.joins,
        }
        .expression(ctx);
        Join {
            start: " WHERE ",
            separator: " AND ",
            items: self.wheres,
        }
        .expression(ctx);
        Join {
            start: " GROUP BY ",
            separator: ", ",
            items: self.group_by,
        }
        .expression(ctx);
        Join {
            start: " ORDER BY ",
            separator: ", ",
            items: self.order,
        }
        .expression(ctx);
        Prefixed {
            prefix: " LIMIT ",
            inner: self.limit,
        }
        .expression(ctx);
        ctx.syntax(";");
    }
}

pub struct SelectStatementV2<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit> {
    pub select_items: SelectItems,
    pub from: From,
    pub joins: Joins,
    pub wheres: Wheres,
    pub group_by: GroupBy,
    pub order: Order,
    pub limit: Limit,
}

impl<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit> OpExpression
    for SelectStatementV2<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit>
{
}

impl<'q, S, SelectItems, From, Joins, Wheres, Limit, Order, GroupBy> Expression<'q, S>
    for SelectStatementV2<SelectItems, From, Joins, Wheres, GroupBy, Order, Limit>
where
    SelectItems: Expression<'q, S>,
    From: Expression<'q, S>,
    Limit: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax("SELECT ");
        self.select_items.expression(ctx);
        self.from.expression(ctx);
        self.limit.expression(ctx);
        ctx.syntax(";");
    }
}

pub struct LimitClause<Limit> {
    pub limit: Limit,
}

impl<Limit> OpExpression for LimitClause<Limit> {
}

impl<'q, S, Limit> Expression<'q, S> for LimitClause<Limit>
where
    Limit: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax(" LIMIT ");
        self.limit.expression(ctx);
    }
}

pub struct FromClause<From> {
    pub from: From,
}

impl<From> OpExpression for FromClause<From> {
}

impl<'q, S, From> Expression<'q, S> for FromClause<From>
where
    From: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.syntax(" FROM ");
        self.from.expression(ctx);
    }
}

pub struct Aliased<Alias, Aliased> {
    pub aliased: Aliased,
    pub alias: Alias,
}

impl<A, Ad> OpExpression for Aliased<A, Ad> {
}

impl<'q, S, A, Ad> Expression<'q, S> for Aliased<A, Ad>
where
    A: Expression<'q, S>,
    Ad: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        self.aliased.expression(ctx);
        ctx.syntax(" AS ");
        self.alias.expression(ctx);
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Sqlite;

    use crate::sqlx_query_builder::{
        Bind, StatementBuilder,
        statements::select_statement::{Aliased, FromClause, LimitClause, SelectStatementV2},
    };

    #[test]
    fn test_select_statement_v2() {
        let alias = String::from("value");

        let stmt = SelectStatementV2 {
            select_items: Aliased {
                aliased: Bind(1),
                alias: alias.as_str(),
            },
            from: FromClause { from: "table" },
            joins: (),
            wheres: (),
            order: (),
            limit: LimitClause { limit: Bind(3) },
            group_by: (),
        };

        let stmt = StatementBuilder::<Sqlite>::new(stmt).unwrap().0;
        assert_eq!(stmt, r#"SELECT $1 AS "value" FROM "table" LIMIT $2;"#);
    }
}
