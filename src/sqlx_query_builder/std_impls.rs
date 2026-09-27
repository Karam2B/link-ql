use std::sync::Arc;

use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{OpExpression, RefExpression, RefOpExpression, StatementBuilder},
};

impl OpExpression for String {}

impl RefOpExpression for String {}

impl<S> RefExpression<'_, S> for String
where
    S: DatabaseExt,
{
    fn ref_expression<'q>(&self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.sanitize(self.as_str());
    }
}

impl OpExpression for Arc<str> {}

impl RefOpExpression for Arc<str> {}

impl<S> RefExpression<'_, S> for Arc<str>
where
    S: DatabaseExt,
{
    fn ref_expression<'q>(&self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.sanitize(self.as_ref());
    }
}

impl OpExpression for &'_ str {}

impl RefOpExpression for &'_ str {}

impl<S> RefExpression<'_, S> for &'_ str
where
    S: DatabaseExt,
{
    fn ref_expression<'q>(&self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt,
    {
        ctx.sanitize(self);
    }
}
