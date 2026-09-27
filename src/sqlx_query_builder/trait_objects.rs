use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{
        Expression, Join, OpExpression, StatementBuilder, combinators::OptionalExpression,
    },
};

pub trait BoxedExpression<S: DatabaseExt>: Send {
    fn boxed_expression<'q>(self: Box<Self>, ctx: &mut StatementBuilder<'q, S>);
    fn boxed_is_op_own(&self) -> bool;
}
impl<E, S> BoxedExpression<S> for E
where
    S: DatabaseExt,
    E: for<'e> Expression<'e, S> + Send,
{
    fn boxed_expression<'q>(self: Box<Self>, ctx: &mut StatementBuilder<'q, S>) {
        Expression::expression(*self, ctx);
    }

    fn boxed_is_op_own(&self) -> bool {
        self.is_expression_present()
    }
}

impl<S: DatabaseExt> BoxedExpression<S> for () {
    fn boxed_expression<'q>(self: Box<Self>, _: &mut StatementBuilder<'q, S>) {}

    fn boxed_is_op_own(&self) -> bool {
        false
    }
}

impl<S> OpExpression for Box<dyn BoxedExpression<S> + Send>
where
    S: DatabaseExt,
{
    fn is_expression_present(&self) -> bool {
        BoxedExpression::boxed_is_op_own(self.as_ref())
    }
}

pub fn box_expression<S, T>(
    items: T,
    separator: &'static str,
) -> Box<dyn BoxedExpression<S> + Send>
where
    S: DatabaseExt,
    T: OptionalExpression + Send + 'static,
    Join<T>: for<'e> Expression<'e, S> + Send,
{
    if !items.is_oper() {
        Box::new(())
    } else {
        Box::new(Join {
            start: "",
            separator,
            items,
        })
    }
}

impl<'q, S> Expression<'q, S> for Box<dyn BoxedExpression<S> + Send>
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        BoxedExpression::boxed_expression(self, ctx);
    }
}
