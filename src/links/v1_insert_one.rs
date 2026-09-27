use crate::{
    collections::{Collection, CollectionId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, FromRowData},
    links::compat::statements::insert_statement::{InsertStatement, One},
    operations::{
        LinkedOutput, Operation, OperationOutput,
        operations_expressions_crossover::{ExpressionsForOperation, OnInsert},
    },
    sqlx_query_builder::{
        Expression, StatementBuilder, Join,
    },
};

pub trait InsertLink {
    type PreOp;
    fn pre_operation(&self) -> Self::PreOp;

    type InsertItems;
    fn insert_items(&self, pre_op: <Self::PreOp as OperationOutput>::Output) -> Self::InsertItems
    where
        Self::PreOp: OperationOutput;

    type PostOp;
    fn post_operation(
        &self,
        insert_items: &<Self::InsertItems as FromRowData>::RData,
    ) -> Self::PostOp
    where
        Self::InsertItems: FromRowData;

    type Output;
    fn take(
        self,
        pre_op: <Self::PostOp as OperationOutput>::Output,
        insert_items: <Self::InsertItems as FromRowData>::RData,
    ) -> Self::Output
    where
        Self::PostOp: OperationOutput,
        Self::InsertItems: FromRowData;
}

impl InsertLink for () {
    type PreOp = ();

    fn pre_operation(&self) -> Self::PreOp {}

    type InsertItems = ();

    fn insert_items(&self, _: <Self::PreOp as OperationOutput>::Output) -> Self::InsertItems
    where
        Self::PreOp: OperationOutput,
    {
    }

    type PostOp = ();

    type Output = ();

    fn take(
        self,
        _: <Self::PostOp as OperationOutput>::Output,
        _: <Self::InsertItems as FromRowData>::RData,
    ) -> Self::Output
    where
        Self::PostOp: OperationOutput,
        Self::InsertItems: FromRowData,
    {
    }

    fn post_operation(&self, _: &<Self::InsertItems as FromRowData>::RData) -> Self::PostOp
    where
        Self::InsertItems: FromRowData,
    {
    }
}

pub struct InsertOne<Handler, Data, Links> {
    pub base: Handler,
    pub data: Data,
    pub links: Links,
}

impl<H, L> OperationOutput for InsertOne<H, H::InputData, L>
where
    L: InsertLink,
    H: Collection,
{
    type Output = LinkedOutput<<H::Id as CollectionId>::IdData, H::OutputData, L::Output>;
}

impl<S, Base, Link> Operation<S> for InsertOne<Base, Base::InputData, Link>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    Link: InsertLink<Output: Send>,
    Link: Send,
    Base: Collection<InputData: Send, OutputData: Send, Id: Send + CollectionId<IdData: Send>>,
    Base: Send,
    Base: Clone,
    Base: ExpressionsForOperation<Identifier: for<'q> Expression<'q, S>>,
    Link::PreOp: Operation<S>,
    Link::PostOp: Operation<S>,
    Link::InsertItems: Send + Clone,
    Link::InsertItems: ExpressionsForOperation<Identifier: for<'q> Expression<'q, S>>,
    Link::InsertItems:
        OnInsert<(), InsertExpression: for<'q> Expression<'q, S>>,
    Link::InsertItems: for<'r> FromRowAlias<'r, S::Row, RData: Send>,
    Base: OnInsert<Base::InputData, InsertExpression: for<'q> Expression<'q, S>>,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::Id: ExpressionsForOperation<Identifier: for<'q> Expression<'q, S>>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
{
    fn exec_operation(self, pool: &mut S::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let pre_op = self.links.pre_operation().exec_operation(&mut *pool).await;

            let insert = self.links.insert_items(pre_op);

            let (stmt, arg) = StatementBuilder::<'_, S>::new(InsertStatement {
                table_name: self.base.table_name(),
                identifiers: Join {
                    start: "",
                    separator: ", ",
                    items: (
                        ExpressionsForOperation::identifier(&self.base),
                        ExpressionsForOperation::identifier(&insert),
                    ),
                },
                returning: Join {
                    start: "",
                    separator: ", ",
                    items: (
                        self.base.id().identifier(),
                        ExpressionsForOperation::identifier(&self.base),
                        ExpressionsForOperation::identifier(&insert),
                    ),
                },
                values: One(Join {
                    start: "",
                    separator: ", ",
                    items: (
                        self.base.on_insert(self.data),
                        insert.on_insert(()),
                    ),
                }),
            })
            .unwrap();

            let row = S::fetch_optional(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: arg,
                },
            )
            .await
            .unwrap()
            .unwrap();

            let id = self.base.id().no_alias(&row).unwrap();
            let attributes = self.base.no_alias(&row).unwrap();

            let links = {
                let ii = insert.no_alias(&row).unwrap();
                let po = self
                    .links
                    .post_operation(&ii)
                    .exec_operation(&mut *pool)
                    .await;
                self.links.take(po, ii)
            };

            LinkedOutput {
                id,
                attributes,
                links,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        connect_in_memory::ConnectInMemory,
        from_row::FromRowAlias,
        operations::{CollectionOutput, LinkedOutput, Operation},
        links::v1_insert_one::InsertOne,
        test_module::{self, Todo, todo_members, TodoHandler},
    };
    use sqlx::{Sqlite, query};

    #[tokio::test]
    async fn main() {
        let mut conn = Sqlite::in_memory_connection().await;

        query(
            "
        CREATE TABLE Todo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            done BOOLEAN NOT NULL,
            description TEXT
        );

    ",
        )
        .execute(&mut conn)
        .await
        .unwrap();

        let output = Operation::<Sqlite>::exec_operation(
            InsertOne {
                data: Todo {
                    title: String::from("todo"),
                    done: false,
                    description: None,
                },
                base: test_module::TodoHandler,
                links: (),
            },
            &mut conn,
        )
        .await;

        pretty_assertions::assert_eq!(
            output,
            LinkedOutput {
                id: 1,
                attributes: Todo {
                    title: String::from("todo"),
                    done: false,
                    description: None,
                },
                links: ()
            }
        );

        let check = sqlx::query("SELECT * FROM Todo;")
            .fetch_all(&mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|row| CollectionOutput {
                id: todo_members::id.no_alias(&row).unwrap(),
                attributes: test_module::TodoHandler.no_alias(&row).unwrap(),
            })
            .collect::<Vec<_>>();

        pretty_assertions::assert_eq!(
            check,
            vec![CollectionOutput {
                id: 1,
                attributes: Todo {
                    title: String::from("todo"),
                    done: false,
                    description: None,
                },
            }]
        );
    }
}
