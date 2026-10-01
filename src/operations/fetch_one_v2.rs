#[linked_sql_macros::skip]
use crate::{
    collections::{Collection, CollectionId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, RowStrAliased},
    operations::{
        LinkedOutput, Operation, OperationOutput,
        fetch_many::LinkFetch,
        operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
    },
    sqlx_query_builder::{
        Expression, Join, StatementBuilder,
        combinators::{Nest, OptionalExpression},
        statements::select_statement::SelectStatement,
    },
};

pub struct FetchOne<Base, Links, Wheres> {
    pub base: Base,
    pub links: Links,
    pub wheres: Wheres,
}

#[linked_sql_macros::skip]
impl<B, L, W> OperationOutput for FetchOne<B, L, W>
where
    B: Collection,
    L: LinkFetch,
{
    type Output = Option<LinkedOutput<<B::Id as CollectionId>::IdData, B::OutputData, L::Output>>;
}

#[linked_sql_macros::skip]
impl<S, Base, Links, Wheres> Operation<S> for FetchOne<Base, Links, Wheres>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    Base: Send,
    Base: Collection,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::Id: Send + CollectionId<IdData: Send>,
    Base::Id: ExpressionsForOperation<ScopedAliased: OptionalExpression>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::ScopedAliased>: Expression<'q, S>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
    Base::OutputData: Send,
    Base: TableExpressions<
            ScopedAliased: OptionalExpression,
            PascalCase: for<'q> Expression<'q, S>,
            InheritJoin: OptionalExpression,
        >,
    for<'q> Join<Base::ScopedAliased>: Expression<'q, S>,
    for<'q> Join<Base::InheritJoin>: Expression<'q, S>,
    Links: Send,
    Links: LinkFetch,
    Links::SelectItems: Send,
    Links::SelectItems: ExpressionsForOperation<ScopedAliased: OptionalExpression>,
    for<'q> Join<<Links::SelectItems as ExpressionsForOperation>::ScopedAliased>: Expression<'q, S>,
    Links::SelectItems: for<'r> FromRowAlias<'r, S::Row, RData: Send>,
    Links::Output: Send,
    Links::Join: OptionalExpression,
    for<'q> Join<Links::Join>: Expression<'q, S>,
    Links::Wheres: OptionalExpression,
    for<'q> Join<Links::Wheres>: Expression<'q, S>,
    Links::Op: Operation<S>,
    Wheres: Send + OptionalExpression,
    for<'q> Join<Wheres>: Expression<'q, S>,
{
    fn exec_operation(self, pool: &mut <S>::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let lsi = self.links.non_aggregating_select_items();
            let id = self.base.id();

            let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
                select_items: (
                    Nest(id.scoped_aliased("i")),
                    Nest(self.base.scoped_aliased("b")),
                    Nest(lsi.scoped_aliased("l")),
                ),
                from: self.base.table_name_pascal_case(),
                joins: (
                    Nest(self.base.inherit_join()),
                    Nest(self.links.non_duplicating_join_expressions()),
                ),
                wheres: (Nest(self.wheres), Nest(self.links.where_expressions())),
                group_by: (),
                order: (),
                limit: (),
            })
            .unwrap();

            let row = S::fetch_optional(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: args,
                },
            )
            .await
            .unwrap()?;

            let id = id.str_alias(RowStrAliased::new(&row, "i")).unwrap();
            let attributes = self.base.str_alias(RowStrAliased::new(&row, "b")).unwrap();
            let link_items = lsi.str_alias(RowStrAliased::new(&row, "l")).unwrap();

            let op = self
                .links
                .operation_construct_once(&link_items)
                .exec_operation(&mut *pool)
                .await;

            Some(LinkedOutput {
                id,
                attributes,
                links: self.links.take_once(link_items, op),
            })
        }
    }
}

#[linked_sql_macros::skip]
#[cfg(test)]
mod test {
    use crate::{
        connect_in_memory::ConnectInMemory,
        operations::{LinkedOutput, Operation, fetch_one::FetchOne},
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual, ScopedColumn},
        test_module::{Todo, TodoHandler},
        track_sqlx_query::watch_sqlx_calls,
    };
    use sqlx::Sqlite;

    #[tokio::test(flavor = "current_thread")]
    async fn main() {
        watch_sqlx_calls(async |actions| {
            let mut pool = Sqlite::in_memory_connection().await;

            sqlx::query(
                "
                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT
                );
                INSERT INTO Todo (title, done, description) VALUES
                    ('todo_1', false, 'description_1'),
                    ('todo_2', true, 'description_2'),
                    ('todo_3', false, 'description_3');
                ",
            )
            .execute(&mut pool)
            .await
            .unwrap();
            actions.clear();

            let output = Operation::<Sqlite>::exec_operation(
                FetchOne {
                    base: TodoHandler,
                    links: (),
                    wheres: ColumnEqual {
                        col: ScopedColumn {
                            table: "Todo",
                            col: "id",
                        },
                        eq: Bind(2),
                    },
                },
                &mut pool,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription" FROM "Todo" WHERE "Todo"."id" = $1;"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                output,
                Some(LinkedOutput {
                    id: 2,
                    attributes: Todo {
                        title: String::from("todo_2"),
                        done: true,
                        description: Some(String::from("description_2")),
                    },
                    links: ()
                })
            );
        })
        .await;
    }
}
