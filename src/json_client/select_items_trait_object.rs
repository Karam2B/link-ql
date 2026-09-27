
use crate::{
    from_row::{
        FromRowAlias, FromRowData, FromRowError, RowNumAliased, RowStrAliased,
    },
    operations::operations_expressions_crossover::ExpressionsForOperation,
    sqlx_query_builder::{
        Expression, Join,
        combinators::OptionalExpression,
        trait_objects::{box_expression, BoxedExpression},
    },
};
use sqlx::Database;
use std::any::Any;

pub trait SelectItemsTraitObject<S, CastFromRowResult>: Send {
    fn str_alias_erase(&self, alias: &'static str) -> Box<dyn BoxedExpression<S> + Send>;

    fn num_alias_erase(
        &self,
        num: usize,
        alias: &'static str,
    ) -> Box<dyn BoxedExpression<S> + Send>;

    fn no_alias_2<'r>(&self, row: &'r S::Row) -> Result<Box<dyn Any + Send>, FromRowError>
    where
        S: Database;

    fn str_alias_2<'r>(
        &self,
        row: RowStrAliased<'r, S::Row>,
    ) -> Result<Box<dyn Any + Send>, FromRowError>
    where
        S: Database,
        S::Row: sqlx::Row;

    fn num_alias_2<'r>(
        &self,
        row: RowNumAliased<'r, S::Row>,
    ) -> Result<Box<dyn Any + Send>, FromRowError>
    where
        S: Database,
        S::Row: sqlx::Row;
}

pub struct ToImplSelectItems<Se, CastFromRowResult> {
    pub select_items: Se,
    pub cast_from_row_result: CastFromRowResult,
}

impl<Se, S> SelectItemsTraitObject<S, ()> for ToImplSelectItems<Se, ()>
where
    Se: Send,
    Se: ExpressionsForOperation<
        ScopedAliased: 'static + Send + OptionalExpression,
        NumScopedAliased: 'static + Send + OptionalExpression,
    >,
    for<'e> Join<<Se as ExpressionsForOperation>::ScopedAliased>: Expression<'e, S>,
    for<'e> Join<<Se as ExpressionsForOperation>::NumScopedAliased>: Expression<'e, S>,
    Se: for<'r> FromRowAlias<'r, S::Row>,
    Se: FromRowData<RData: Send + 'static>,
    S: Database,
    S: crate::database_extention::DatabaseExt,
{
    fn str_alias_erase(&self, alias: &'static str) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.select_items.scoped_aliased(alias), ", ")
    }
    fn num_alias_erase(
        &self,
        num: usize,
        alias: &'static str,
    ) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.select_items.num_scoped_aliased(num, alias), ", ")
    }
    fn no_alias_2<'r>(&self, row: &'r S::Row) -> Result<Box<dyn Any + Send>, FromRowError> {
        Ok(Box::new(self.select_items.no_alias(row)?))
    }
    fn str_alias_2<'r>(
        &self,
        row: RowStrAliased<'r, S::Row>,
    ) -> Result<Box<dyn Any + Send>, FromRowError> {
        let ret = self.select_items.str_alias(row)?;
        Ok(Box::new(ret))
    }
    fn num_alias_2<'r>(
        &self,
        row: RowNumAliased<'r, S::Row>,
    ) -> Result<Box<dyn Any + Send>, FromRowError> {
        Ok(Box::new(self.select_items.num_alias(row)?))
    }
}

impl<'r, S, C> ExpressionsForOperation for Box<dyn SelectItemsTraitObject<S, C> + 'r> {
    type Identifier = ();
    fn identifier(&self) -> Self::Identifier {}

    type Scoped = ();
    fn scoped(&self) -> Self::Scoped {}

    type ScopedAliased = Box<dyn BoxedExpression<S> + Send>;
    fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
        self.str_alias_erase(alias)
    }

    type NumScopedAliased = Box<dyn BoxedExpression<S> + Send>;
    fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
        self.num_alias_erase(num, alias)
    }
}

impl<'r, S> FromRowData for Box<dyn SelectItemsTraitObject<S, ()> + 'r> {
    type RData = Box<dyn Any + Send>;
}

impl<'r, 'b, S: Database> FromRowAlias<'r, S::Row> for Box<dyn SelectItemsTraitObject<S, ()> + 'b> {
    fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
        Ok(self.no_alias_2(row)?)
    }
    fn str_alias(&self, row: RowStrAliased<'r, S::Row>) -> Result<Self::RData, FromRowError> {
        Ok(self.str_alias_2(row)?)
    }

    fn num_alias(&self, row: RowNumAliased<'r, S::Row>) -> Result<Self::RData, FromRowError>
    where
        S::Row: sqlx::Row,
    {
        Ok(self.num_alias_2(row)?)
    }
}
impl<'r, S> FromRowData for Vec<Box<dyn SelectItemsTraitObject<S, ()> + 'r>> {
    type RData = Vec<Box<dyn Any + Send>>;
}

impl<'r, 'b, S: Database> FromRowAlias<'r, S::Row>
    for Vec<Box<dyn SelectItemsTraitObject<S, ()> + 'b>>
{
    fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
        let mut v = vec![];
        for (i, each) in self.iter().enumerate() {
            v.push(each.num_alias(RowNumAliased {
                row: row,
                str_alias: "",
                num_alias: Some(i),
            })?);
        }
        Ok(v)
    }
    fn str_alias(&self, row: RowStrAliased<'r, S::Row>) -> Result<Self::RData, FromRowError> {
        let mut v = vec![];
        for (i, each) in self.iter().enumerate() {
            v.push(each.num_alias(RowNumAliased {
                row: row.row,
                str_alias: row.alias,
                num_alias: Some(i),
            })?);
        }
        Ok(v)
    }

    fn num_alias(&self, _: RowNumAliased<'r, S::Row>) -> Result<Self::RData, FromRowError>
    where
        S::Row: sqlx::Row,
    {
        panic!("nesting where it was not expected");
    }
}

#[cfg(test)]
mod test {
    use crate::{
        connect_in_memory::ConnectInMemory,
        links::{DefaultRelationKey, relation_one_to_many::OneToMany},
        operations::{Operation, fetch_many::FetchMany},
        test_module::{CategoryHandler, TodoHandler},
    };
    use sqlx::Sqlite;

    #[tokio::test]
    async fn test_ref_link() {
        let mut db = Sqlite::in_memory_connection().await;

        sqlx::query(
            r#"
        CREATE TABLE Category ( 
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT
        );
        CREATE TABLE Todo ( 
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT, 
            done BOOLEAN, 
            description TEXT, 
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            fk_category_def INTEGER, FOREIGN KEY (fk_category_def) REFERENCES Category(id)
        );

        INSERT INTO Category (title) VALUES 
        ('category_1'), ('category_2'), ('category_3');

        INSERT INTO Todo
            (title, done, description, fk_category_def, created_at, updated_at)
        VALUES
            ('first_todo', true, 'description_1', 1, 'test_0', 'test_1'),
            ('second_todo', false, 'description_2', NULL, 'test_2', 'test_3'),
            ('third_todo', true, 'description_3', 2, 'test_4', 'test_5'),
            ('fourth_todo', false, 'description_4', 2, 'test_6', 'test_7');
    "#,
        )
        .execute(&mut db)
        .await
        .unwrap();

        let output = Operation::<Sqlite>::exec_operation(
            FetchMany {
                base: TodoHandler,
                wheres: (),
                links: OneToMany {
                    from: TodoHandler,
                    to: CategoryHandler,
                    fk_unique_id: DefaultRelationKey,
                },
                cursor_order_by: (),
                cursor_first_item: None::<(i64, ())>,
                limit: 10,
            },
            &mut db,
        )
        .await;

        assert_eq!(output.items.len(), 4);
        assert_eq!(output.items[0].id, 1);
        assert_eq!(output.items[0].attributes.title, "first_todo");
        assert!(output.items[0].links.is_some());
        assert_eq!(output.items[0].links.as_ref().unwrap().id, 1);
        assert_eq!(
            output.items[0].links.as_ref().unwrap().attributes.title,
            "category_1"
        );
        assert!(output.items[1].links.is_none());
        assert!(output.next_item.is_none());
    }
}
