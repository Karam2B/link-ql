pub struct FetchJunction<const RETURN_FROM_ID: bool, const INVERSE: bool, Key, From, To, Wheres> {
    pub key: Key,
    pub from: From,
    pub to: To,
    pub wheres: Wheres,
}

mod not_returning_from_id {
    use super::FetchJunction;
    use crate::links::utils::conventional_junction_table_name;
    use crate::operations::operations_expressions_crossover::{
        ExpressionsForOperation, TableExpressions,
    };
    use crate::sqlx_query_builder::basic_expressions::{AliasedScopedColumn, JoinExpression};
    use crate::sqlx_query_builder::sanitize_combinator::Sanitize;
    use crate::sqlx_query_builder::{
        Expression, RefExpression, StatementBuilder,
        combinators::{Join, Nest, OptionalExpression},
        statements::select_statement::SelectStatement,
    };
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        from_row::{FromRowAlias, RowStrAliased},
        operations::{Operation, OperationOutput},
    };
    use sqlx::{ColumnIndex, Decode, Encode, Row, Type};

    impl<const INVERSE: bool, Key, From, To, Wheres> OperationOutput
        for FetchJunction<false, INVERSE, Key, From, To, Wheres>
    where
        To: Collection<Id: SingleColumnId>,
        From: Collection<Id: SingleColumnId>,
    {
        type Output = Vec<(<To::Id as CollectionId>::IdData, To::OutputData)>;
    }
    impl<const INVERSE: bool, S, Key, From, To, Wheres> Operation<S>
        for FetchJunction<false, INVERSE, Key, From, To, Wheres>
    where
        S: DatabaseExt,
        S: ExecutorTrait,
        Key: Send
            + Clone
            + AsRef<str>
            + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
            + 'static,
        From: Send + Clone + 'static + Collection<Id: SingleColumnId + Send>,
        From: TableExpressions<
                PascalCase: Send + Clone + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        From::Id: Send + CollectionId<IdData: Send>,
        To: Send
            + Clone
            + 'static
            + Collection<Id: SingleColumnId + Send, OutputData: Send>
            + for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
        To: TableExpressions<
                ScopedAliased: Send + OptionalExpression,
                PascalCase: Send
                                + Clone
                                + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
                                + for<'q> Expression<'q, S>
                                + for<'a> RefExpression<'a, S>,
                SnakeCase: crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        for<'q> Join<To::ScopedAliased>: Expression<'q, S>,
        To::Id: Send
            + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>
            + for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
        To::Id: ExpressionsForOperation<
                Identifier: Send + for<'q> Expression<'q, S>,
                ScopedAliased: Send + OptionalExpression,
            >,
        Wheres: Send + for<'q> Expression<'q, S>,
        i64: for<'q> Encode<'q, S> + Type<S>,
        for<'a> &'a str: ColumnIndex<S::Row>,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let (stmt, args) = if INVERSE {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(SelectStatement {
                        from: t.clone(),
                        select_items: (
                            Nest(AliasedScopedColumn {
                                table: t.clone(),
                                column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                                alias: "to_id",
                            }),
                            Nest(self.to.scoped_aliased("t_")),
                        ),
                        joins: (JoinExpression {
                            join_type: "INNER JOIN",
                            foreign_table: self.to.table_name_pascal_case(),
                            foreign_column: self.to.id().identifier(),
                            local_table: t,
                            local_column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        },),
                        wheres: (self.wheres,),
                        group_by: (),
                        order: (),
                        limit: (),
                    })
                    .unwrap()
                } else {
                    let junction = Sanitize(conventional_junction_table_name(
                        self.key.clone(),
                        self.from.clone(),
                        self.to.clone(),
                    ));
                    let junction_for_join = Sanitize(conventional_junction_table_name(
                        self.key.clone(),
                        self.from.clone(),
                        self.to.clone(),
                    ));
                    StatementBuilder::<S>::new(SelectStatement {
                        from: junction,
                        select_items: (
                            Nest(AliasedScopedColumn {
                                table: junction_for_join,
                                column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                                alias: "to_id",
                            }),
                            Nest(self.to.scoped_aliased("t_")),
                        ),
                        joins: (JoinExpression {
                            join_type: "INNER JOIN",
                            foreign_table: self.to.table_name_pascal_case(),
                            foreign_column: self.to.id().identifier(),
                            local_table: Sanitize(conventional_junction_table_name(
                                self.key.clone(),
                                self.from.clone(),
                                self.to.clone(),
                            )),
                            local_column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        },),
                        wheres: (self.wheres,),
                        group_by: (),
                        order: (),
                        limit: (),
                    })
                    .unwrap()
                };

                let rows = S::fetch_all(
                    &mut *pool,
                    Executable {
                        string: &stmt,
                        arguments: args,
                    },
                )
                .await
                .unwrap();

                rows.into_iter()
                    .map(|row| {
                        let to_id: <To::Id as CollectionId>::IdData = row.get("to_id");
                        let to_data: To::OutputData =
                            self.to.str_alias(RowStrAliased::new(&row, "t_")).unwrap();
                        (to_id, to_data)
                    })
                    .collect()
            }
        }
    }
    #[cfg(test)]
    mod test {
        use crate::connect_in_memory::ConnectInMemory;
        use crate::links::DefaultRelationKey;
        use crate::operations::Operation;
        use crate::sqlx_query_builder::{
            Bind, basic_expressions::ColumnEqual, sanitize_combinator::Sanitize,
        };
        use crate::test_module::Tag;
        use crate::test_module::{TagHandler, TodoHandler};

        use super::super::seed_todo_tag_junction;
        use super::FetchJunction;
        use sqlx::Sqlite;

        #[tokio::test(flavor = "current_thread")]
        async fn fetch_junction_without_return_from_id() {
            let mut conn = Sqlite::in_memory_connection().await;
            seed_todo_tag_junction(&mut conn).await;

            let result = Operation::<Sqlite>::exec_operation(
                FetchJunction::<false, true, DefaultRelationKey, TodoHandler, TagHandler, _> {
                    key: DefaultRelationKey,
                    from: TodoHandler,
                    to: TagHandler,
                    wheres: ColumnEqual {
                        col: Sanitize(("todo_id",)),
                        eq: Bind(4i64),
                    },
                },
                &mut conn,
            )
            .await;

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].0, 1);
            assert_eq!(
                result[0].1,
                Tag {
                    title: "tag_1".into()
                }
            );
        }
    }
}
mod returning_from_id {
    use super::FetchJunction;
    use crate::collections::{Collection, CollectionId, SingleColumnId};
    use crate::database_extention::DatabaseExt;
    use crate::execute::Executable;
    use crate::fix_executor::ExecutorTrait;
    use crate::from_row::{FromRowAlias, RowStrAliased};
    use crate::links::utils::conventional_junction_table_name;
    use crate::operations::operations_expressions_crossover::ExpressionsForOperation;
    use crate::operations::operations_expressions_crossover::TableExpressions;
    use crate::operations::{CollectionOutput, Operation, OperationOutput};
    use crate::sqlx_query_builder::{
        Expression, RefExpression, StatementBuilder,
        basic_expressions::{AliasedScopedColumn, JoinExpression},
        combinators::{Join, Nest, OptionalExpression},
        sanitize_combinator::Sanitize,
        statements::select_statement::SelectStatement,
    };
    use sqlx::{ColumnIndex, Decode, Encode, Row, Type};

    impl<const INVERSE: bool, Key, From, To, Wheres> OperationOutput
        for FetchJunction<true, INVERSE, Key, From, To, Wheres>
    where
        To: Collection<Id: SingleColumnId>,
        From: Collection<Id: SingleColumnId>,
    {
        type Output = Vec<(
            <From::Id as CollectionId>::IdData,
            CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>,
        )>;
    }

    impl<const INVERSE: bool, S, Key, From, To, Wheres> Operation<S>
        for FetchJunction<true, INVERSE, Key, From, To, Wheres>
    where
        S: DatabaseExt,
        S: ExecutorTrait,
        Key: Send
            + Clone
            + AsRef<str>
            + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
            + 'static,
        From: Send + Clone + 'static + Collection<Id: SingleColumnId + Send>,
        From: TableExpressions<
                PascalCase: Send + Clone + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        From::Id: Send + CollectionId<IdData: Send>,
        To: Send
            + Clone
            + 'static
            + Collection<Id: SingleColumnId + Send, OutputData: Send>
            + for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
        To: TableExpressions<
                ScopedAliased: Send + OptionalExpression,
                PascalCase: Send
                                + Clone
                                + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
                                + for<'q> Expression<'q, S>
                                + for<'a> RefExpression<'a, S>,
                SnakeCase: crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        for<'q> Join<To::ScopedAliased>: Expression<'q, S>,
        To::Id: Send
            + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>
            + for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
        To::Id: ExpressionsForOperation<
                Identifier: Send + for<'q> Expression<'q, S>,
                ScopedAliased: Send + OptionalExpression,
            >,
        From::Id: CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>,
        Wheres: Send + for<'q> Expression<'q, S>,
        i64: for<'q> Encode<'q, S> + Type<S>,
        for<'a> &'a str: ColumnIndex<S::Row>,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let (stmt, args) = if INVERSE {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(SelectStatement {
                        from: t.clone(),
                        select_items: (
                            Nest(AliasedScopedColumn {
                                table: t.clone(),
                                column: Sanitize(((self.from.table_name_snake_case(),), "_id")),
                                alias: "from_id",
                            }),
                            Nest(AliasedScopedColumn {
                                table: t.clone(),
                                column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                                alias: "to_id",
                            }),
                            Nest(self.to.scoped_aliased("t_")),
                        ),
                        joins: (JoinExpression {
                            join_type: "INNER JOIN",
                            foreign_table: self.to.table_name_pascal_case(),
                            foreign_column: self.to.id().identifier(),
                            local_table: t,
                            local_column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        },),
                        wheres: (self.wheres,),
                        group_by: (),
                        order: (),
                        limit: (),
                    })
                    .unwrap()
                } else {
                    let junction = Sanitize(conventional_junction_table_name(
                        self.key.clone(),
                        self.from.clone(),
                        self.to.clone(),
                    ));
                    let junction_for_cols = Sanitize(conventional_junction_table_name(
                        self.key.clone(),
                        self.from.clone(),
                        self.to.clone(),
                    ));
                    StatementBuilder::<S>::new(SelectStatement {
                        from: junction,
                        select_items: (
                            Nest(AliasedScopedColumn {
                                table: junction_for_cols,
                                column: Sanitize(((self.from.table_name_snake_case(),), "_id")),
                                alias: "from_id",
                            }),
                            Nest(AliasedScopedColumn {
                                table: Sanitize(conventional_junction_table_name(
                                    self.key.clone(),
                                    self.from.clone(),
                                    self.to.clone(),
                                )),
                                column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                                alias: "to_id",
                            }),
                            Nest(self.to.scoped_aliased("t_")),
                        ),
                        joins: (JoinExpression {
                            join_type: "INNER JOIN",
                            foreign_table: self.to.table_name_pascal_case(),
                            foreign_column: self.to.id().identifier(),
                            local_table: Sanitize(conventional_junction_table_name(
                                self.key.clone(),
                                self.from.clone(),
                                self.to.clone(),
                            )),
                            local_column: Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        },),
                        wheres: (self.wheres,),
                        group_by: (),
                        order: (),
                        limit: (),
                    })
                    .unwrap()
                };

                let rows = S::fetch_all(
                    &mut *pool,
                    Executable {
                        string: &stmt,
                        arguments: args,
                    },
                )
                .await
                .unwrap();

                rows.into_iter()
                    .map(|row| {
                        let to_id: <To::Id as CollectionId>::IdData = row.get("to_id");
                        let from_id: <From::Id as CollectionId>::IdData = row.get("from_id");
                        let to_data: To::OutputData =
                            self.to.str_alias(RowStrAliased::new(&row, "t_")).unwrap();
                        (
                            from_id,
                            CollectionOutput {
                                id: to_id,
                                attributes: to_data,
                            },
                        )
                    })
                    .collect()
            }
        }
    }
    #[cfg(test)]
    mod test {
        use crate::links::junction_table_op::seed_todo_tag_junction;
        use crate::sqlx_query_builder::{Bind, basic_expressions::ColumnIn};
        use crate::test_module::{Tag, TagHandler, TodoHandler};
        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{DefaultRelationKey, junction_table_op::FetchJunction},
            operations::{CollectionOutput, Operation},
        };
        use sqlx::Sqlite;
        #[tokio::test(flavor = "current_thread")]
        async fn fetch_junction_inverse_with_return_from_id() {
            let mut conn = Sqlite::in_memory_connection().await;
            seed_todo_tag_junction(&mut conn).await;

            let result = Operation::<Sqlite>::exec_operation(
                FetchJunction::<true, true, DefaultRelationKey, TodoHandler, TagHandler, _> {
                    key: DefaultRelationKey,
                    from: TodoHandler,
                    to: TagHandler,
                    wheres: ColumnIn {
                        col: "todo_id",
                        values: vec![Bind(5i64), Bind(7i64)],
                    },
                },
                &mut conn,
            )
            .await;

            assert_eq!(
                result,
                vec![
                    (
                        5,
                        CollectionOutput {
                            id: 1,
                            attributes: Tag { title: "tag_1".to_string() },
                        },
                    ),
                    (
                        5,
                        CollectionOutput {
                            id: 2,
                            attributes: Tag { title: "tag_2".to_string() },
                        },
                    ),
                    (
                        5,
                        CollectionOutput {
                            id: 3,
                            attributes: Tag { title: "tag_3".to_string() },
                        },
                    ),
                    (
                        7,
                        CollectionOutput {
                            id: 1,
                            attributes: Tag { title: "tag_1".to_string() },
                        },
                    ),
                    (
                        7,
                        CollectionOutput {
                            id: 3,
                            attributes: Tag { title: "tag_3".to_string() },
                        },
                    ),
                ]
            );
        }
    }
}

pub struct InsertJunction<const INVERSE: bool, Key, From, To, Data> {
    pub key: Key,
    pub from: From,
    pub to: To,
    pub data: Data,
}

mod impl_one_insert {
    use sqlx::{ColumnIndex, Decode, Row, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        operations::{
            Operation, OperationOutput, operations_expressions_crossover::TableExpressions,
        },
        sqlx_query_builder::{
            Expression, Join, StatementBuilder,
            sanitize_combinator::Sanitize,
            statements::insert_statement::{InsertStatement, One},
        },
    };

    use super::InsertJunction;

    impl<const INVERSE: bool, Key, From, To, Data> OperationOutput
        for InsertJunction<INVERSE, Key, From, To, One<Data>>
    where
        From: Collection<Id: SingleColumnId>,
        To: Collection<Id: SingleColumnId>,
    {
        type Output = (
            <From::Id as CollectionId>::IdData,
            <To::Id as CollectionId>::IdData,
        );
    }

    impl<const INVERSE: bool, S, Key, From, To, Data> Operation<S>
        for InsertJunction<INVERSE, Key, From, To, One<Data>>
    where
        S: DatabaseExt,
        S: ExecutorTrait,
        Key: Send
            + Clone
            + AsRef<str>
            + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
            + 'static,
        From: Send + Collection<Id: Send + SingleColumnId>,
        From: TableExpressions<
                PascalCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        From::Id: Send + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>,
        To: Send + Collection<Id: Send + SingleColumnId>,
        To: TableExpressions<
                PascalCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        To::Id: Send + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>,
        Data: for<'q> Expression<'q, S> + Send,
        usize: ColumnIndex<S::Row>,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let (stmt, args) = if INVERSE {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(InsertStatement {
                        table_name: t,
                        identifiers: Join {
                            start: "",
                            separator: ", ",
                            items: (
                                Sanitize(((self.from.table_name_snake_case(),), "_id")),
                                Sanitize(((self.to.table_name_snake_case(),), "_id")),
                            ),
                        },
                        values: self.data,
                        returning: (
                            Sanitize(((self.from.table_name_snake_case(),), "_id")),
                            Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        ),
                    })
                    .unwrap()
                } else {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(InsertStatement {
                        table_name: t,
                        identifiers: Join {
                            start: "",
                            separator: ", ",
                            items: (
                                Sanitize(((self.from.table_name_snake_case(),), "_id")),
                                Sanitize(((self.to.table_name_snake_case(),), "_id")),
                            ),
                        },
                        values: self.data,
                        returning: (
                            Sanitize(((self.from.table_name_snake_case(),), "_id")),
                            Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        ),
                    })
                    .unwrap()
                };

                let row = S::fetch_optional(
                    &mut *pool,
                    Executable {
                        string: &stmt,
                        arguments: args,
                    },
                )
                .await
                .unwrap()
                .unwrap();

                (Row::get(&row, 0), Row::get(&row, 1))
            }
        }
    }
}

pub struct DeleteJunction<const INVERSE: bool, Key, From, To, Where> {
    pub key: Key,
    pub from: From,
    pub to: To,
    pub wheres: Where,
}

mod impl_on_delete {
    use super::DeleteJunction;
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        operations::{
            Operation, OperationOutput, operations_expressions_crossover::TableExpressions,
        },
        sqlx_query_builder::{
            Expression, StatementBuilder, sanitize_combinator::Sanitize,
            statements::delete_statement::DeleteStatement,
        },
    };
    use sqlx::{ColumnIndex, Decode, Row, Type};

    impl<const INVERSE: bool, Key, From, To, Where> OperationOutput
        for DeleteJunction<INVERSE, Key, From, To, Where>
    where
        From: Collection<Id: SingleColumnId>,
        To: Collection<Id: SingleColumnId>,
    {
        type Output = Vec<(
            <From::Id as CollectionId>::IdData,
            <To::Id as CollectionId>::IdData,
        )>;
    }

    impl<const INVERSE: bool, S, Key, From, To, Where> Operation<S>
        for DeleteJunction<INVERSE, Key, From, To, Where>
    where
        S: DatabaseExt,
        S: ExecutorTrait,
        Key: Send
            + Clone
            + AsRef<str>
            + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite
            + 'static,
        From: Send + Collection<Id: Send + SingleColumnId>,
        From: TableExpressions<
                PascalCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        From::Id: Send + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>,
        To: Send + Collection<Id: Send + SingleColumnId>,
        To: TableExpressions<
                PascalCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
                SnakeCase: 'static + crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite,
            >,
        To::Id: Send + CollectionId<IdData: Send + Type<S> + for<'q> Decode<'q, S>>,
        Where: for<'q> Expression<'q, S> + Send,
        usize: ColumnIndex<S::Row>,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let (stmt, args) = if INVERSE {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(DeleteStatement {
                        table_name: t,
                        returning: (
                            Sanitize(((self.from.table_name_snake_case(),), "_id")),
                            Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        ),
                        wheres: (self.wheres,),
                    })
                    .unwrap()
                } else {
                    let t = Sanitize((
                        "JUNCTION_",
                        (self.to.table_name_pascal_case(),),
                        "_",
                        (self.from.table_name_pascal_case(),),
                        "_",
                        (self.key,),
                    ));
                    StatementBuilder::<S>::new(DeleteStatement {
                        table_name: t,
                        returning: (
                            Sanitize(((self.from.table_name_snake_case(),), "_id")),
                            Sanitize(((self.to.table_name_snake_case(),), "_id")),
                        ),
                        wheres: (self.wheres,),
                    })
                    .unwrap()
                };

                let rows = S::fetch_all(
                    &mut *pool,
                    Executable {
                        string: &stmt,
                        arguments: args,
                    },
                )
                .await
                .unwrap();

                rows.into_iter()
                    .map(|row| (Row::get(&row, 0), Row::get(&row, 1)))
                    .collect()
            }
        }
    }
}

#[cfg(test)]
async fn seed_todo_tag_junction(conn: &mut sqlx::SqliteConnection) {
    sqlx::query(
        r#"
                CREATE TABLE "Todo" (
                    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
                    "title" TEXT NOT NULL,
                    "done" INTEGER NOT NULL,
                    "description" TEXT
                );
                CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                CREATE TABLE "JUNCTION_Todo_Tag__def" (
                    "todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE,
                    "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE,
                    PRIMARY KEY ("todo_id", "tag_id")
                );
                CREATE TABLE "JUNCTION_Tag_Todo__def" (
                    "todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE,
                    "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE,
                    PRIMARY KEY ("todo_id", "tag_id")
                );

                INSERT INTO "Todo" ("title", "done", "description") VALUES
                    ("todo_1", 0, NULL), ("todo_2", 0, NULL), ("todo_3", 0, NULL),
                    ("todo_4", 0, NULL), ("todo_5", 0, NULL), ("todo_6", 0, NULL),
                    ("todo_7", 0, NULL), ("todo_8", 0, NULL), ("todo_9", 0, NULL),
                    ("todo_10", 0, NULL);
                INSERT INTO "Tag" ("title") VALUES ("tag_1"), ("tag_2"), ("tag_3");
                INSERT INTO "JUNCTION_Todo_Tag__def" ("todo_id", "tag_id") VALUES
                    (1, 2), (2, 3), (4, 1), (5, 1), (5, 2), (5, 3), (7, 1), (7, 3), (8, 1), (10, 3);
                INSERT INTO "JUNCTION_Tag_Todo__def" ("todo_id", "tag_id") VALUES
                    (1, 2), (2, 3), (4, 1), (5, 1), (5, 2), (5, 3), (7, 1), (7, 3), (8, 1), (10, 3);
                "#,
    )
    .execute(conn)
    .await
    .unwrap();
}
