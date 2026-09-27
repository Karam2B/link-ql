use crate::sqlx_query_builder::OpExpression;

pub struct AddColumn<Table, ColDef> {
    pub table: Table,
    pub col_def: ColDef,
}

pub struct DropColumn<Table, Name> {
    pub table: Table,
    pub col_name: Name,
}

impl<Table, ColDef> OpExpression for AddColumn<Table, ColDef> {
}

mod impl_for_sqlx_fo {
    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{
            Expression, StatementBuilder, statements::add_column_statement::AddColumn,
        },
    };
    use sqlx::Database;

    impl<'q, S, Table, ColDef> Expression<'q, S> for AddColumn<Table, ColDef>
    where
        S: Database + DatabaseExt,
        Table: Expression<'q, S>,
        ColDef: Expression<'q, S>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.syntax("ALTER TABLE ");
            self.table.expression(ctx);
            ctx.syntax(" ADD COLUMN ");
            self.col_def.expression(ctx);
            ctx.syntax(";");
        }
    }
}
