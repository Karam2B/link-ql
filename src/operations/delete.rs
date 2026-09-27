use crate::{
    collections::{Collection, CollectionId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, FromRowData},
    operations::{
        LinkedOutput, Operation, OperationOutput,
        operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
    },
    sqlx_query_builder::{
        Expression, Join, StatementBuilder,
        combinators::{Nest, OptionalExpression},
        statements::delete_statement::DeleteStatement,
    },
};

pub struct Delete<Base, Wheres, Links> {
    pub base: Base,
    pub wheres: Wheres,
    pub links: Links,
}

pub trait DeleteLinkSplit {
    type Link: DeleteLink;
    type InitSplitForPreOp;
    fn init_split(
        self,
    ) -> (
        Self::Link,
        Self::InitSplitForPreOp,
        DeleteLinkData<<Self::Link as DeleteLink>::InitSplitForWheres>,
    );
}

pub struct DeleteLinkData<Wheres> {
    pub wheres: Wheres,
}

pub trait DeleteLinkPreOp<WhereExpression: Clone>: DeleteLink {
    type InitSplitForPreOp;
    type PreOp: OperationOutput<Output = Self::PreOpOutput>;
    fn pre_op(
        &self,
        init_pre_split: Self::InitSplitForPreOp,
        wheres: &WhereExpression,
    ) -> Self::PreOp;
}

pub trait DeleteLink {
    type Output;
    type PreOpOutput;
    type PreOpSplitWheres;
    type PreOpSplitTake;
    fn split_pre_op(
        &self,
        pre_op: Self::PreOpOutput,
    ) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake);

    type InitSplitForWheres;
    type Wheres;
    fn wheres(
        &self,
        init_split_for_wheres: Self::InitSplitForWheres,
        pre_op_split_wheres: Self::PreOpSplitWheres,
    ) -> Self::Wheres;

    type DeleteReturnExpression;
    fn delete_return_expression(&self) -> Self::DeleteReturnExpression;

    type DeleteReturnFromRow: FromRowData;
    fn from_row(&self) -> Self::DeleteReturnFromRow;

    fn take_mut(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output;

    fn take_once(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: Self::PreOpSplitTake,
    ) -> Self::Output;
}

impl DeleteLinkSplit for () {
    type Link = ();

    type InitSplitForPreOp = ();

    fn init_split(self) -> (Self::Link, Self::InitSplitForPreOp, DeleteLinkData<()>) {
        ((), (), DeleteLinkData { wheres: () })
    }
}

impl<W: Clone> DeleteLinkPreOp<W> for () {
    type InitSplitForPreOp = ();
    type PreOp = ();
    fn pre_op(&self, _: Self::InitSplitForPreOp, _: &W) -> Self::PreOp {}
}

impl DeleteLink for () {
    type Output = ();
    type PreOpOutput = ();
    type PreOpSplitWheres = ();
    type PreOpSplitTake = ();
    fn split_pre_op(&self, _: Self::PreOpOutput) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake) {
        ((), ())
    }
    type InitSplitForWheres = ();
    type Wheres = ();
    fn wheres(&self, _: Self::InitSplitForWheres, _: Self::PreOpSplitWheres) -> Self::Wheres {
        ()
    }

    type DeleteReturnExpression = ();
    fn delete_return_expression(&self) -> Self::DeleteReturnExpression {}

    type DeleteReturnFromRow = ();
    fn from_row(&self) -> Self::DeleteReturnFromRow {}

    fn take_once(
        &self,
        _: <Self::DeleteReturnFromRow as FromRowData>::RData,
        _: Self::PreOpSplitTake,
    ) -> Self::Output {
    }

    fn take_mut(
        &self,
        _: <Self::DeleteReturnFromRow as FromRowData>::RData,
        _: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
    }
}

impl<Base, Wheres, PL, Links> OperationOutput for Delete<Base, Wheres, PL>
where
    Base: Collection,
    PL: DeleteLinkSplit<Link = Links>,
    Links: DeleteLink,
{
    type Output = Vec<
        LinkedOutput<
            <Base::Id as CollectionId>::IdData,
            <Base as Collection>::OutputData,
            Links::Output,
        >,
    >;
}

impl<S, Base, Wheres, PL, Links> Operation<S> for Delete<Base, Wheres, PL>
where
    S: DatabaseExt + ExecutorTrait,
    PL: DeleteLinkSplit<Link = Links>,
    PL::InitSplitForPreOp: Send,
    PL: Send,
    Base: Send,
    Base: Collection,
    Base: TableExpressions<
            Identifier: OptionalExpression,
            PascalCase: for<'q> Expression<'q, S>,
        >,
    for<'q> Join<Base::Identifier>: Expression<'q, S>,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::OutputData: Send,
    Base::Id: Send + CollectionId<IdData: Send>,
    Base::Id: ExpressionsForOperation<Identifier: OptionalExpression>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::Identifier>: Expression<'q, S>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
    Wheres: Send + OptionalExpression,
    for<'q> Join<Wheres>: Expression<'q, S>,
    Links: Send,
    Links: DeleteLink,
    Links::Output: Send,
    Wheres: Clone,
    Links: DeleteLinkPreOp<Wheres, InitSplitForPreOp = PL::InitSplitForPreOp>,
    Links::PreOp: Send + Operation<S, Output = Links::PreOpOutput>,
    Links::InitSplitForWheres: Send,
    Links::Wheres: OptionalExpression,
    for<'q> Join<Links::Wheres>: Expression<'q, S>,
    Links::DeleteReturnExpression: OptionalExpression,
    for<'q> Join<Links::DeleteReturnExpression>: Expression<'q, S>,
    Links::DeleteReturnFromRow: Send,
    Links::DeleteReturnFromRow: for<'r> FromRowAlias<'r, S::Row>,
    Links::Output: Send,
    // for closure
    Links::DeleteReturnFromRow: Sync,
    Links: Sync,
    Links::PreOpSplitTake: Send,
    Base::Id: Sync,
    Base: Sync,
{
    fn exec_operation(self, pool: &mut <S>::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let (link, init_pre_split, link_data) = self.links.init_split();

            let output = link
                .pre_op(init_pre_split, &self.wheres)
                .exec_operation(&mut *pool)
                .await;

            let (pre_op_split_wheres, mut pre_op_split_take) = link.split_pre_op(output);

            let id = self.base.id();

            let (stmt, args) = StatementBuilder::<S>::new(DeleteStatement {
                table_name: self.base.table_name_pascal_case(),
                wheres: (
                    Nest(self.wheres),
                    Nest(link.wheres(link_data.wheres, pre_op_split_wheres)),
                ),
                returning: (
                    Nest(id.identifier()),
                    Nest(self.base.identifier()),
                    Nest(link.delete_return_expression()),
                ),
            })
            .unwrap();

            let link_from_row = link.from_row();

            let mut res = S::fetch_all_mapped(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: args,
                },
                |row| {
                    let id = id.no_alias(&row).unwrap();
                    let attributes = self.base.no_alias(&row).unwrap();
                    let links = link_from_row.no_alias(&row).unwrap();
                    LinkedOutput {
                        id,
                        attributes,
                        links,
                    }
                },
            )
            .await
            .unwrap();

            if res.len() == 1 {
                let first = res.pop().unwrap();
                let links = link.take_once(first.links, pre_op_split_take);
                return vec![LinkedOutput {
                    id: first.id,
                    attributes: first.attributes,
                    links,
                }];
            } else {
                res.into_iter()
                    .map(|each| {
                        let links = link.take_mut(each.links, &mut pre_op_split_take);
                        LinkedOutput {
                            id: each.id,
                            attributes: each.attributes,
                            links,
                        }
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        connect_in_memory::ConnectInMemory,
        operations::{LinkedOutput, Operation, delete::Delete},
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
        test_module::{Todo, TodoHandler},
        track_sqlx_query::watch_sqlx_calls,
    };
    use sqlx::{Row, Sqlite};

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
                Delete {
                    base: TodoHandler,
                    wheres: ColumnEqual { col: "id", eq: Bind(2) },
                    links: (),
                },
                &mut pool,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"DELETE FROM "Todo" WHERE "id" = $1 RETURNING "id", "title", "done", "description";"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                output,
                vec![LinkedOutput {
                    id: 2,
                    attributes: Todo {
                        title: String::from("todo_2"),
                        done: true,
                        description: Some(String::from("description_2")),
                    },
                    links: ()
                }]
            );

            let check = sqlx::query("SELECT * FROM Todo;")
                .fetch_all(&mut pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get::<i64, _>("id"))
                .collect::<Vec<_>>();

            pretty_assertions::assert_eq!(check, vec![1, 3]);
        })
        .await;
    }
}
