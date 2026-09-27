use crate::{
    collections::Collection,
    from_row::FromRowData,
    links::{LinkedToBase, LinkedViaIds},
};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ManyToMany<const INVERSE: bool, Key, From, To> {
    pub relation_key: Key,
    pub from: From,
    pub to: To,
}

impl<const INVERSE: bool, VKey, From, To> LinkedViaIds for ManyToMany<INVERSE, VKey, From, To> {}

impl<const INVERSE: bool, VKey, From, To> LinkedToBase for ManyToMany<INVERSE, VKey, From, To> {
    type Base = From;
}

pub struct InsertJunctionManyRows<const INVERSE: bool, Key, From, To, I>
where
    From: Collection,
    From::Id: FromRowData,
{
    pub relation: ManyToMany<INVERSE, Key, From, To>,
    pub from_id: <From::Id as FromRowData>::RData,
    pub to_ids: I,
}

mod impl_insert_junction_many_rows {
    use std::future::Future;

    use sqlx::{Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        from_row::FromRowData,
        links::{
            fetch_linked_records::ManyToManyJunctionNames,
            relation_many_to_many::{InsertJunctionManyRows},
        },
        operations::Operation,
        sqlx_query_builder::{
            Join, StatementBuilder,
            basic_expressions::Bind,
            statements::insert_statement::{InsertStatement, IteratorSpec},
        },
    };

    impl<const INVERSE: bool, Key, From, To, I> crate::operations::OperationOutput
        for InsertJunctionManyRows<INVERSE, Key, From, To, I>
    where
        From: Collection,
        From::Id: FromRowData,
        To: Collection,
    {
        type Output = ();
    }

    impl<const INVERSE: bool, S, Key, From, To, I> Operation<S>
        for InsertJunctionManyRows<INVERSE, Key, From, To, I>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId + FromRowData> + Clone + Send,
        <From::Id as FromRowData>::RData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        To: Collection<Id: SingleColumnId> + Clone + Send,
        <To::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        I: Send + IntoIterator<Item = <To::Id as CollectionId>::IdData>,
    {
        fn exec_operation(self, pool: &mut S::Connection) -> impl Future<Output = ()> + Send {
            async move {
                let junction = self.relation.junction_table_as_str();
                let from_col = self.relation.from_junction_col_as_str();
                let to_col = self.relation.to_junction_col_as_str();
                let from_id = self.from_id;

                let (stmt, args) = StatementBuilder::<'_, S>::new(InsertStatement {
                    table_name: junction,
                    identifiers: Join {
                        start: "",
                        separator: ", ",
                        items: (from_col.as_str(), to_col.as_str()),
                    },
                    values: IteratorSpec(self.to_ids.into_iter().map(|to_id| {
                        Join {
                            start: "",
                            separator: ", ",
                            items: (Bind(from_id), Bind(to_id)),
                        }
                    })),
                    returning: (),
                })
                .unwrap();

                S::fetch_all(
                    &mut *pool,
                    Executable {
                        string: &stmt,
                        arguments: args,
                    },
                )
                .await
                .unwrap();
            }
        }
    }
}

mod impl_on_migrate {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, SingleColumnId},
        links::{
            relation_many_to_many::ManyToMany,
            utils::{ConventionalJunctionTableName, conventional_junction_table_name},
        },
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, MigrateExpression, TableExpressions,
        },
        sqlx_query_builder::{
            Join,
            basic_expressions::{CompositePrimaryKey, ForeignKey, OnDeleteCascase, TypeAsSyntax},
            sanitize_combinator::Sanitize,
            statements::create_table_statement::{
                ColumnDefinition, CreateTable, NotNull, expressions::CreateTableInit,
            },
        },
    };

    impl<VKey, From, To> MigrateExpression for ManyToMany<false, VKey, From, To>
    where
        To: 'static + TableExpressions,
        To::Id: ExpressionsForOperation,
        From: 'static + TableExpressions,
        From::Id: ExpressionsForOperation,
        VKey: AsRef<str> + Clone,
        From: Collection<Id: SingleColumnId> + Clone + TableExpressions,
        To: Collection<Id: SingleColumnId> + Clone + TableExpressions,
    {
        type Migrate = CreateTable<
            CreateTableInit,
            Sanitize<ConventionalJunctionTableName<VKey, From, To>>,
            (
                ColumnDefinition<(
                    Sanitize<(From::SnakeCase, &'static str)>,
                    TypeAsSyntax<i64>,
                    NotNull,
                    ForeignKey<
                        From::PascalCase,
                        <From::Id as ExpressionsForOperation>::Identifier,
                        (OnDeleteCascase,),
                    >,
                )>,
                ColumnDefinition<(
                    Sanitize<(To::SnakeCase, &'static str)>,
                    TypeAsSyntax<i64>,
                    NotNull,
                    ForeignKey<
                        To::PascalCase,
                        <To::Id as ExpressionsForOperation>::Identifier,
                        (OnDeleteCascase,),
                    >,
                )>,
                CompositePrimaryKey<
                    Join<(
                        Sanitize<(From::SnakeCase, &'static str)>,
                        Sanitize<(To::SnakeCase, &'static str)>,
                    )>,
                >,
            ),
        >;

        fn migrate(&self) -> Self::Migrate {
            CreateTable {
                init: CreateTableInit,
                name: Sanitize(conventional_junction_table_name(
                    self.relation_key.clone(),
                    self.from.clone(),
                    self.to.clone(),
                )),
                col_defs: (
                    ColumnDefinition((
                        Sanitize((self.from.table_name_snake_case(), "_id")),
                        TypeAsSyntax(PhantomData::<i64>),
                        NotNull,
                        ForeignKey {
                            references_table: self.from.table_name_pascal_case(),
                            references_col: self.from.id().identifier(),
                            ons: (OnDeleteCascase,),
                        },
                    )),
                    ColumnDefinition((
                        Sanitize((self.to.table_name_snake_case(), "_id")),
                        TypeAsSyntax(PhantomData::<i64>),
                        NotNull,
                        ForeignKey {
                            references_table: self.to.table_name_pascal_case(),
                            references_col: self.to.id().identifier(),
                            ons: (OnDeleteCascase,),
                        },
                    )),
                    CompositePrimaryKey(Join {
                        start: "",
                        separator: ", ",
                        items: (
                            Sanitize((self.from.table_name_snake_case(), "_id")),
                            Sanitize((self.to.table_name_snake_case(), "_id")),
                        ),
                    }),
                ),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{
            links::{DefaultRelationKey, relation_many_to_many::ManyToMany},
            operations::operations_expressions_crossover::MigrateExpression,
            sqlx_query_builder::StatementBuilder,
            test_module::{TagHandler, TodoHandler},
        };

        #[test]
        fn on_migrate() {
            let sl = StatementBuilder::<sqlx::Sqlite>::new_no_data(
                ManyToMany::<false, _, _, _> {
                    relation_key: DefaultRelationKey,
                    from: TodoHandler,
                    to: TagHandler,
                }
                .migrate(),
            )
            .unwrap();

            pretty_assertions::assert_eq!(
                sl,
                r#"CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));"#
            );
        }
    }
}

mod impl_link_fetch_many {
    use std::collections::{HashMap};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId}, from_row::FromRowData, links::{junction_table_op::FetchJunction, relation_many_to_many::ManyToMany}, operations::{
            CollectionOutput, ManyLinkOutput, OperationOutput,
            fetch_many::LinkFetch,
            map_operation::MapOperation,
            operations_expressions_crossover::{
                ExpressionsForOperation, IdentifierColNames, TableExpressions,
            },
        }, sqlx_query_builder::{
            Bind, basic_expressions::ColumnIn, sanitize_combinator::Sanitize,
        },
    };

    impl<const INVERSE: bool, Key, From, To> LinkFetch for ManyToMany<INVERSE, Key, From, To>
    where
        Key: Clone + AsRef<str>,
        From: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        <From as TableExpressions>::SnakeCase: AsRef<str>,
        To: Collection<Id: SingleColumnId> + TableExpressions + Clone,
        To: ExpressionsForOperation<Identifier: IdentifierColNames>,
        <To as TableExpressions>::SnakeCase: AsRef<str>,
        <To as TableExpressions>::PascalCase: AsRef<str>,
        To::InputData: crate::tuple_trait::AsTuple,
        <From::Id as CollectionId>::IdData: Copy + Clone + std::hash::Hash + Eq,
        From::Id: FromRowData<RData = <From::Id as CollectionId>::IdData>,
    {
        type SelectItems = From::Id;

        fn non_aggregating_select_items(&self) -> Self::SelectItems {
            self.from.id()
        }

        type Join = ();

        fn non_duplicating_join_expressions(&self) -> Self::Join {}

        type Wheres = ();

        fn where_expressions(&self) -> Self::Wheres {}

        type Op = MapOperation<
            FetchJunction<
                true,
                INVERSE,
                Key,
                From,
                To,
                ColumnIn<
                    Sanitize<(<From as TableExpressions>::SnakeCase, &'static str)>,
                    Vec<Bind<<From::Id as CollectionId>::IdData>>,
                >,
            >,
            HashMap<
                <From::Id as CollectionId>::IdData,
                ManyLinkOutput<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>,
            >,
        >;

        fn take_many(
            &self,
            from_id: <Self::SelectItems as FromRowData>::RData,
            op: &mut <Self::Op as OperationOutput>::Output,
        ) -> Self::Output
        where
            Self::SelectItems: FromRowData,
        {
            ManyLinkOutput {
                many_output: op
                    .remove(&from_id)
                    .map(|e| e.many_output)
                    .unwrap_or_default(),
            }
        }

        type Output =
            ManyLinkOutput<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        type OpInput = Vec<Bind<<From::Id as CollectionId>::IdData>>;

        fn operation_initialize_input(&self) -> Self::OpInput {
            Vec::new()
        }

        fn operation_fix_on_many(
            &self,
            from_id: &<Self::SelectItems as FromRowData>::RData,
            input: &mut Self::OpInput,
        ) where
            Self::SelectItems: FromRowData,
        {
            input.push(Bind(from_id.clone()));
        }

        fn operation_construct(&self, input: Self::OpInput) -> Self::Op
        where
            Self::SelectItems: FromRowData,
        {
            MapOperation {
                operation: FetchJunction {
                    key: self.relation_key.clone(),
                    from: self.from.clone(),
                    to: self.to.clone(),
                    wheres: ColumnIn {
                        col: Sanitize((self.from.table_name_snake_case(), "_id")),
                        values: input,
                    },
                },
                map_fn: |items| {
                    let mut ret = HashMap::new();
                    for (from_id, to_row) in items {
                        ret.entry(from_id)
                            .or_insert_with(|| ManyLinkOutput {
                                many_output: Vec::new(),
                            })
                            .many_output
                            .push(to_row);
                    }
                    ret
                },
            }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{DefaultRelationKey, relation_many_to_many::ManyToMany},
            operations::{
                CollectionOutput, LinkedOutput, ManyLinkOutput, Operation,
                fetch_many::{FetchMany, ManyOutput},
            },
            test_module::{Tag, TagHandler, Todo, TodoHandler, todo_members},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn fetch_many() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent'), ('home');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES
                        ('todo_a', true, 'a'),
                        ('todo_not_link', true, NULL),
                        ('todo_b', false, 'b');
                    INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES
                        (1, 1),
                        (1, 2),
                        (3, 1);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    FetchMany {
                        base: TodoHandler,
                        wheres: (),
                        links: ManyToMany::<false,_,_,_> {
                            relation_key: DefaultRelationKey,
                            from: TodoHandler,
                            to: TagHandler,
                        },
                        cursor_order_by: todo_members::id,
                        cursor_first_item: (), 
                        limit: 10,
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        format!("SELECT {todo_members}, {link_member} {rest}",
                            todo_members = r#""Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription""#,
                            link_member = r#""Todo"."id" AS "lid""#,
                            rest = r#"FROM "Todo" ORDER BY "Todo"."id" LIMIT $1;"#
                        ),
                        format!("SELECT {from_id}, {tab_members} FROM \"ct_todo_tag_def\" {join} {wheres}",
                            from_id = r#""ct_todo_tag_def"."todo_id" AS "from_id""#,
                            tab_members = r#""ct_todo_tag_def"."tag_id" AS "to_id", "Tag"."title" AS "t_title""#,
                            join = r#"INNER JOIN "Tag" ON "ct_todo_tag_def"."tag_id" = "Tag"."id""#,
                            wheres = r#"WHERE "todo_id" IN ($1, $2, $3);"#
                        )
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    ManyOutput {
                        items: vec![
                            LinkedOutput {
                                id: 1,
                                attributes: Todo {
                                    title: "todo_a".to_string(),
                                    done: true,
                                    description: Some("a".to_string()),
                                },
                                links: ManyLinkOutput {
                                    many_output: vec![
                                        CollectionOutput {
                                            id: 1,
                                            attributes: Tag {
                                                title: "urgent".to_string(),
                                            },
                                        },
                                        CollectionOutput {
                                            id: 2,
                                            attributes: Tag {
                                                title: "home".to_string(),
                                            },
                                        },
                                    ],
                                },
                            },
                            LinkedOutput {
                                id: 2,
                                attributes: Todo {
                                    title: "todo_not_link".to_string(),
                                    done: true,
                                    description: None,
                                },
                                links: ManyLinkOutput {
                                    many_output: vec![]
                                },
                            },
                            LinkedOutput {
                                id: 3,
                                attributes: Todo {
                                    title: "todo_b".to_string(),
                                    done: false,
                                    description: Some("b".to_string()),
                                },
                                links: ManyLinkOutput {
                                    many_output: vec![CollectionOutput {
                                        id: 1,
                                        attributes: Tag {
                                            title: "urgent".to_string(),
                                        },
                                    },],
                                },
                            },
                        ],
                        next_item: None,
                    }
                );
            })
            .await;
        }
    }
}

mod impl_many_to_many_set_new {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId}, from_row::FromRowData, links::{
            relation_many_to_many::{InsertJunctionManyRows, ManyToMany},
            update_links::SetNew,
        }, operations::{
            CollectionOutput, ManyLinkOutput, OperationOutput,
            insert::{
                AbortOperation, ConstraintViolation, InsertLinkConsumeData, InsertLinkData,
                InsertOne, InsertOneLink,
            },
        }, sqlx_query_builder::statements::insert_statement::IteratorSpec,
    };

    impl<const INVERSE: bool, Key, From, To, I> InsertLinkConsumeData
        for SetNew<ManyToMany<INVERSE, Key, From, To>, IteratorSpec<I>>
    where
        I: IntoIterator<Item = To::InputData>,
        From: Collection<Id: SingleColumnId + FromRowData> + Clone,
        To: Collection<Id: SingleColumnId> + Clone,
        <To::Id as CollectionId>::IdData: Clone,
        Key: Clone + AsRef<str>,
    {
        type Link = SetNew<ManyToMany<INVERSE, Key, From, To>, PhantomData<IteratorSpec<I>>>;

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
                SetNew {
                    relation: self.relation,
                    data: PhantomData,
                },
                InsertLinkData {
                    pre_op_data: self.data,
                    insert_value_data: (),
                    post_op_data: (),
                },
            )
        }
    }

    impl<const INVERSE: bool, Key, From, To, I> InsertOneLink
        for SetNew<ManyToMany<INVERSE, Key, From, To>, PhantomData<IteratorSpec<I>>>
    where
        I: IntoIterator<Item = To::InputData>,
        To: Collection<Id: SingleColumnId> + Clone,
        From: Collection<Id: SingleColumnId + FromRowData> + Clone,
        <To::Id as CollectionId>::IdData: Clone,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
    {
        type PreOp = InsertOne<To, IteratorSpec<I>, AbortOperation>;
        type PreOpData = IteratorSpec<I>;
        type InsertValuesData = ();
        type PostOpData = ();

        fn pre_operation_init(&self, data: Self::PreOpData) -> Self::PreOp {
            InsertOne {
                handler: self.relation.to.clone(),
                data,
                infalibility: AbortOperation,
            }
        }

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
        > {
            let tag_ids = pre_op_output
                .iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>();
            Ok(((), pre_op_output, tag_ids))
        }

        type PreOpToInsertValue = ();
        type PreOpToTake = Vec<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;
        type PreOpToPostOp = Vec<<To::Id as CollectionId>::IdData>;

        type InsertReturning = ();
        fn insert_returning(&self) -> Self::InsertReturning {}

        type InsertSets = ();
        fn insert_value(&self, _: Self::InsertValuesData, _: Self::PreOpToInsertValue) -> Self::InsertSets {}

        type FromRow = From::Id;
        fn from_row(&self) -> Self::FromRow {
            self.relation.from.id()
        }

        type TakeInput = ();

        type PostOp = InsertJunctionManyRows<
            INVERSE,
            Key,
            From,
            To,
            Vec<<To::Id as CollectionId>::IdData>,
        >;

        fn from_row_result(
            &self,
            _: Self::PostOpData,
            from_id: <Self::FromRow as FromRowData>::RData,
            to_ids: Self::PreOpToPostOp,
        ) -> (Self::PostOp, Self::TakeInput) {
            (
                InsertJunctionManyRows {
                    relation: self.relation.clone(),
                    from_id,
                    to_ids,
                },
                (),
            )
        }

        type PostOpOutput = ();
        fn post_op_output(
            &self,
            _: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(())
        }

        type Output = ManyLinkOutput<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        fn take(
            self,
            _: Self::PostOpOutput,
            _: Self::TakeInput,
            tags: Self::PreOpToTake,
        ) -> Self::Output {
            ManyLinkOutput { many_output: tags }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{DefaultRelationKey, relation_many_to_many::ManyToMany, update_links::SetNew},
            operations::{
                CollectionOutput, LinkedOutput, ManyLinkOutput, Operation,
                insert::{AbortOperation, InsertEntity, InsertOne},
            },
            sqlx_query_builder::statements::insert_statement::IteratorSpec,
            test_module::{Tag, TagHandler, Todo, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn insert_new() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    InsertOne {
                        handler: TodoHandler,
                        data: InsertEntity {
                            attributes: Todo {
                                title: "todo_a".to_string(),
                                done: true,
                                description: Some("a".to_string()),
                            },
                            link: SetNew {
                                relation: ManyToMany::<false, _, _, _> {
                                    relation_key: DefaultRelationKey,
                                    from: TodoHandler,
                                    to: TagHandler,
                                },
                                data: IteratorSpec(vec![
                                    Tag {
                                        title: "urgent".to_string(),
                                    },
                                    Tag {
                                        title: "home".to_string(),
                                    },
                                ]),
                            },
                        },
                        infalibility: AbortOperation,
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"INSERT INTO "Tag" ("title") VALUES ($1), ($2) RETURNING "id", "title";"#,
                        r#"INSERT INTO "Todo" ("title", "done", "description") VALUES ($1, $2, $3) RETURNING "id", "title", "done", "description";"#,
                        r#"INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES ($1, $2), ($3, $4);"#,
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "todo_a".to_string(),
                            done: true,
                            description: Some("a".to_string()),
                        },
                        links: ManyLinkOutput {
                            many_output: vec![
                                CollectionOutput {
                                    id: 1,
                                    attributes: Tag {
                                        title: "urgent".to_string(),
                                    },
                                },
                                CollectionOutput {
                                    id: 2,
                                    attributes: Tag {
                                        title: "home".to_string(),
                                    },
                                },
                            ],
                        },
                    },
                );
            })
            .await;
        }
    }
}


pub mod junction_from_id_set {
    use crate::{
        collections::Collection,
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, SelfPrescribedInsert, TableExpressions,
        },
        sqlx_query_builder::basic_expressions::{Bind, UpdatingColumn},
    };

    pub struct JunctionFromIdSet<From, B> {
        pub from: From,
        pub bind: B,
    }

    impl<From, B> SelfPrescribedInsert for JunctionFromIdSet<From, B>
    where
        From: TableExpressions + Collection<Id: ExpressionsForOperation>,
        B: Copy,
    {
        type InsertValue = B;
        type InsertId = ();
        fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
            ((), self.bind)
        }
        type UpdateSets =
            UpdatingColumn<<From::Id as ExpressionsForOperation>::Identifier, Option<Bind<B>>>;
        fn on_update(self) -> Self::UpdateSets {
            UpdatingColumn {
                col: self.from.id().identifier(),
                set: Some(Bind(self.bind)),
            }
        }
    }
}

mod impl_set_id_for_insert {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        from_row::{FromRowData, named_col_from_row::NamedColFromRow},
        links::{
            fetch_linked_records::InsertJunctionAndFetch, relation_many_to_many::ManyToMany,
            update_links::SetId,
        },
        operations::{
            CollectionOutput, LinkedOutput, OperationOutput,
            insert::{ConstraintViolation, InsertLinkConsumeData, InsertLinkData, InsertOneLink},
        },
    };

    impl<Key, From, To> InsertLinkConsumeData
        for SetId<ManyToMany<false, Key, From, To>, <To::Id as CollectionId>::IdData>
    where
        To: Collection<Id: SingleColumnId> + Clone,
        From: Collection<Id: SingleColumnId> + Clone,
        <From::Id as CollectionId>::IdData: Into<i64>,
        <To::Id as CollectionId>::IdData: Into<i64>,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
    {
        type Link = SetId<ManyToMany<false, Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>;

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
                SetId {
                    relation: self.relation,
                    id: PhantomData,
                },
                InsertLinkData {
                    pre_op_data: (),
                    insert_value_data: (),
                    post_op_data: self.id,
                },
            )
        }
    }

    impl<Key, From, To> InsertOneLink
        for SetId<ManyToMany<false, Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>
    where
        To: Collection<Id: SingleColumnId> + Clone,
        From: Collection<Id: SingleColumnId> + Clone,
        <From::Id as CollectionId>::IdData: Into<i64>,
        <To::Id as CollectionId>::IdData: Into<i64>,
        Key: Clone + AsRef<str>,
    {
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
        fn insert_value(&self, _: Self::InsertValuesData, _: ()) -> Self::InsertSets {}
        type FromRow = NamedColFromRow<&'static str, <From::Id as CollectionId>::IdData>;
        fn from_row(&self) -> Self::FromRow {
            NamedColFromRow {
                name: "id",
                ty: PhantomData,
            }
        }
        type TakeInput = ();
        type PostOp = InsertJunctionAndFetch<Key, From, To>;
        type PostOpOutput = LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>;
        fn post_op_output(
            &self,
            poo: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(poo)
        }
        type PostOpData = <To::Id as CollectionId>::IdData;
        fn from_row_result(
            &self,
            to_id: Self::PostOpData,
            from_id: <Self::FromRow as FromRowData>::RData,
            _: Self::PreOpToPostOp,
        ) -> (Self::PostOp, Self::TakeInput) {
            (
                InsertJunctionAndFetch::new(self.relation.clone(), from_id.into(), to_id.into()),
                (),
            )
        }
        type Output = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;
        fn take(
            self,
            pre_op: LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>,
            _: (),
            _: Self::PreOpToTake,
        ) -> Self::Output {
            pre_op.into()
        }
    }
}

mod impl_set_junzcction_id_for_update {
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        links::{
            fetch_linked_records::InsertJunctionRow,
            relation_many_to_many::{ManyToMany, junction_from_id_set::JunctionFromIdSet},
        },
        operations::{
            CollectionOutput, LinkedOutput, OperationOutput,
            fetch_one::FetchOne,
            insert::ConstraintViolation,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
            update::{UpdateLink, UpdateLinkData, UpdateLinkSplit},
        },
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
    };

    #[derive(Clone)]
    pub struct SetJunctionId<Key, From, To> {
        pub relation: ManyToMany<false, Key, From, To>,
        pub from_id: i64,
        pub to_id: i64,
    }

    impl<Key, From, To> UpdateLinkSplit for SetJunctionId<Key, From, To>
    where
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + Clone,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: ::std::convert::From<i64> + Clone,
        From: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        ManyToMany<false, Key, From, To>: Clone,
    {
        type Link = Self;
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
        ) {
            let from_id = self.from_id;
            (
                self,
                UpdateLinkData {
                    wheres: (),
                    update_values: from_id,
                    pre_op: (),
                    post_op: (),
                },
            )
        }
    }

    impl<Key, From, To> UpdateLink for SetJunctionId<Key, From, To>
    where
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + Clone,
        From: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        ManyToMany<false, Key, From, To>: Clone,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: ::std::convert::From<i64> + Clone,
    {
        type InitSplitForPreOp = ();
        type PreOpSplitWheres = ();
        type PreOpSplitValues = ();
        type PreOpSplitPostOp = ();
        type PreOpSplitTake = ();
        type PreOp = InsertJunctionRow<Key, From, To>;
        fn pre_op(&self, _: Self::InitSplitForPreOp) -> Self::PreOp {
            InsertJunctionRow::new(self.relation.clone(), self.from_id, self.to_id)
        }
        fn split_pre_op(
            &self,
            _: <Self::PreOp as OperationOutput>::Output,
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
        type InitSplitForWheres = ();
        type UpdateWhere = ();
        fn wheres(&self, _: Self::InitSplitForWheres) -> Self::UpdateWhere {}
        type UpdateReturning = ();
        fn update_names(&self) -> Self::UpdateReturning {}
        type InitSplitForUpdateValues = i64;
        type UpdateSets = JunctionFromIdSet<From, i64>;
        fn update_values(
            &self,
            values: Self::InitSplitForUpdateValues,
            _: Self::PreOpSplitValues,
        ) -> Self::UpdateSets {
            JunctionFromIdSet {
                from: self.relation.from.clone(),
                bind: values,
            }
        }
        type FromRow = ();
        fn from_row(&self) -> Self::FromRow {}
        type PostOp = FetchOne<
            To,
            (),
            ColumnEqual<
                <To::Id as ExpressionsForOperation>::Identifier,
                Bind<<To::Id as CollectionId>::IdData>,
            >,
        >;
        type InitSplitPostOp = ();
        fn post_op(&self, _: Self::InitSplitPostOp, _: Self::PreOpSplitPostOp) -> Self::PostOp {
            FetchOne {
                base: self.relation.to.clone(),
                links: (),
                wheres: ColumnEqual {
                    col: self.relation.to.id().identifier(),
                    eq: Bind(<To::Id as CollectionId>::IdData::from(self.to_id)),
                },
            }
        }
        fn from_row_result(&self, _: &(), _: &mut Self::PostOp) {}
        type Output = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;
        type PostOpOutput = LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>;
        fn post_op_output(
            &self,
            poo: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(poo.expect("linked row should exist"))
        }
        fn take(
            &self,
            _: (),
            post_op: &mut Self::PostOpOutput,
            _: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            CollectionOutput {
                id: post_op.id.clone(),
                attributes: post_op.attributes.clone(),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey,
                relation_many_to_many::{ManyToMany, test_support::migrate_todo_tag_fixtures},
            },
            operations::{
                CollectionOutput, Operation,
                insert::AbortOperation,
                update::Update,
            },
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
            test_module::{Tag, TagHandler, TodoHandler, TodoPartial},
            track_sqlx_query::watch_sqlx_calls,
            update_mod::Update as PartialUpdate,
        };

        use super::SetJunctionId;

        #[tokio::test(flavor = "current_thread")]
        async fn set_junction_id_links_tag() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                let link = ManyToMany {
                    relation_key: DefaultRelationKey,
                    from: TodoHandler,
                    to: TagHandler,
                };
                migrate_todo_tag_fixtures(&mut conn, &link).await;

                sqlx::query(
                    r#"
                    INSERT INTO "Tag" ("title") VALUES ('urgent');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo', false, 'before');
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let out = Operation::<Sqlite>::exec_operation(
                    Update {
                        base: TodoHandler,
                        partial: TodoPartial {
                            title: PartialUpdate::Set("linked".to_string()),
                            done: PartialUpdate::Keep,
                            description: PartialUpdate::Keep,
                        },
                        wheres: ColumnEqual { col: "id", eq: Bind(1) },
                        links: SetJunctionId {
                            relation: link,
                            from_id: 1,
                            to_id: 1,
                        },
                        infalibility: AbortOperation,
                    },
                    &mut conn,
                )
                .await
                .into_iter()
                .next()
                .unwrap();

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES ($1, $2);"#
                            .to_string(),
                        r#"UPDATE "Todo" SET title = $1, "id" = $2 WHERE "id" = $3 RETURNING "id", "title", "done", "description";"#
                            .to_string(),
                        r#"SELECT "Tag"."id" AS "iid", "Tag"."title" AS "btitle" FROM "Tag" WHERE "id" = $1;"#
                            .to_string(),
                    ]
                );

                assert_eq!(
                    out.links,
                    CollectionOutput {
                        id: 1,
                        attributes: Tag {
                            title: "urgent".to_string(),
                        },
                    }
                );
            })
            .await;
        }
    }
}

mod impl_remove_junction_id_for_update {
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        links::{
            fetch_linked_records::DeleteJunctionRow,
            relation_many_to_many::{ManyToMany, junction_from_id_set::JunctionFromIdSet},
        },
        operations::{
            CollectionOutput, LinkedOutput, OperationOutput,
            fetch_one::FetchOne,
            insert::ConstraintViolation,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
            update::{UpdateLink, UpdateLinkData, UpdateLinkSplit},
        },
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
    };

    #[derive(Clone)]
    pub struct RemoveJunctionId<Key, From, To> {
        pub relation: ManyToMany<false, Key, From, To>,
        pub from_id: i64,
        pub to_id: i64,
    }

    impl<Key, From, To> UpdateLinkSplit for RemoveJunctionId<Key, From, To>
    where
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + Clone,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: ::std::convert::From<i64> + Clone,
        From: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        ManyToMany<false, Key, From, To>: Clone,
    {
        type Link = Self;
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
        ) {
            let from_id = self.from_id;
            (
                self,
                UpdateLinkData {
                    wheres: (),
                    update_values: from_id,
                    pre_op: (),
                    post_op: (),
                },
            )
        }
    }

    impl<Key, From, To> UpdateLink for RemoveJunctionId<Key, From, To>
    where
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + Clone,
        From: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        ManyToMany<false, Key, From, To>: Clone,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: ::std::convert::From<i64> + Clone,
    {
        type InitSplitForPreOp = ();
        type PreOpSplitWheres = ();
        type PreOpSplitValues = ();
        type PreOpSplitPostOp = ();
        type PreOpSplitTake =
            Option<LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>>;
        type PreOp = FetchOne<
            To,
            (),
            ColumnEqual<
                <To::Id as ExpressionsForOperation>::Identifier,
                Bind<<To::Id as CollectionId>::IdData>,
            >,
        >;
        fn pre_op(&self, _: Self::InitSplitForPreOp) -> Self::PreOp {
            FetchOne {
                base: self.relation.to.clone(),
                links: (),
                wheres: ColumnEqual {
                    col: self.relation.to.id().identifier(),
                    eq: Bind(<To::Id as CollectionId>::IdData::from(self.to_id)),
                },
            }
        }
        fn split_pre_op(
            &self,
            linked: <Self::PreOp as OperationOutput>::Output,
        ) -> Result<
            (
                Self::PreOpSplitWheres,
                Self::PreOpSplitValues,
                Self::PreOpSplitPostOp,
                Self::PreOpSplitTake,
            ),
            ConstraintViolation,
        > {
            Ok(((), (), (), linked))
        }
        type InitSplitForWheres = ();
        type UpdateWhere = ();
        fn wheres(&self, _: Self::InitSplitForWheres) -> Self::UpdateWhere {}
        type UpdateReturning = ();
        fn update_names(&self) -> Self::UpdateReturning {}
        type InitSplitForUpdateValues = i64;
        type UpdateSets = JunctionFromIdSet<From, i64>;
        fn update_values(
            &self,
            values: Self::InitSplitForUpdateValues,
            _: Self::PreOpSplitValues,
        ) -> Self::UpdateSets {
            JunctionFromIdSet {
                from: self.relation.from.clone(),
                bind: values,
            }
        }
        type FromRow = ();
        fn from_row(&self) -> Self::FromRow {}
        type PostOp = DeleteJunctionRow<Key, From, To>;
        type InitSplitPostOp = ();
        fn post_op(&self, _: Self::InitSplitPostOp, _: Self::PreOpSplitPostOp) -> Self::PostOp {
            DeleteJunctionRow::new(self.relation.clone(), self.from_id, self.to_id)
        }
        fn from_row_result(&self, _: &(), _: &mut Self::PostOp) {}
        type Output = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;
        type PostOpOutput = ();
        fn post_op_output(
            &self,
            _: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(())
        }
        fn take(
            &self,
            _: (),
            _: &mut Self::PostOpOutput,
            pre_op_split_take: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            let linked = pre_op_split_take.take().expect("linked row should exist");
            CollectionOutput {
                id: linked.id,
                attributes: linked.attributes,
            }
        }
    }
}

mod impl_for_delete {
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        from_row::FromRowData,
        links::{
            fetch_linked_records::{FetchManyToManyLinked, ManyToManyLinkedMap},
            relation_many_to_many::ManyToMany,
        },
        operations::{
            CollectionOutput, ManyLinkOutput,
            delete::{DeleteLink, DeleteLinkData, DeleteLinkPreOp, DeleteLinkSplit},
            operations_expressions_crossover::{
                ExpressionsForOperation, IdentifierColNames, TableExpressions,
            },
        },
    };

    #[derive(Clone)]
    pub struct DeleteManyToManyLinked<Key, From, To> {
        pub link: ManyToMany<false, Key, From, To>,
        pub from_id: i64,
    }

    impl<Key, From, To> DeleteLinkSplit for DeleteManyToManyLinked<Key, From, To>
    where
        Self: Clone,
        To: Collection,
        From: Collection,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        <From::Id as CollectionId>::IdData: ::std::convert::From<i64> + Copy + Eq + std::hash::Hash,
    {
        type Link = Self;
        type InitSplitForPreOp = ();
        fn init_split(self) -> (Self::Link, Self::InitSplitForPreOp, DeleteLinkData<()>) {
            (self, (), DeleteLinkData { wheres: () })
        }
    }

    impl<Wheres, Key, From, To> DeleteLinkPreOp<Wheres> for DeleteManyToManyLinked<Key, From, To>
    where
        Self: Clone,
        From: Collection<Id: SingleColumnId> + Clone + TableExpressions,
        To: Collection<Id: SingleColumnId> + Clone + TableExpressions,
        To: ExpressionsForOperation<Identifier: IdentifierColNames>,
        To::InputData: crate::tuple_trait::AsTuple,
        <From as TableExpressions>::SnakeCase: AsRef<str>,
        <To as TableExpressions>::SnakeCase: AsRef<str>,
        <To as TableExpressions>::PascalCase: AsRef<str>,
        <From::Id as CollectionId>::IdData: ::std::convert::From<i64> + Copy + Eq + std::hash::Hash,
        Wheres: Clone,
        Key: Clone + AsRef<str>,
    {
        type InitSplitForPreOp = ();
        type PreOp = FetchManyToManyLinked<Key, From, To>;
        fn pre_op(&self, _: Self::InitSplitForPreOp, _: &Wheres) -> Self::PreOp {
            FetchManyToManyLinked::new(self.link.clone(), vec![self.from_id.into()])
        }
    }

    impl<Key, From, To> DeleteLink for DeleteManyToManyLinked<Key, From, To>
    where
        Self: Clone,
        To: Collection,
        Key: Clone + AsRef<str>,
        From: Collection + Clone,
        To: Clone,
        <From::Id as CollectionId>::IdData: ::std::convert::From<i64> + Copy + Eq + std::hash::Hash,
    {
        type Output =
            ManyLinkOutput<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;
        type PreOpOutput = ManyToManyLinkedMap<
            <From::Id as CollectionId>::IdData,
            <To::Id as CollectionId>::IdData,
            To::OutputData,
        >;
        type PreOpSplitWheres = ();
        type PreOpSplitTake =
            Vec<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;
        fn split_pre_op(
            &self,
            mut pre_op: Self::PreOpOutput,
        ) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake) {
            ((), pre_op.remove(&self.from_id.into()).unwrap_or_default())
        }
        type InitSplitForWheres = ();
        type Wheres = ();
        fn wheres(&self, _: Self::InitSplitForWheres, _: Self::PreOpSplitWheres) -> Self::Wheres {}
        type DeleteReturnExpression = ();
        fn delete_return_expression(&self) -> Self::DeleteReturnExpression {}
        type DeleteReturnFromRow = ();
        fn from_row(&self) -> Self::DeleteReturnFromRow {}
        fn take_mut(
            &self,
            _: <Self::DeleteReturnFromRow as FromRowData>::RData,
            pre_op_split_take: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            ManyLinkOutput {
                many_output: std::mem::take(pre_op_split_take),
            }
        }
        fn take_once(
            &self,
            _: <Self::DeleteReturnFromRow as FromRowData>::RData,
            pre_op_split_take: Self::PreOpSplitTake,
        ) -> Self::Output {
            ManyLinkOutput {
                many_output: pre_op_split_take,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey,
                relation_many_to_many::{DeleteManyToManyLinked, ManyToMany},
            },
            operations::{
                CollectionOutput, LinkedOutput, ManyLinkOutput, Operation, delete::Delete,
            },
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
            test_module::{Tag, TagHandler, Todo, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn delete_todo_returns_linked_tags() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent'), ('home');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo_a', true, 'a');
                    INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES (1, 1), (1, 2);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let link = ManyToMany {
                    relation_key: DefaultRelationKey,
                    from: TodoHandler,
                    to: TagHandler,
                };

                let result = Operation::<Sqlite>::exec_operation(
                    Delete {
                        base: TodoHandler,
                        wheres: ColumnEqual { col: "id", eq: Bind(1) },
                        links: DeleteManyToManyLinked {
                            link,
                            from_id: 1,
                        },
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "ct_todo_tag_def"."todo_id" AS "from_id", "Tag"."id", "Tag"."title" FROM "ct_todo_tag_def" INNER JOIN "Tag" ON "ct_todo_tag_def"."tag_id" = "Tag"."id" WHERE "ct_todo_tag_def"."todo_id" IN ($1);"#
                            .to_string(),
                        r#"DELETE FROM "Todo" WHERE "id" = $1 RETURNING "id", "title", "done", "description";"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    result,
                    vec![LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "todo_a".to_string(),
                            done: true,
                            description: Some("a".to_string()),
                        },
                        links: ManyLinkOutput {
                            many_output: vec![
                                CollectionOutput {
                                    id: 1,
                                    attributes: Tag {
                                        title: "urgent".to_string(),
                                    },
                                },
                                CollectionOutput {
                                    id: 2,
                                    attributes: Tag {
                                        title: "home".to_string(),
                                    },
                                },
                            ],
                        },
                    },]
                );
            })
            .await;
        }
    }
}

pub use impl_for_delete::DeleteManyToManyLinked;
pub use impl_remove_junction_id_for_update::RemoveJunctionId;
pub use impl_set_junzcction_id_for_update::SetJunctionId;

#[cfg(test)]
pub(crate) mod test_support {
    use sqlx::Sqlite;

    use crate::{
        links::{DefaultRelationKey, relation_many_to_many::ManyToMany},
        operations::operations_expressions_crossover::MigrateExpression,
        sqlx_query_builder::StatementBuilder,
        test_module::{TagHandler, TodoHandler},
    };

    pub fn todo_to_tag_link() -> ManyToMany<false, DefaultRelationKey, TodoHandler, TagHandler> {
        ManyToMany::<false, _, _, _> {
            relation_key: DefaultRelationKey,
            from: TodoHandler,
            to: TagHandler,
        }
    }

    pub async fn migrate_todo_tag_fixtures(
        conn: &mut sqlx::SqliteConnection,
        link: &ManyToMany<false, DefaultRelationKey, TodoHandler, TagHandler>,
    ) {
        sqlx::query(
            r#"
            CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
            CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
            "#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        let sl = StatementBuilder::<Sqlite>::new_no_data(link.migrate()).unwrap();
        sqlx::query(&sl).execute(&mut *conn).await.unwrap();
    }
}
