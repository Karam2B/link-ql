use crate::{
    database_extention::DatabaseExt,
    sqlx_query_builder::{
        Expression, Join, OpExpression, StatementBuilder, combinators::OptionalExpression,
    },
};

pub struct CreateTrigger<TriggerName, Lifetime, OperationName, OnTable, WhenExpression, Statements>
{
    pub trigger_name: TriggerName,
    pub temp: bool,
    pub if_not_exists: bool,
    pub lifetime: Lifetime,
    pub operation_name: OperationName,
    pub on_table: OnTable,
    pub for_each_row: bool,
    pub when_expression: WhenExpression,
    pub statements: Statements,
}

impl<Tn, Lt, On, Ont, We, St> OpExpression for CreateTrigger<Tn, Lt, On, Ont, We, St> {}

impl<'q, S, TriggerName, Lifetime, OperationName, OnTable, WhenExpression, Statements>
    Expression<'q, S>
    for CreateTrigger<TriggerName, Lifetime, OperationName, OnTable, WhenExpression, Statements>
where
    S: DatabaseExt,
    TriggerName: Expression<'q, S>,
    Lifetime: Expression<'q, S>,
    OperationName: Expression<'q, S>,
    OnTable: Expression<'q, S>,
    WhenExpression: OptionalExpression,
    Join<WhenExpression>: Expression<'q, S>,
    Join<Statements>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("CREATE ");
        if self.temp {
            ctx.syntax("TEMP ");
        }
        if self.if_not_exists {
            ctx.syntax("IF NOT EXISTS ");
        }
        ctx.syntax("TRIGGER ");
        self.trigger_name.expression(ctx);
        ctx.syntax(" ");
        self.lifetime.expression(ctx);
        ctx.syntax(" ");
        self.operation_name.expression(ctx);
        ctx.syntax(" ON ");
        self.on_table.expression(ctx);
        if self.for_each_row {
            ctx.syntax(" FOR EACH ROW ");
        }
        Join {
            start: " WHEN ",
            separator: " ",
            items: self.when_expression,
        }
        .expression(ctx);
        ctx.syntax(" BEGIN ");
        Join {
            start: "",
            separator: ";",
            items: self.statements,
        }
        .expression(ctx);
        ctx.syntax(" END;");
    }
}

pub struct TriggerLifetimeBefore;

impl OpExpression for TriggerLifetimeBefore {}

impl<'q, S> Expression<'q, S> for TriggerLifetimeBefore
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("BEFORE");
    }
}

pub struct TriggerLifetimeAfter;

impl OpExpression for TriggerLifetimeAfter {}

impl<'q, S> Expression<'q, S> for TriggerLifetimeAfter
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("AFTER");
    }
}

pub struct TriggerLifetimeInsteadOf;

impl OpExpression for TriggerLifetimeInsteadOf {}

impl<'q, S> Expression<'q, S> for TriggerLifetimeInsteadOf
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("INSTEAD OF");
    }
}

pub struct TriggerOperationNameInsert;

impl OpExpression for TriggerOperationNameInsert {}

impl<'q, S> Expression<'q, S> for TriggerOperationNameInsert
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("INSERT");
    }
}

pub struct TriggerOperationNameUpdate<OfColumns>(pub OfColumns);

impl<OfColumns> OpExpression for TriggerOperationNameUpdate<OfColumns> {}

impl<'q, S, OfColumns> Expression<'q, S> for TriggerOperationNameUpdate<OfColumns>
where
    S: DatabaseExt,
    Join<OfColumns>: Expression<'q, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.syntax("UPDATE");
        Join {
            start: " OF ",
            separator: ", ",
            items: self.0,
        }
        .expression(ctx);
    }
}
