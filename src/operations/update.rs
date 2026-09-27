use crate::{
    collections::{Collection, CollectionId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, FromRowData},
    operations::{
        LinkedOutput, Operation, OperationOutput,
        insert::{AbortOperation, ConstraintViolation},
        operations_expressions_crossover::{
            ExpressionsForOperation, OnUpdate, SelfPrescribedInsert, TableExpressions,
        },
    },
    sqlx_query_builder::{
        Expression, Join, StatementBuilder,
        combinators::{Nest, OptionalExpression},
        statements::update_statement::UpdateStatement,
    },
};

pub struct Update<Base, Partial, Wheres, Links, Infalibility> {
    pub base: Base,
    pub partial: Partial,
    pub wheres: Wheres,
    pub links: Links,
    pub infalibility: Infalibility,
}

pub trait UpdateLinkSplit {
    type Link: UpdateLink;
    fn init_split(
        self,
    ) -> (
        Self::Link,
        UpdateLinkData<
            <Self::Link as UpdateLink>::InitSplitForWheres,
            <Self::Link as UpdateLink>::InitSplitForUpdateValues,
            <Self::Link as UpdateLink>::InitSplitForPreOp,
            <Self::Link as UpdateLink>::InitSplitPostOp,
        >,
    );
}

pub struct UpdateLinkData<Wheres, UpdateValues, PreOp, PostOp> {
    pub wheres: Wheres,
    pub update_values: UpdateValues,
    pub pre_op: PreOp,
    pub post_op: PostOp,
}

pub trait UpdateLink {
    type InitSplitForPreOp;

    type PreOpSplitWheres;
    type PreOpSplitValues;
    type PreOpSplitPostOp;
    type PreOpSplitTake;
    type PreOp: OperationOutput;
    fn pre_op(&self, init_split_for_pre_op: Self::InitSplitForPreOp) -> Self::PreOp;
    fn split_pre_op(
        &self,
        pre_op: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpSplitWheres,
            Self::PreOpSplitValues,
            Self::PreOpSplitPostOp,
            Self::PreOpSplitTake,
        ),
        ConstraintViolation,
    >;

    type InitSplitForWheres;

    type UpdateWhere;
    fn wheres(&self, wheres: Self::InitSplitForWheres) -> Self::UpdateWhere;

    type UpdateReturning;
    fn update_names(&self) -> Self::UpdateReturning;

    type InitSplitForUpdateValues;
    type UpdateSets;
    fn update_values(
        &self,
        values: Self::InitSplitForUpdateValues,
        pre_op_output: Self::PreOpSplitValues,
    ) -> Self::UpdateSets;

    type FromRow: FromRowData;
    fn from_row(&self) -> Self::FromRow;

    type PostOp: OperationOutput;
    type InitSplitPostOp;
    fn post_op(
        &self,
        from_init_split: Self::InitSplitPostOp,
        from_pre_op: Self::PreOpSplitPostOp,
    ) -> Self::PostOp;

    fn from_row_result(
        &self,
        row_data: &<Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOp,
    );

    type Output;
    type PostOpOutput;
    fn post_op_output(
        &self,
        poo: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation>;

    fn take(
        &self,
        from_row: <Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOpOutput,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output;
}

impl UpdateLinkSplit for () {
    type Link = ();
    fn init_split(self) -> (Self::Link, UpdateLinkData<(), (), (), ()>) {
        (
            (),
            UpdateLinkData {
                wheres: (),
                update_values: (),
                pre_op: (),
                post_op: (),
            },
        )
    }
}

impl UpdateLink for () {
    type InitSplitForWheres = ();
    type UpdateWhere = ();
    type PreOp = ();
    fn pre_op(&self, _: Self::InitSplitForPreOp) -> Self::PreOp {}
    fn wheres(&self, _: Self::InitSplitForWheres) -> Self::UpdateWhere {}

    type InitSplitForPreOp = ();
    type PreOpSplitWheres = ();
    type PreOpSplitValues = ();
    type PreOpSplitPostOp = ();
    type PreOpSplitTake = ();
    fn split_pre_op(
        &self,
        _: (),
    ) -> Result<
        (
            Self::PreOpSplitWheres,
            Self::PreOpSplitValues,
            Self::PreOpSplitPostOp,
            Self::PreOpSplitTake,
        ),
        ConstraintViolation,
    > {
        Ok(((), (), (), ()))
    }

    type UpdateReturning = ();
    fn update_names(&self) -> Self::UpdateReturning {}

    type InitSplitForUpdateValues = ();
    type UpdateSets = ();
    fn update_values(
        &self,
        _: Self::InitSplitForUpdateValues,
        _: Self::PreOpSplitValues,
    ) -> Self::UpdateSets {
    }

    type FromRow = ();
    fn from_row(&self) -> Self::FromRow {}

    type PostOp = ();
    type InitSplitPostOp = ();
    fn post_op(&self, _: Self::InitSplitPostOp, _: Self::PreOpSplitPostOp) -> Self::PostOp {}
    fn from_row_result(&self, _: &<Self::FromRow as FromRowData>::RData, _: &mut Self::PostOp) {}

    type PostOpOutput = ();
    fn post_op_output(
        &self,
        _: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        Ok(())
    }

    type Output = ();
    fn take(
        &self,
        _: <Self::FromRow as FromRowData>::RData,
        _: &mut <Self::PostOp as OperationOutput>::Output,
        _: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
    }
}

impl<Handler, Partial, Wheres, PL, Links> OperationOutput
    for Update<Handler, Partial, Wheres, PL, AbortOperation>
where
    Handler: Collection,
    PL: UpdateLinkSplit<Link = Links>,
    Links: UpdateLink,
{
    type Output = Vec<
        LinkedOutput<<Handler::Id as CollectionId>::IdData, Handler::OutputData, Links::Output>,
    >;
}

impl<S, Base, Partial, Wheres, PreSplitLink, Links> Operation<S>
    for Update<Base, Partial, Wheres, PreSplitLink, AbortOperation>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    Base: Clone,
    Base: Send,
    Base: TableExpressions<
            Identifier: Send + OptionalExpression,
            PascalCase: for<'q> Expression<'q, S>,
        >,
    for<'q> Join<Base::Identifier>: Expression<'q, S>,
    Base: OnUpdate<Partial, UpdateExpression: Send + OptionalExpression>,
    for<'q> Join<<Base as OnUpdate<Partial>>::UpdateExpression>: Expression<'q, S>,
    Base::Id: ExpressionsForOperation<Identifier: Send + OptionalExpression>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::Identifier>: Expression<'q, S>,
    // Base: Identifier<Identifier: Send + for<'q> Expression<'q, S>>,
    // Base: TableNameExpression<TableNameExpression: for<'q> Expression<'q, S>>,
    Base: Collection<OutputData: Send>,
    // Base:
    //     V0OnUpdate<UpdateInput = Partial, UpdateExpression: Send + for<'q> Expression<'q, S>>,
    Base: for<'r> FromRowAlias<'r, S::Row, RData = Base::OutputData>,
    Base::Id: Send + CollectionId<IdData: Send>,
    // Base::Id: Identifier<Identifier: Send + for<'q> Expression<'q, S>>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row, RData = <Base::Id as CollectionId>::IdData>,
    Partial: Send,
    Wheres: Send + OptionalExpression,
    for<'q> Join<Wheres>: Expression<'q, S>,
    PreSplitLink: Send + UpdateLinkSplit<Link = Links>,
    Links: Send + UpdateLink,
    Links::InitSplitForWheres: Send,
    Links::UpdateWhere: OptionalExpression,
    for<'q> Join<Links::UpdateWhere>: Expression<'q, S>,
    Links::InitSplitForUpdateValues: Send,
    // Links::UpdateSets: Send + for<'q> Expression<'q, S>,
    Links::UpdateSets: Send
        + SelfPrescribedInsert<UpdateSets: Send + OptionalExpression>,
    for<'q> Join<<Links::UpdateSets as SelfPrescribedInsert>::UpdateSets>: Expression<'q, S>,
    Links::UpdateReturning: Send + OptionalExpression,
    for<'q> Join<Links::UpdateReturning>: Expression<'q, S>,
    Links::FromRow: Send + for<'r> FromRowAlias<'r, S::Row, RData: Send>,
    Links::InitSplitPostOp: Send,
    Links::InitSplitForPreOp: Send,
    Links::PreOp: Send + Operation<S>,
    Links::PreOpSplitWheres: Send + OptionalExpression,
    for<'q> Join<Links::PreOpSplitWheres>: Expression<'q, S>,
    // Links::PreOpSplitValues: Send + for<'q> Expression<'q, S>,
    Links::PostOp: Send + Operation<S>,
    Links::Output: Send,
    Links::PreOpSplitTake: Send,
{
    fn exec_operation(self, pool: &mut <S>::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: sqlx::Database,
        Self: Sized,
    {
        async move {
            let (self_link, self_link_data) = self.links.init_split();
            let id = self.base.id();

            let pre_op = self_link
                .pre_op(self_link_data.pre_op)
                .exec_operation(&mut *pool)
                .await;

            let (pre_op_wheres, pre_op_values, pre_op_split_for_post_op, mut pre_op_split_take) =
                self_link.split_pre_op(pre_op).expect("constraint violation");

            let base_values = self.base.clone().on_update(self.partial);
            let link_values = self_link
                .update_values(self_link_data.update_values, pre_op_values)
                .on_update();

            // Check if values are operational
            if !base_values.is_oper() && !link_values.is_oper() {
                panic!(
                    "bug: update operation is not operational, the bug should be catched before using Update"
                );
            }

            let (stmt, args) = StatementBuilder::<S>::new(UpdateStatement {
                table_name: self.base.table_name_pascal_case(),
                wheres: (
                    Nest(self.wheres),
                    Nest(self_link.wheres(self_link_data.wheres)),
                    Nest(pre_op_wheres),
                ),
                returning: (
                    Nest(id.identifier()),
                    Nest(self.base.identifier()),
                    Nest(self_link.update_names()),
                ),
                values: (Nest(base_values), Nest(link_values)),
            })
            .unwrap();

            let link_from_row = self_link.from_row();
            let mut from_row_data = vec![];
            let mut post_op = self_link.post_op(self_link_data.post_op, pre_op_split_for_post_op);

            let res = S::fetch_all(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: args,
                },
            )
            .await
            .unwrap()
            // .map_err(|e| {
            //     if let Some(db) = e.as_database_error() {
            //         if db.is_check_violation() || db.is_unique_violation() || db.is_foreign_key_violation() {
            //             return ConstraintViolation(db.constraint().map(|c| c.to_string()));
            //         }
            //     }
            //     tracing::error!(sqlx_error = ?e, "bug: must clear all sqlx errors, hard to know where this error was originated!");
            //     panic!()
            // })?
            .into_iter()
            .map(|e| {
                let id = self.base.id().no_alias(&e).unwrap();
                let attributes = self.base.no_alias(&e).unwrap();
                let link_r = link_from_row.no_alias(&e).unwrap();
                from_row_data.push(link_r);

                LinkedOutput {
                    id,
                    attributes,
                    links: (),
                }
            })
            .collect::<Vec<_>>();

            from_row_data.iter().for_each(|e| {
                self_link.from_row_result(e, &mut post_op);
            });

            let poo = post_op.exec_operation(pool).await;
            let mut poo = self_link.post_op_output(poo).expect("constraint violation");

            res.into_iter()
                .zip(from_row_data.into_iter())
                .map(|(e, f)| LinkedOutput {
                    id: e.id,
                    attributes: e.attributes,
                    links: self_link.take(f, &mut poo, &mut pre_op_split_take),
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        collections::Collection,
        connect_in_memory::ConnectInMemory,
        from_row::FromRowAlias,
        operations::{
            CollectionOutput, LinkedOutput, Operation, insert::AbortOperation,
            update::Update as UpdateOp,
        },
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual, ScopedColumn},
        test_module::{Todo, TodoHandler, TodoPartial},
        track_sqlx_query::watch_sqlx_calls,
        update_mod::Update,
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
                UpdateOp {
                    base: TodoHandler,
                    wheres: ColumnEqual {
                        col: ScopedColumn {
                            table: "Todo",
                            col: "id",
                        },
                        eq: Bind(2),
                    },
                    partial: TodoPartial {
                        title: Update::Set("new_title".to_string()),
                        done: Update::Keep,
                        description: Update::Keep,
                    },
                    links: (),
                    infalibility: AbortOperation,
                },
                &mut pool,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"UPDATE "Todo" SET title = $1 WHERE "Todo"."id" = $2 RETURNING "id", "title", "done", "description";"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                output,
                vec![LinkedOutput {
                    id: 2,
                    attributes: Todo {
                        title: "new_title".to_string(),
                        done: true,
                        description: Some("description_2".to_string()),
                    },
                    links: ()
                }]
            );

            let check = sqlx::query("SELECT * FROM Todo;")
                .fetch_all(&mut pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| CollectionOutput {
                    id: TodoHandler.id().no_alias(&row).unwrap(),
                    attributes: TodoHandler.no_alias(&row).unwrap(),
                })
                .collect::<Vec<_>>();

            pretty_assertions::assert_eq!(
                check,
                vec![
                    CollectionOutput {
                        id: 1,
                        attributes: Todo {
                            title: "todo_1".to_string(),
                            done: false,
                            description: Some("description_1".to_string()),
                        },
                    },
                    CollectionOutput {
                        id: 2,
                        attributes: Todo {
                            title: "new_title".to_string(),
                            done: true,
                            description: Some("description_2".to_string()),
                        },
                    },
                    CollectionOutput {
                        id: 3,
                        attributes: Todo {
                            title: "todo_3".to_string(),
                            done: false,
                            description: Some("description_3".to_string()),
                        },
                    },
                ]
            );
        })
        .await;
    }
}
