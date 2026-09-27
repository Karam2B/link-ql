use crate::sqlx_query_builder::{Expression, Join, OpExpression};

pub struct DeleteStatement<TableName, Wheres, Returning> {
    pub table_name: TableName,
    pub wheres: Wheres,
    pub returning: Returning,
}

impl<T, W, R> OpExpression for DeleteStatement<T, W, R> {
}

impl<'q, S, TableName, Wheres, Returning> Expression<'q, S>
    for DeleteStatement<TableName, Wheres, Returning>
where
    TableName: Expression<'q, S>,
    Join<Wheres>: Expression<'q, S>,
    Join<Returning>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut crate::sqlx_query_builder::StatementBuilder<'q, S>)
    where
        S: crate::database_extention::DatabaseExt,
    {
        ctx.syntax("DELETE FROM ");
        self.table_name.expression(ctx);
        Join {
            start: " WHERE ",
            separator: " AND ",
            items: self.wheres,
        }
        .expression(ctx);
        Join {
            start: " RETURNING ",
            separator: ", ",
            items: self.returning,
        }
        .expression(ctx);
        ctx.syntax(";");
    }
}
