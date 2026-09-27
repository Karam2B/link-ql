use crate::database_extention::DatabaseExt;
use crate::sqlx_query_builder::{Expression, Join, OpExpression, StatementBuilder};

pub struct UpdateStatement<TableName, Values, Wheres, Returning> {
    pub table_name: TableName,
    pub values: Values,
    pub wheres: Wheres,
    pub returning: Returning,
}

impl<TableName, Values, Wheres, Returning> OpExpression
    for UpdateStatement<TableName, Values, Wheres, Returning>
{
}

impl<'q, S, TableName, Values, Wheres, Returning> Expression<'q, S>
    for UpdateStatement<TableName, Values, Wheres, Returning>
where
    S: DatabaseExt,
    TableName: Expression<'q, S>,
    Join<Values>: Expression<'q, S>,
    Join<Wheres>: Expression<'q, S>,
    Join<Returning>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("UPDATE ");
        self.table_name.expression(ctx);
        ctx.syntax(" SET ");
        Join {
            start: "",
            separator: ", ",
            items: self.values,
        }
        .expression(ctx);
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
