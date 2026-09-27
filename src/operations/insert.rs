use crate::{
    collections::{Collection, CollectionId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, FromRowData},
    operations::{
        CollectionOutput, LinkedOutput, Operation, OperationOutput,
        insert_id_mode::AutoGenerate,
        operations_expressions_crossover::{
            ExpressionsForOperation, IdentifierOnly, OnInsert, SelfPrescribedInsert,
            TableExpressions,
        },
    },
    sqlx_query_builder::{
        Expression, Join, StatementBuilder,
        combinators::{Nest, OptionalExpression},
        statements::insert_statement::{InsertStatement, IteratorSpec, One},
    },
};

pub trait InsertLinkConsumeData {
    type Link: InsertOneLink;
    fn consume_data(
        self,
    ) -> (
        Self::Link,
        InsertLinkData<
            <Self::Link as InsertOneLink>::PreOpData,
            <Self::Link as InsertOneLink>::InsertValuesData,
            <Self::Link as InsertOneLink>::PostOpData,
        >,
    );
}

pub struct InsertLinkData<PreOpData, InsertValueData, PostOpData> {
    pub insert_value_data: InsertValueData,
    pub pre_op_data: PreOpData,
    pub post_op_data: PostOpData,
}

#[derive(Debug)]
pub struct ConstraintViolation(pub Option<String>);

pub trait InsertOneLink {
    type PreOp: OperationOutput;
    type PreOpData;
    fn pre_operation_init(&self, input: Self::PreOpData) -> Self::PreOp;

    fn pre_op_split(
        &self,
        pre_op_output: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        ConstraintViolation,
    >;

    type PreOpToInsertValue;
    type PreOpToTake;
    type PreOpToPostOp;

    type InsertReturning;
    fn insert_returning(&self) -> Self::InsertReturning;

    type InsertValuesData;
    type InsertSets;
    fn insert_value(
        &self,
        from_data: Self::InsertValuesData,
        pre_op_output: Self::PreOpToInsertValue,
    ) -> Self::InsertSets;

    type FromRow: FromRowData;
    fn from_row(&self) -> Self::FromRow;

    type TakeInput;
    type PostOp: OperationOutput;
    type PostOpData;
    fn from_row_result(
        &self,
        from_data: Self::PostOpData,
        from_row: <Self::FromRow as FromRowData>::RData,
        pre_op_to_post_op: Self::PreOpToPostOp,
    ) -> (Self::PostOp, Self::TakeInput);

    type PostOpOutput;
    fn post_op_output(
        &self,
        poo: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation>;

    type Output;
    fn take(
        self,
        post_op_output: Self::PostOpOutput,
        insert_items: Self::TakeInput,
        pre_op_to_post_op: Self::PreOpToTake,
    ) -> Self::Output;
}

impl InsertLinkConsumeData for () {
    type Link = ();
    fn consume_data(
        self,
    ) -> (
        Self::Link,
        InsertLinkData<
            <Self::Link as InsertOneLink>::PreOpData,
            <Self::Link as InsertOneLink>::InsertValuesData,
            <Self::Link as InsertOneLink>::PostOpData,
        >,
    ) {
        (
            (),
            InsertLinkData {
                insert_value_data: (),
                pre_op_data: (),
                post_op_data: (),
            },
        )
    }
}

impl InsertOneLink for () {
    type PreOp = ();

    type PreOpData = ();

    fn pre_operation_init(&self, _: Self::PreOpData) -> Self::PreOp {}

    fn pre_op_split(
        &self,
        _: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        ConstraintViolation,
    > {
        Ok(((), (), ()))
    }

    type PreOpToInsertValue = ();
    type PreOpToTake = ();
    type PreOpToPostOp = ();

    type InsertReturning = ();

    fn insert_returning(&self) -> Self::InsertReturning {}

    type InsertValuesData = ();

    type InsertSets = ();

    fn insert_value(
        &self,
        _: Self::InsertValuesData,
        _: <Self::PreOp as OperationOutput>::Output,
    ) -> Self::InsertSets {
    }

    type FromRow = ();

    fn from_row(&self) -> Self::FromRow {}

    type TakeInput = ();

    type PostOp = ();

    type PostOpData = ();

    fn from_row_result(
        &self,
        _: Self::PostOpData,
        _: <Self::FromRow as FromRowData>::RData,
        _: Self::PreOpToTake,
    ) -> (Self::PostOp, Self::TakeInput) {
        ((), ())
    }

    type PostOpOutput = ();
    fn post_op_output(
        &self,
        _: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        Ok(())
    }

    type Output = ();

    fn take(self, _: Self::PostOpOutput, _: Self::TakeInput, _: Self::PreOpToTake) -> Self::Output {
    }
}

pub struct InsertOne<Handler, Data, Infalibility> {
    pub handler: Handler,
    pub data: Data,
    pub infalibility: Infalibility,
}

pub struct AbortOperation;
pub struct DismissFailure;

pub struct InsertEntity<Attributes, Link> {
    pub attributes: Attributes,
    pub link: Link,
}

impl<H, PreL, L> OperationOutput for InsertOne<H, InsertEntity<H::InputData, PreL>, AbortOperation>
where
    PreL: InsertLinkConsumeData<Link = L>,
    L: InsertOneLink,
    H: Collection,
{
    type Output = LinkedOutput<<H::Id as CollectionId>::IdData, H::OutputData, L::Output>;
}

// one item, with links
impl<S, Base, LinkPreSplit, Link> Operation<S>
    for InsertOne<Base, InsertEntity<Base::InputData, LinkPreSplit>, AbortOperation>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    // Base::Id: OnInsert<Id, InsertId:Send+Clone+ for<'q> Expression<'q, S>,InsertExpression : for<'q> Expression<'q, S>>,
    // Id: Send,
    LinkPreSplit: Send + InsertLinkConsumeData<Link = Link>,
    Link: Send,
    Link: InsertOneLink,
    Base: TableExpressions<
            PascalCase: for<'q> Expression<'q, S>,
            Identifier: OptionalExpression,
        >,
    for<'q> Join<Base::Identifier>: Expression<'q, S>,
    Base: OnInsert<Base::InputData, InsertExpression: OptionalExpression>,
    for<'q> Join<<Base as OnInsert<Base::InputData>>::InsertExpression>: Expression<'q, S>,
    Base: Collection,
    Base: Collection<InputData: Send, OutputData: Send, Id: Send + CollectionId<IdData: Send>>,
    Base: Send,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::Id: ExpressionsForOperation<Identifier: OptionalExpression>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::Identifier>: Expression<'q, S>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
    Link::PreOp: Operation<S, Output: Send>,
    Link::PreOpData: Send,
    Link::InsertSets: SelfPrescribedInsert<
            InsertId: Send + OptionalExpression,
            InsertValue: Send + OptionalExpression,
        >,
    for<'q> Join<<Link::InsertSets as SelfPrescribedInsert>::InsertId>: Expression<'q, S>,
    for<'q> Join<<Link::InsertSets as SelfPrescribedInsert>::InsertValue>: Expression<'q, S>,
    Link::InsertValuesData: Send,
    Link::PostOpData: Send,
    Link::InsertReturning: IdentifierOnly<Identifier: OptionalExpression>,
    for<'q> Join<<Link::InsertReturning as IdentifierOnly>::Identifier>: Expression<'q, S>,
    Link::FromRow: for<'r> FromRowAlias<'r, S::Row, RData: Send>,
    Link::TakeInput: Send,
    Link::PostOp: Operation<S, Output: Send>,
    Link::Output: Send,
    Link::PreOpToInsertValue: Send,
    Link::PreOpToTake: Send,
    Link::PreOpToPostOp: Send,
{
    fn exec_operation(self, pool: &mut S::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let (link, link_data) = self.data.link.consume_data();
            let pre_op = link
                .pre_operation_init(link_data.pre_op_data)
                .exec_operation(&mut *pool)
                .await;

            let (pre_op_to_insert_value, pre_op_to_take, pre_op_to_post_op) =
                link.pre_op_split(pre_op).expect("constraint violation");

            let base_id = self.handler.id();

            // let (id_insert_id, id_insert_val) = base_id.on_insert_with_id(self.id);

            let insert_sets = link
                .insert_value(link_data.insert_value_data, pre_op_to_insert_value)
                .on_insert();

            let (stmt, arg) = StatementBuilder::<'_, S>::new(InsertStatement {
                table_name: self.handler.table_name_pascal_case(),
                identifiers: Join {
                    start: "",
                    separator: ", ",
                    items: (Nest(self.handler.identifier()), Nest(insert_sets.0)),
                },
                values: One(Join {
                    start: "",
                    separator: ", ",
                    items: (
                        Nest(self.handler.on_insert(self.data.attributes)),
                        Nest(insert_sets.1),
                    ),
                }),
                returning: (
                    Nest(base_id.identifier()),
                    Nest(self.handler.identifier()),
                    Nest(link.insert_returning().identifier_only()),
                ),
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
            // .map_err(|e| {
            //     if let Some(e) = e.as_database_error() {
            //         if e.is_check_violation() || e.is_unique_violation() || e.is_foreign_key_violation() {
            //             return ConstraintViolation(e.constraint().map(|c| c.to_string()));
            //         }
            //     }
            //         tracing::error!(sqlx_error = ?e, "bug: must clear all sqlx errors, hard to know where this error was originated!");
            //         panic!()
            // })?
            .unwrap();

            let id = base_id.no_alias(&row).unwrap();
            let attributes = self.handler.no_alias(&row).unwrap();

            let links = {
                let ii = link.from_row().no_alias(&row).unwrap();
                let (post_op_input_2, from_row_take_input) =
                    link.from_row_result(link_data.post_op_data, ii, pre_op_to_post_op);
                let po = post_op_input_2.exec_operation(&mut *pool).await;
                let po = link.post_op_output(po).expect("constraint violation");
                link.take(po, from_row_take_input, pre_op_to_take)
            };

            LinkedOutput {
                id,
                attributes,
                links,
            }
        }
    }
}

impl<Base, I> OperationOutput for InsertOne<Base, IteratorSpec<I>, AbortOperation>
where
    Base: Collection,
    I: IntoIterator<Item = Base::InputData>,
{
    type Output = Vec<CollectionOutput<<Base::Id as CollectionId>::IdData, Base::OutputData>>;
}

// many items, no links
impl<S, Base, I> Operation<S> for InsertOne<Base, IteratorSpec<I>, AbortOperation>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    I: Send + IntoIterator<Item = Base::InputData>,
    Base::Id: OnInsert<
            AutoGenerate,
            InsertId: Send + Clone + OptionalExpression,
            InsertExpression: OptionalExpression,
        >,
    for<'q> Join<<Base::Id as OnInsert<AutoGenerate>>::InsertId>: Expression<'q, S>,
    for<'q> Join<<Base::Id as OnInsert<AutoGenerate>>::InsertExpression>: Expression<'q, S>,
    Base: TableExpressions<
            PascalCase: for<'q> Expression<'q, S>,
            Identifier: OptionalExpression,
        >,
    for<'q> Join<Base::Identifier>: Expression<'q, S>,
    Base::InputData: for<'q> Expression<'q, S>,
    Base: Sync,
    Base: OnInsert<IteratorSpec<I>>,
    Base: Collection,
    Base:
        Collection<InputData: Send, OutputData: Send, Id: Send + CollectionId<IdData: Send> + Sync>,
    Base: Send,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::Id: ExpressionsForOperation<Identifier: OptionalExpression>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::Identifier>: Expression<'q, S>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
{
    fn exec_operation(self, pool: &mut S::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let base_id = self.handler.id();

            let id_insert_id = base_id.identifier();

            let (stmt, arg) = StatementBuilder::<'_, S>::new(InsertStatement {
                table_name: self.handler.table_name_pascal_case(),
                identifiers: Join {
                    start: "",
                    separator: ", ",
                    items: (Nest(self.handler.identifier()),),
                },
                values: self.data,
                returning: (Nest(id_insert_id), Nest(self.handler.identifier())),
            })
            .unwrap();

            let row = S::fetch_all_mapped(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: arg,
                },
                |row| {
                    let id = base_id.no_alias(&row).unwrap();
                    let attributes = self.handler.no_alias(&row).unwrap();
                    CollectionOutput { id, attributes }
                },
            )
            .await
            // .map_err(|e| {
            //     if let Some(e) = e.as_database_error() {
            //         if e.is_check_violation() || e.is_unique_violation() || e.is_foreign_key_violation() {
            //             return ConstraintViolation(e.constraint().map(|c| c.to_string()));
            //         }
            //     }
            //         tracing::error!(sqlx_error = ?e, "bug: must clear all sqlx errors, hard to know where this error was originated!");
            //         panic!()
            // })
            .unwrap();

            row
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        connect_in_memory::ConnectInMemory, operations::{
            CollectionOutput, LinkedOutput, Operation, insert::{AbortOperation, InsertEntity, InsertOne},
        }, sqlx_query_builder::statements::insert_statement::IteratorSpec, test_module::{Todo, TodoHandler}, track_sqlx_query::watch_sqlx_calls,
    };
    use sqlx::{Sqlite, query};

    #[tokio::test(flavor = "current_thread")]
    async fn insert_one_test() {
        watch_sqlx_calls(async |actions| {
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
            actions.clear();

            let output = Operation::<Sqlite>::exec_operation(
                InsertOne {
                    data: InsertEntity {
                        attributes: Todo { title: String::from("todo"), done: false, description: None }, 
                        link: () 
                    },
                    handler: TodoHandler,
                    infalibility: AbortOperation,
                },
                &mut conn,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"INSERT INTO "Todo" ("title", "done", "description") VALUES ($1, $2, $3) RETURNING "id", "title", "done", "description";"#
                        .to_string(),
                ]
            );

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
        })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn insert_many_test() {
        watch_sqlx_calls(async |actions| {
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
            actions.clear();


            let output = Operation::<Sqlite>::exec_operation(
                InsertOne {
                    handler: TodoHandler,
                    data: IteratorSpec([
                        Todo {
                            title: String::from("todo"),
                            done: false,
                            description: None,
                        },
                        Todo {
                            title: String::from("todo2"),
                            done: true,
                            description: Some(String::from("description")),
                        },
                    ]),
                    infalibility: AbortOperation,
                },
                &mut conn,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"INSERT INTO "Todo" ("title", "done", "description") VALUES ($1, $2, $3), ($4, $5, $6) RETURNING "id", "title", "done", "description";"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                output,
                vec![
                    CollectionOutput {
                        id: 1,
                        attributes: Todo { title: String::from("todo"), done: false, description: None },
                    },
                    CollectionOutput {
                        id: 2,
                        attributes: Todo { title: String::from("todo2"), done: true, description: Some(String::from("description")) },
                    }
                ]
            );
        })
        .await;
    }
}
