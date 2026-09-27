use crate::links::{LinkedToBase, LinkedViaId};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct OneToMany<Id, F, T> {
    pub fk_unique_id: Id,
    pub from: F,
    pub to: T,
}

impl<Id, F, T> LinkedViaId for OneToMany<Id, F, T> {}

impl<Id, F, T> LinkedToBase for OneToMany<Id, F, T> {
    type Base = F;
}

mod impl_on_migrate {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, SingleColumnId},
        links::{
            relation_one_to_many::OneToMany,
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        },
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, MigrateExpression, TableExpressions,
        },
        sqlx_query_builder::{
            basic_expressions::{ForeignKey, OnDeleteSetNull, TypeAsSyntax},
            sanitize_combinator::Sanitize,
            statements::{
                add_column_statement::AddColumn,
                create_table_statement::ColumnDefinition,
            },
        },
    };

    impl<Key, F, T> MigrateExpression for OneToMany<Key, F, T>
    where
        Key: AsRef<str>,
        F: Collection + Clone,
        T: Collection<Id: SingleColumnId> + Clone,
        T::Id: ExpressionsForOperation,
        T: TableExpressions,
        F: TableExpressions,
        Key: Clone,
    {
        type Migrate = AddColumn<
            F::PascalCase,
            ColumnDefinition<(
                Sanitize<ConventionalForeignKeyName<Key, T>>,
                TypeAsSyntax<i64>,
                ForeignKey<
                    T::PascalCase,
                    <T::Id as ExpressionsForOperation>::Identifier,
                    (OnDeleteSetNull,),
                >,
            )>,
        >;
        fn migrate(&self) -> Self::Migrate {
            AddColumn {
                table: self.from.table_name_pascal_case(),
                col_def: ColumnDefinition((
                    Sanitize(conventional_foreign_key_name(
                        self.fk_unique_id.clone(),
                        &self.to,
                    )),
                    TypeAsSyntax(PhantomData),
                    ForeignKey {
                        references_table: self.to.table_name_pascal_case(),
                        references_col: self.to.id().identifier(),
                        ons: (OnDeleteSetNull,),
                    },
                )),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{
            links::Link,
            operations::operations_expressions_crossover::MigrateExpression,
            sqlx_query_builder::StatementBuilder,
            test_module::{CategoryHandler, TodoHandler},
        };

        #[test]
        fn on_migrate() {
            let sl = StatementBuilder::<sqlx::Sqlite>::new_no_data(
                Link::<TodoHandler>::spec(CategoryHandler).migrate(),
            )
            .unwrap();

            pretty_assertions::assert_eq!(
                sl,
                r#"ALTER TABLE "Todo" ADD COLUMN "fk_category_def" INTEGER REFERENCES "Category"("id") ON DELETE SET NULL;"#
            );
        }
    }
}

mod one_to_many_items_names {
    use core::fmt;

    use crate::{
        collections::{Collection, CollectionId},
        database_extention::DatabaseExt,
        from_row::{
            FromRowAlias, FromRowData, TryFromRowAlias,
            swich_to_base_id::{num_alias_to_base_id, str_alias_to_base_id},
        },
        links::utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, SelfPrescribedInsert, TableExpressions,
        },
        sqlx_query_builder::{
            Expression, Join, OpExpression, StatementBuilder,
            basic_expressions::UpdatingColumn,
            combinators::Nest,
            sanitize_combinator::Sanitize,
        },
    };

    // "from" id exists in the sql statement, and I want attributes and id of "to"
    #[derive(Clone, Debug)]
    pub struct OneToManyItems<FromId, ToId, ToAttributes> {
        pub from_id: FromId,
        pub to_id: ToId,
        pub to_attributes: ToAttributes,
    }

    impl<FromId, ToId, ToAttributes> OpExpression for OneToManyItems<FromId, ToId, ToAttributes> {
    }

    impl<FromId, ToId, ToAttributes> ExpressionsForOperation
        for OneToManyItems<FromId, ToId, ToAttributes>
    where
        FromId: ExpressionsForOperation,
        ToId: ExpressionsForOperation,
        ToAttributes: ExpressionsForOperation,
    {
        type Identifier = (ToId::Identifier, ToAttributes::Identifier);
        fn identifier(&self) -> Self::Identifier {
            (self.to_id.identifier(), self.to_attributes.identifier())
        }

        type Scoped = (ToId::Scoped, ToAttributes::Scoped);
        fn scoped(&self) -> Self::Scoped {
            (self.to_id.scoped(), self.to_attributes.scoped())
        }

        type ScopedAliased =
            Join<(Nest<ToId::ScopedAliased>, Nest<ToAttributes::ScopedAliased>)>;
        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            Join {
                start: "",
                separator: ", ",
                items: (
                    Nest(self.to_id.scoped_aliased(alias)),
                    Nest(self.to_attributes.scoped_aliased(alias)),
                ),
            }
        }

        type NumScopedAliased = (ToId::NumScopedAliased, ToAttributes::NumScopedAliased);
        fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
            (
                self.to_id.num_scoped_aliased(num, alias),
                self.to_attributes.num_scoped_aliased(num, alias),
            )
        }
    }

    impl<'q, S, FromId, ToId, ToAttributes> Expression<'q, S>
        for OneToManyItems<FromId, ToId, ToAttributes>
    where
        S: DatabaseExt,
        FromId: Expression<'q, S>,
        ToId: Expression<'q, S>,
        ToAttributes: Expression<'q, S>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            Join {
                start: "",
                separator: ", ",
                items: (self.to_id, self.to_attributes),
            }
            .expression(ctx);
        }
    }

    impl<FromId, To> FromRowData for OneToManyItems<FromId, To::Id, To>
    where
        FromId: CollectionId,
        To: FromRowData + Collection,
        To::Id: FromRowData,
    {
        type RData = (
            FromId::IdData,
            Option<(<To::Id as CollectionId>::IdData, To::OutputData)>,
        );
    }

    impl<'r, R, FromId, To> FromRowAlias<'r, R> for OneToManyItems<FromId, To::Id, To>
    where
        FromId: CollectionId + FromRowAlias<'r, R, RData = <FromId as CollectionId>::IdData>,
        To: Collection,
        To: FromRowAlias<'r, R, RData = <To as Collection>::OutputData>,
        To::Id: TryFromRowAlias<'r, R, RData = <To::Id as CollectionId>::IdData>,
        To::OutputData: fmt::Debug,
        FromId::IdData: fmt::Debug,
        <To::Id as CollectionId>::IdData: fmt::Debug,
        R: sqlx::Row,
        for<'q> &'q str: sqlx::ColumnIndex<R>,
        i64: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
        FromId: fmt::Debug,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, crate::from_row::FromRowError> {
            let _ = row;
            todo!()
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            let try_to_find_id = self.to_id.try_str_alias(row.clone())?;
            let found = if let Some(found) = try_to_find_id {
                Some((found, self.to_attributes.str_alias(row.clone())?))
            } else {
                None
            };

            Ok((self.from_id.str_alias(str_alias_to_base_id(row))?, found))
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            let try_to_find_id = self.to_id.try_num_alias(row.clone())?;
            let found = if let Some(found) = try_to_find_id {
                Some((found, self.to_attributes.num_alias(row.clone())?))
            } else {
                None
            };

            Ok((self.from_id.num_alias(num_alias_to_base_id(row))?, found))
        }
    }

    pub struct OneToManySet<Key, To, B> {
        pub key: Key,
        pub to: To,
        pub bind: B,
    }

    impl<Key, To, B> SelfPrescribedInsert for OneToManySet<Key, To, B>
    where
        Key: AsRef<str>,
        To: TableExpressions,
    {
        type InsertValue = B;

        type InsertId = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
            (
                Sanitize(conventional_foreign_key_name(self.key, &self.to)),
                self.bind,
            )
        }

        type UpdateSets = UpdatingColumn<Sanitize<ConventionalForeignKeyName<Key, To>>, B>;
        fn on_update(self) -> Self::UpdateSets {
            UpdatingColumn {
                col: Sanitize(conventional_foreign_key_name(self.key, &self.to)),
                set: self.bind,
            }
        }
    }
}

mod impl_link_fetch_many {
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        from_row::FromRowData,
        links::{
            relation_one_to_many::{OneToMany, one_to_many_items_names::OneToManyItems},
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        },
        operations::{
            CollectionOutput, OperationOutput,
            fetch_many::LinkFetch,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
        },
        sqlx_query_builder::{basic_expressions::JoinExpression, sanitize_combinator::Sanitize},
    };

    impl<Key, F, T> LinkFetch for OneToMany<Key, F, T>
    where
        Key: Clone + AsRef<str>,
        T: Collection<Id: SingleColumnId> + TableExpressions + Clone,
        F: Collection + TableExpressions + Clone,
        T::Id: ExpressionsForOperation,
        // or maybe this
        OneToManyItems<F::Id, T::Id, T>: FromRowData<
            RData = (
                <F::Id as CollectionId>::IdData,
                Option<(<T::Id as CollectionId>::IdData, T::OutputData)>,
            ),
        >,
    {
        type SelectItems = OneToManyItems<F::Id, T::Id, T>;

        fn non_aggregating_select_items(&self) -> Self::SelectItems {
            let s = OneToManyItems {
                from_id: self.from.id(),
                to_id: self.to.id(),
                to_attributes: self.to.clone(),
            };

            s
        }

        type Join = JoinExpression<
            T::PascalCase,
            <T::Id as ExpressionsForOperation>::Identifier,
            F::PascalCase,
            Sanitize<ConventionalForeignKeyName<Key, T>>,
        >;

        fn non_duplicating_join_expressions(&self) -> Self::Join {
            JoinExpression {
                join_type: "LEFT JOIN",
                foreign_table: self.to.table_name_pascal_case(),
                foreign_column: self.to.id().identifier(),
                local_table: self.from.table_name_pascal_case(),
                local_column: Sanitize(conventional_foreign_key_name(
                    self.fk_unique_id.clone(),
                    &self.to,
                )),
            }
        }

        type Wheres = ();

        fn where_expressions(&self) -> Self::Wheres {}

        type Op = ();

        type Output = Option<CollectionOutput<<T::Id as CollectionId>::IdData, T::OutputData>>;

        fn take_many(
            &self,
            item: <Self::SelectItems as FromRowData>::RData,
            _: &mut <Self::Op as OperationOutput>::Output,
        ) -> Self::Output
        where
            Self::SelectItems: FromRowData,
        {
            item.1.map(|e| CollectionOutput {
                id: e.0,
                attributes: e.1,
            })
        }

        fn operation_fix_on_many(
            &self,
            _: &<Self::SelectItems as FromRowData>::RData,
            _: &mut Self::Op,
        ) where
            Self::SelectItems: FromRowData,
        {
        }

        type OpInput = ();

        fn operation_initialize_input(&self) -> Self::OpInput {}

        fn operation_construct(&self, _: Self::OpInput) -> Self::Op
        where
            Self::SelectItems: FromRowData,
        {
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::{Sqlite, query};

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::Link,
            operations::{
                CollectionOutput, LinkedOutput, Operation,
                fetch_many::{FetchMany, ManyOutput},
            },
            test_module::{Category, CategoryHandler, Todo, TodoHandler, todo_members},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn fetch_many() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                query(
                    r#"
                    CREATE TABLE Category (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL
                    );
                    CREATE TABLE Todo (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL,
                        done BOOLEAN NOT NULL,
                        description TEXT,
                        fk_category_def INTEGER,
                        FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                    );

                    INSERT INTO Category (title) VALUES ('category_1');
                    INSERT INTO Todo (title, done, description, fk_category_def) VALUES
                        ('todo_1', true, 'description_1', 1),
                        ('todo_2', false, 'description_2', NULL);
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
                        links: Link::<TodoHandler>::spec(CategoryHandler),
                        cursor_order_by: todo_members::title,
                        cursor_first_item: None::<(
                            i64,
                            crate::operations::operations_expressions_crossover::NamedBind<
                                TodoHandler,
                                todo_members::title,
                                String,
                            >,
                        )>,
                        limit: 10,
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription", "Category"."id" AS "lid", "Category"."title" AS "ltitle" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" ORDER BY "Todo"."title" LIMIT $1;"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    ManyOutput {
                        items: vec![
                            LinkedOutput {
                                id: 1,
                                attributes: Todo {
                                    title: "todo_1".to_string(),
                                    done: true,
                                    description: Some("description_1".to_string()),
                                },
                                links: Some(CollectionOutput {
                                    id: 1,
                                    attributes: Category {
                                        title: "category_1".to_string(),
                                    },
                                }),
                            },
                            LinkedOutput {
                                id: 2,
                                attributes: Todo {
                                    title: "todo_2".to_string(),
                                    done: false,
                                    description: Some("description_2".to_string()),
                                },
                                links: None,
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

mod impl_set_new_for_insert {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId},
        links::{
            relation_one_to_many::{OneToMany, one_to_many_items_names::OneToManySet},
            update_links::SetNew,
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        },
        operations::{
            CollectionOutput, OperationOutput,
            insert::{
                AbortOperation, ConstraintViolation, InsertEntity, InsertLinkConsumeData,
                InsertLinkData, InsertOne, InsertOneLink,
            },
            operations_expressions_crossover::TableExpressions,
        },
        sqlx_query_builder::{Bind, sanitize_combinator::Sanitize},
    };

    impl<Key, From, To> InsertLinkConsumeData for SetNew<OneToMany<Key, From, To>, To::InputData>
    where
        To: Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        <To::Id as CollectionId>::IdData: Clone,
    {
        type Link = SetNew<OneToMany<Key, From, To>, PhantomData<To::InputData>>;

        fn consume_data(
            self,
        ) -> (
            Self::Link,
            crate::operations::insert::InsertLinkData<
                <Self::Link as crate::operations::insert::InsertOneLink>::PreOpData,
                <Self::Link as crate::operations::insert::InsertOneLink>::InsertValuesData,
                <Self::Link as crate::operations::insert::InsertOneLink>::PostOpData,
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

    impl<Key, From, To> InsertOneLink for SetNew<OneToMany<Key, From, To>, PhantomData<To::InputData>>
    where
        To: Clone + TableExpressions,
        From: Clone,
        Key: Clone + AsRef<str>,
        To: Collection,
        To::Id: CollectionId<IdData: Clone>,
    {
        type PreOp = InsertOne<To, InsertEntity<To::InputData, ()>, AbortOperation>;

        type PreOpData = To::InputData;

        fn pre_operation_init(&self, input: Self::PreOpData) -> Self::PreOp {
            InsertOne {
                handler: self.relation.to.clone(),
                data: InsertEntity {
                    attributes: input,
                    link: (),
                },
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
            let unwrapped = pre_op_output;
            Ok((unwrapped.id.clone(), unwrapped.into(), ()))
        }

        type PreOpToInsertValue = <To::Id as CollectionId>::IdData;
        type PreOpToTake = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;
        type PreOpToPostOp = ();

        type InsertSets = OneToManySet<Key, To, Bind<<To::Id as CollectionId>::IdData>>;
        fn insert_value(
            &self,
            _: Self::InsertValuesData,
            pre_op_output: Self::PreOpToInsertValue,
        ) -> Self::InsertSets {
            OneToManySet {
                key: self.relation.fk_unique_id.clone(),
                to: self.relation.to.clone(),
                bind: Bind(pre_op_output),
            }
        }

        type InsertReturning = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn insert_returning(&self) -> Self::InsertReturning {
            Sanitize(conventional_foreign_key_name(
                self.relation.fk_unique_id.clone(),
                &self.relation.to,
            ))
        }

        type InsertValuesData = ();

        type FromRow = ();

        fn from_row(&self) -> Self::FromRow {}

        type TakeInput = ();

        type PostOp = ();

        type PostOpData = ();

        fn from_row_result(
            &self,
            _: Self::PostOpData,
            _: <Self::FromRow as crate::from_row::FromRowData>::RData,
            _: Self::PreOpToPostOp,
        ) -> (Self::PostOp, Self::TakeInput) {
            ((), ())
        }

        type Output = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;

        type PostOpOutput = ();
        fn post_op_output(
            &self,
            _: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(())
        }

        fn take(
            self,
            _: <Self::PostOp as crate::operations::OperationOutput>::Output,
            _: Self::TakeInput,
            take: Self::PreOpToTake,
        ) -> Self::Output {
            take
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{Link, update_links::SetNew},
            operations::{
                CollectionOutput, LinkedOutput, Operation,
                insert::{AbortOperation, InsertEntity, InsertOne},
            },
            test_module::{self, Category, CategoryHandler, Todo},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn test_insert_one_set_new() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    "
                CREATE TABLE Category (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL
                );
                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT,
                    fk_category_def INTEGER,
                    FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                );
                ",
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    InsertOne {
                        handler: test_module::TodoHandler,
                        data: InsertEntity {
                            attributes: Todo {
                                title: "first_todo".to_string(),
                                done: true,
                                description: None,
                            },
                            link: SetNew {
                                relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                                data: Category {
                                    title: "category_1".to_string(),
                                },
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
                        r#"INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";"#
                            .to_string(),
                        r#"INSERT INTO "Todo" ("title", "done", "description", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "done", "description", "fk_category_def";"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "first_todo".to_string(),
                            done: true,
                            description: None,
                        },
                        links: CollectionOutput {
                            id: 1,
                            attributes: Category {
                                title: "category_1".to_string(),
                            },
                        },
                    }
                );
            })
            .await;
        }
    }
}

mod impl_set_id_for_insert {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId},
        from_row::named_col_from_row::NamedColFromRow,
        links::{
            relation_one_to_many::{OneToMany, one_to_many_items_names::OneToManySet},
            update_links::SetId,
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        },
        operations::{
            CollectionOutput, LinkedOutput, OperationOutput,
            fetch_one::FetchOne,
            insert::{ConstraintViolation, InsertLinkConsumeData, InsertLinkData, InsertOneLink},
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
        },
        sqlx_query_builder::{Bind, basic_expressions::ColumnEqual, sanitize_combinator::Sanitize},
    };

    impl<Key, From, To> InsertLinkConsumeData
        for SetId<OneToMany<Key, From, To>, <To::Id as CollectionId>::IdData>
    where
        To: Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        To::Id: ExpressionsForOperation,
    {
        type Link = SetId<OneToMany<Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>;

        fn consume_data(
            self,
        ) -> (
            Self::Link,
            crate::operations::insert::InsertLinkData<
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
                    insert_value_data: self.id,
                    post_op_data: (),
                },
            )
        }
    }

    impl<Key, From, To> InsertOneLink
        for SetId<OneToMany<Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>
    where
        To: TableExpressions,
        To: Collection,
        Key: Clone + AsRef<str>,
        From: Clone,
        To: Clone,
        To::Id: ExpressionsForOperation,
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

        type InsertReturning = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn insert_returning(&self) -> Self::InsertReturning {
            Sanitize(conventional_foreign_key_name(
                self.relation.fk_unique_id.clone(),
                &self.relation.to,
            ))
        }

        type InsertValuesData = <To::Id as CollectionId>::IdData;

        type InsertSets = OneToManySet<Key, To, Bind<<To::Id as CollectionId>::IdData>>;

        fn insert_value(
            &self,
            from_data: Self::InsertValuesData,
            _: Self::PreOpToInsertValue,
        ) -> Self::InsertSets {
            OneToManySet {
                key: self.relation.fk_unique_id.clone(),
                to: self.relation.to.clone(),
                bind: Bind(from_data),
            }
        }

        type FromRow = NamedColFromRow<
            Sanitize<ConventionalForeignKeyName<Key, To>>,
            <To::Id as CollectionId>::IdData,
        >;

        fn from_row(&self) -> Self::FromRow {
            NamedColFromRow {
                name: Sanitize(conventional_foreign_key_name(
                    self.relation.fk_unique_id.clone(),
                    &self.relation.to,
                )),
                ty: PhantomData,
            }
        }

        type TakeInput = ();

        type PostOp = FetchOne<
            To,
            (),
            ColumnEqual<
                <To::Id as ExpressionsForOperation>::Identifier,
                Bind<<To::Id as CollectionId>::IdData>,
            >,
        >;

        type PostOpOutput = LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>;
        fn post_op_output(
            &self,
            poo: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(poo.expect("sql query should have failed by now"))
        }

        type PostOpData = ();

        fn from_row_result(
            &self,
            _: (),
            from_row: <To::Id as CollectionId>::IdData,
            _: Self::PreOpToPostOp,
        ) -> (Self::PostOp, Self::TakeInput) {
            (
                FetchOne {
                    base: self.relation.to.clone(),
                    links: (),
                    wheres: ColumnEqual {
                        col: self.relation.to.id().identifier(),
                        eq: Bind(from_row),
                    },
                },
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

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{Link, update_links::SetId},
            operations::{
                CollectionOutput, LinkedOutput, Operation,
                insert::{AbortOperation, InsertEntity, InsertOne},
            },
            test_module::{self, Category, CategoryHandler, Todo},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn test_insert_one() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    "
                CREATE TABLE Category (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL
                );
                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT,
                    fk_category_def INTEGER,
                    FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                );

                INSERT INTO Category (title) VALUES ('category_1');
                ",
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    InsertOne {
                        handler: test_module::TodoHandler,
                        data: InsertEntity {
                            attributes: Todo {
                                title: "first_todo".to_string(),
                                done: true,
                                description: None,
                            },
                            link: SetId {
                                id: 1,
                                relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                            },
                        },
                        infalibility: AbortOperation,
                        // data: One(Todo {
                        //     title: "first_todo".to_string(),
                        //     done: true,
                        //     description: None,
                        // }),
                        // links: SetId {
                        //     id: 1,
                        //     relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                        // },
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"INSERT INTO "Todo" ("title", "done", "description", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "done", "description", "fk_category_def";"#
                            .to_string(),
                        r#"SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "first_todo".to_string(),
                            done: true,
                            description: None,
                        },
                        links: CollectionOutput {
                            id: 1,
                            attributes: Category {
                                title: "category_1".to_string(),
                            },
                        },
                    }
                );
            })
            .await;
        }
    }
}

mod impl_set_id_for_update {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId},
        from_row::named_col_from_row::NamedColFromRow,
        links::{
            relation_one_to_many::{OneToMany, one_to_many_items_names::OneToManySet},
            update_links::SetId,
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        },
        operations::{
            CollectionOutput, LinkedOutput, OperationOutput,
            fetch_one::FetchOne,
            insert::ConstraintViolation,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
            update::{UpdateLink, UpdateLinkData, UpdateLinkSplit},
        },
        sqlx_query_builder::{Bind, basic_expressions::ColumnEqual, sanitize_combinator::Sanitize},
    };

    impl<Key, From, To> UpdateLinkSplit
        for SetId<OneToMany<Key, From, To>, Option<<To::Id as CollectionId>::IdData>>
    where
        To: Clone + Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: Clone,
        To::Id: ExpressionsForOperation,
        OneToMany<Key, From, To>: Clone,
    {
        type Link = SetId<OneToMany<Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>;

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
            (
                SetId {
                    relation: self.relation,
                    id: PhantomData,
                },
                UpdateLinkData {
                    wheres: (),
                    update_values: self.id.clone(),
                    pre_op: (),
                    post_op: self.id,
                },
            )
        }
    }

    impl<Key, From, To> UpdateLink
        for SetId<OneToMany<Key, From, To>, PhantomData<<To::Id as CollectionId>::IdData>>
    where
        To: Clone + Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        To::OutputData: Clone,
        To::Id: ExpressionsForOperation,
        <To::Id as CollectionId>::IdData: Clone,
        OneToMany<Key, From, To>: Clone,
    {
        type InitSplitForPreOp = ();

        type PreOpSplitWheres = ();

        type PreOpSplitValues = ();

        type PreOpSplitPostOp = ();
        type PreOpSplitTake = ();
        type PreOp = ();

        fn pre_op(&self, _: Self::InitSplitForPreOp) -> Self::PreOp {}

        fn split_pre_op(
            &self,
            _: <Self::PreOp as OperationOutput>::Output,
        ) -> Result<(
            Self::PreOpSplitWheres,
            Self::PreOpSplitValues,
            Self::PreOpSplitPostOp,
            Self::PreOpSplitTake,
        ), ConstraintViolation> {
            Ok(((), (), (), ()))
        }

        type InitSplitForWheres = ();

        type UpdateWhere = ();

        fn wheres(&self, _: Self::InitSplitForWheres) -> Self::UpdateWhere {}

        type UpdateReturning = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn update_names(&self) -> Self::UpdateReturning {
            Sanitize(conventional_foreign_key_name(
                self.relation.fk_unique_id.clone(),
                &self.relation.to,
            ))
        }

        type InitSplitForUpdateValues = Option<<To::Id as CollectionId>::IdData>;

        type UpdateSets = OneToManySet<Key, To, Option<Bind<<To::Id as CollectionId>::IdData>>>;
        // type UpdateSets = UpdatingColumn<
        //     ConventionalForeignKeyName<Key, To>,
        //     Option<Bind<<To::Id as CollectionId>::IdData>>,
        // >;

        fn update_values(
            &self,
            values: Self::InitSplitForUpdateValues,
            _: Self::PreOpSplitValues,
        ) -> Self::UpdateSets {
            OneToManySet {
                key: self.relation.fk_unique_id.clone(),
                to: self.relation.to.clone(),
                bind: values.map(Bind),
            }
            // UpdatingColumn {
            //     col: conventional_foreign_key_name(
            //         self.relation.fk_unique_id.clone(),
            //         &self.relation.to,
            //     ),
            //     set: values.map(Bind),
            // }
        }

        type FromRow = NamedColFromRow<
            Sanitize<ConventionalForeignKeyName<Key, To>>,
            Option<<To::Id as CollectionId>::IdData>,
        >;
        fn from_row(&self) -> Self::FromRow {
            NamedColFromRow {
                name: Sanitize(conventional_foreign_key_name(
                    self.relation.fk_unique_id.clone(),
                    &self.relation.to,
                )),
                ty: PhantomData,
            }
        }

        type PostOp = Option<
            FetchOne<
                To,
                (),
                ColumnEqual<
                    <To::Id as ExpressionsForOperation>::Identifier,
                    Bind<<To::Id as CollectionId>::IdData>,
                >,
            >,
        >;

        type InitSplitPostOp = Option<<To::Id as CollectionId>::IdData>;

        fn post_op(&self, id: Self::InitSplitPostOp, _: Self::PreOpSplitPostOp) -> Self::PostOp {
            match id {
                None => None,
                Some(id) => Some(FetchOne {
                    base: self.relation.to.clone(),
                    links: (),
                    wheres: ColumnEqual {
                        col: self.relation.to.id().identifier(),
                        eq: Bind(id),
                    },
                }),
            }
        }

        fn from_row_result(
            &self,
            _: &Option<<To::Id as CollectionId>::IdData>,
            _: &mut Self::PostOp,
        ) {
        }

        type Output = Option<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        type PostOpOutput =
            Option<LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>>;

        fn post_op_output(
            &self,
            poo: <Self::PostOp as OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(poo.flatten())
        }

        fn take(
            &self,
            fk: Option<<To::Id as CollectionId>::IdData>,
            post_op: &mut Self::PostOpOutput,
            _: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            match (fk, post_op.take()) {
                (Some(id), Some(linked)) => Some(CollectionOutput {
                    id,
                    attributes: linked.attributes,
                }),
                _ => None,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::{Row, Sqlite};

        use crate::{
            connect_in_memory::ConnectInMemory,
            from_row::{FromRowAlias, RowStrAliased},
            links::{
                DefaultRelationKey, Link,
                relation_one_to_many::OneToMany,
                update_links::{SetId, SetNew},
            },
            operations::{
                CollectionOutput, LinkedOutput, Operation,
                insert::{AbortOperation, InsertEntity, InsertOne},
                update::Update,
            },
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
            test_module::{self, Category, CategoryHandler, Todo},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn set_for_update_link() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    "
                CREATE TABLE Category (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL
                );

                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT,
                    fk_category_def INTEGER,
                    FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                );

                INSERT INTO Category (title) VALUES ('category_1'), ('category_2');

                INSERT INTO Todo (title, done, description, fk_category_def) VALUES 
                    ('todo_1', true, 'description_1', 1),
                    ('todo_2', false, 'description_2', NULL),
                    ('todo_3', true, 'description_3', 1);
                ",
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let s = Update {
                    base: test_module::TodoHandler,
                    partial: Default::default(),
                    wheres: ColumnEqual {
                        col: "id",
                        eq: Bind(1),
                    },
                    links: SetId {
                        relation: OneToMany {
                            fk_unique_id: DefaultRelationKey,
                            from: test_module::TodoHandler,
                            to: CategoryHandler,
                        },
                        id: None,
                    },
                    infalibility: AbortOperation,
                };

                Operation::<Sqlite>::exec_operation(s, &mut conn)
                    .await
                    ;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"UPDATE "Todo" SET "fk_category_def" = NULL WHERE "id" = $1 RETURNING "id", "title", "done", "description", "fk_category_def";"#
                            .to_string(),
                    ]
                );

            let row = sqlx::query(
                "
SELECT 
    fk_category_def
FROM Todo 
WHERE id = 1;
",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();

            let fk_category_def: Option<i64> = row.get("fk_category_def");

            pretty_assertions::assert_eq!(fk_category_def, None);
            let _ = actions.take();

            let s = Update {
                base: test_module::TodoHandler,
                partial: Default::default(),
                wheres: ColumnEqual {
                    col: "id",
                    eq: Bind(2),
                },
                links: SetId {
                    relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                    id: Some(2),
                },
                infalibility: AbortOperation,
            };

            Operation::<Sqlite>::exec_operation(s, &mut conn)
                .await
                ;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"UPDATE "Todo" SET "fk_category_def" = $1 WHERE "id" = $2 RETURNING "id", "title", "done", "description", "fk_category_def";"#
                        .to_string(),
                    r#"SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;"#
                        .to_string(),
                ]
            );

            let row = sqlx::query(
                "
SELECT 
    fk_category_def,
    Category.title as title,
    Category.id as id
FROM Todo 
LEFT JOIN Category ON Todo.fk_category_def = Category.id 
WHERE Todo.id = 2;
",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();

            let fk_category_def: Option<i64> = row.get("fk_category_def");

            pretty_assertions::assert_eq!(fk_category_def, Some(2));

            let c = CategoryHandler.no_alias(&row).unwrap();

            pretty_assertions::assert_eq!(
                c,
                Category {
                    title: "category_2".to_string(),
                }
            );
            let _ = actions.take();

            let s = Update {
                base: test_module::TodoHandler,
                partial: Default::default(),
                wheres: ColumnEqual {
                    col: "id",
                    eq: Bind(2),
                },
                links: SetNew {
                    relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                    data: Category {
                        title: "category_3".to_string(),
                    },
                },
                infalibility: AbortOperation,
            };

            Operation::<Sqlite>::exec_operation(s, &mut conn)
                .await
                ;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";"#
                        .to_string(),
                    r#"UPDATE "Todo" SET "fk_category_def" = $1 WHERE "id" = $2 RETURNING "id", "title", "done", "description", "fk_category_def";"#
                        .to_string(),
                ]
            );

            let row = sqlx::query(
                "
SELECT 
    fk_category_def,
    Category.title as title,
    Category.id as id
FROM Todo 
LEFT JOIN Category ON Todo.fk_category_def = Category.id 
WHERE Todo.id = 2;
",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap();

            let fk_category_def: Option<i64> = row.get("fk_category_def");

            pretty_assertions::assert_eq!(fk_category_def, Some(3));

            let c = CategoryHandler.no_alias(&row).unwrap();

            pretty_assertions::assert_eq!(
                c,
                Category {
                    title: "category_3".to_string(),
                }
            );
            let _ = actions.take();

            let s = InsertOne {
                handler: test_module::TodoHandler,
                data: InsertEntity {
                    attributes: Todo {
                    title: "todo_4".to_string(),
                    done: false,
                    description: None,
                },
                link: SetId {
                    relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                    id: 3,
                }},
                infalibility: AbortOperation,
            };

            let s = Operation::<Sqlite>::exec_operation(s, &mut conn)
                .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"INSERT INTO "Todo" ("title", "done", "description", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "done", "description", "fk_category_def";"#
                        .to_string(),
                    r#"SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                s,
                LinkedOutput {
                    id: 4,
                    attributes: Todo {
                        title: "todo_4".to_string(),
                        done: false,
                        description: None,
                    },
                    links: CollectionOutput {
                        id: 3,
                        attributes: Category {
                            title: "category_3".to_string(),
                        },
                    },
                }
            );

            let s = InsertOne {
                handler: test_module::TodoHandler,
                data: InsertEntity {
                    attributes: Todo {
                    title: "todo_5".to_string(),
                    done: false,
                    description: None,
                },
                link: SetNew {
                    relation: <CategoryHandler as Link<test_module::TodoHandler>>::spec(CategoryHandler),
                    data: Category {
                        title: "category_4".to_string(),
                    },
                },
                
            },
                infalibility: AbortOperation,
        };

            let s = Operation::<Sqlite>::exec_operation(s, &mut conn)
                .await
                ;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";"#
                        .to_string(),
                    r#"INSERT INTO "Todo" ("title", "done", "description", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "done", "description", "fk_category_def";"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                s,
                LinkedOutput {
                    id: 5,
                    attributes: Todo {
                        title: "todo_5".to_string(),
                        done: false,
                        description: None,
                    },
                    links: CollectionOutput {
                        id: 4,
                        attributes: Category {
                            title: "category_4".to_string(),
                        },
                    },
                }
            );

            let todos = sqlx::query(
                "
SELECT 
    Todo.title as todo_title, Todo.done as todo_done, Todo.description as todo_description,
    Category.title as category_title,
    Todo.fk_category_def
FROM Todo
LEFT JOIN Category ON Todo.fk_category_def = Category.id;
                ",
            )
            .fetch_all(&mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|e| {
                let linked = e.get::<Option<i64>, _>("fk_category_def");
                (
                    test_module::TodoHandler
                        .str_alias(RowStrAliased::new(&e, "todo_"))
                        .unwrap(),
                    if let Some(linked) = linked {
                        Some(CollectionOutput {
                            id: linked,
                            attributes: CategoryHandler
                                .str_alias(RowStrAliased::new(&e, "category_"))
                                .unwrap(),
                        })
                    } else {
                        None
                    },
                )
            })
            .collect::<Vec<_>>();

            pretty_assertions::assert_eq!(
                todos,
                vec![
                    (
                        Todo {
                            title: "todo_1".to_string(),
                            done: true,
                            description: Some("description_1".to_string()),
                        },
                        None,
                    ),
                    (
                        Todo {
                            title: "todo_2".to_string(),
                            done: false,
                            description: Some("description_2".to_string()),
                        },
                        Some(CollectionOutput {
                            id: 3,
                            attributes: Category {
                                title: "category_3".to_string(),
                            },
                        }),
                    ),
                    (
                        Todo {
                            title: "todo_3".to_string(),
                            done: true,
                            description: Some("description_3".to_string()),
                        },
                        Some(CollectionOutput {
                            id: 1,
                            attributes: Category {
                                title: "category_1".to_string(),
                            },
                        }),
                    ),
                    (
                        Todo {
                            title: "todo_4".to_string(),
                            done: false,
                            description: None,
                        },
                        Some(CollectionOutput {
                            id: 3,
                            attributes: Category {
                                title: "category_3".to_string(),
                            },
                        }),
                    ),
                    (
                        Todo {
                            title: "todo_5".to_string(),
                            done: false,
                            description: None,
                        },
                        Some(CollectionOutput {
                            id: 4,
                            attributes: Category {
                                title: "category_4".to_string(),
                            },
                        }),
                    ),
                ],
            );
            })
            .await;
        }
    }
}

mod impl_set_new_for_update {
    use std::marker::PhantomData;

    use crate::{
        collections::{Collection, CollectionId}, from_row::named_col_from_row::NamedColFromRow, links::{
            relation_one_to_many::{OneToMany, one_to_many_items_names::OneToManySet},
            update_links::SetNew,
            utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        }, operations::{
            CollectionOutput, LinkedOutput, insert::{AbortOperation, ConstraintViolation, InsertEntity, InsertOne}, operations_expressions_crossover::TableExpressions, update::{UpdateLink, UpdateLinkData, UpdateLinkSplit},
        }, sqlx_query_builder::{Bind, sanitize_combinator::Sanitize},
    };

    impl<Key, From, To> UpdateLinkSplit for SetNew<OneToMany<Key, From, To>, To::InputData>
    where
        To: Clone + Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        OneToMany<Key, From, To>: Clone,
        To::InputData: Clone,
        To::OutputData: Clone,
    {
        type Link = SetNew<OneToMany<Key, From, To>, PhantomData<To::InputData>>;
        fn init_split(self) -> (Self::Link, UpdateLinkData<(), (), To::InputData, ()>) {
            (
                SetNew {
                    relation: self.relation,
                    data: PhantomData,
                },
                UpdateLinkData {
                    wheres: (),
                    update_values: (),
                    pre_op: self.data,
                    post_op: (),
                },
            )
        }
    }

    impl<Key, From, To> UpdateLink for SetNew<OneToMany<Key, From, To>, PhantomData<To::InputData>>
    where
        To: Clone + Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        To::InputData: Clone,
        To::OutputData: Clone,
        OneToMany<Key, From, To>: Clone,
    {
        type InitSplitForPreOp = To::InputData;
        type PreOpSplitWheres = ();
        type PreOpSplitValues = <To::Id as CollectionId>::IdData;
        type PreOpSplitPostOp = ();
        type PreOpSplitTake = To::OutputData;

        // AbortOperation because there is no links that result in constraint violation
        type PreOp = InsertOne<To, InsertEntity<To::InputData, ()>, AbortOperation>;

        fn pre_op(&self, init_split_for_pre_op: Self::InitSplitForPreOp) -> Self::PreOp {
            InsertOne {
                handler: self.relation.to.clone(),
                data: InsertEntity {
                    attributes: init_split_for_pre_op,
                    link: (),
                },
                infalibility: AbortOperation,
            }
        }

        fn split_pre_op(
            &self,
            pre_op: LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>,
        ) -> Result<
            (
                Self::PreOpSplitWheres,
                Self::PreOpSplitValues,
                Self::PreOpSplitPostOp,
                Self::PreOpSplitTake,
            ),
            ConstraintViolation,
        > {
            let out = pre_op;
            Ok(((), out.id, (), out.attributes))
        }

        type InitSplitForWheres = ();

        type UpdateWhere = ();

        fn wheres(&self, _: Self::InitSplitForWheres) -> Self::UpdateWhere {}

        type UpdateReturning = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn update_names(&self) -> Self::UpdateReturning {
            Sanitize(conventional_foreign_key_name(
                self.relation.fk_unique_id.clone(),
                &self.relation.to,
            ))
        }

        type InitSplitForUpdateValues = ();

        type UpdateSets = OneToManySet<Key, To, Bind<<To::Id as CollectionId>::IdData>>;
        // type UpdateSets = UpdatingColumn<
        //     AsIdentifier<OneToMany<Key, From, To>>,
        //     Option<<To::Id as CollectionId>::IdData>,
        // >;

        fn update_values(&self, _: (), pre_op_output: Self::PreOpSplitValues) -> Self::UpdateSets {
            OneToManySet {
                key: self.relation.fk_unique_id.clone(),
                to: self.relation.to.clone(),
                bind: Bind(pre_op_output),
            }
        }

        type FromRow = NamedColFromRow<
            Sanitize<ConventionalForeignKeyName<Key, To>>,
            <To::Id as CollectionId>::IdData,
        >;

        fn from_row(&self) -> Self::FromRow {
            NamedColFromRow {
                name: Sanitize(conventional_foreign_key_name(
                    self.relation.fk_unique_id.clone(),
                    &self.relation.to,
                )),
                ty: PhantomData,
            }
        }

        type PostOp = ();

        type InitSplitPostOp = ();

        fn post_op(&self, _: Self::InitSplitPostOp, _: Self::PreOpSplitPostOp) -> Self::PostOp {
            ()
        }

        fn from_row_result(
            &self,
            _: &<Self::FromRow as crate::from_row::FromRowData>::RData,
            _: &mut Self::PostOp,
        ) {
        }

        type Output = CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>;

        type PostOpOutput = ();

        fn post_op_output(
            &self,
            _: <Self::PostOp as crate::operations::OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(())
        }

        fn take(
            &self,
            from_row: <Self::FromRow as crate::from_row::FromRowData>::RData,
            _: &mut Self::PostOpOutput,
            pre_op_split_take: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            CollectionOutput {
                id: from_row,
                attributes: pre_op_split_take.clone(),
            }
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory, links::{DefaultRelationKey, relation_one_to_many::OneToMany, update_links::SetNew}, operations::{CollectionOutput, LinkedOutput, Operation, insert::AbortOperation, update::Update}, sqlx_query_builder::basic_expressions::{Bind, ColumnEqual}, test_module::{self, Category, CategoryHandler, Todo}, track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn set_new_for_update() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE Category (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL
                    );
                    CREATE TABLE Todo (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL,
                        done BOOLEAN NOT NULL,
                        description TEXT,
                        fk_category_def INTEGER,
                        FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                    );

                    INSERT INTO Todo (title, done, description, fk_category_def) VALUES
                        ('todo_1', false, NULL, NULL);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    Update {
                        base: test_module::TodoHandler,
                        partial: Default::default(),
                        wheres: ColumnEqual {
                            col: "id",
                            eq: Bind(1),
                        },
                        links: SetNew {
                            relation: OneToMany {
                                fk_unique_id: DefaultRelationKey,
                                from: test_module::TodoHandler,
                                to: CategoryHandler,
                            },
                            data: Category {
                                title: "category_1".to_string(),
                            },
                        },
                        infalibility: AbortOperation,
                    },
                    &mut conn,
                )
                .await
                ;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";"#
                            .to_string(),
                        r#"UPDATE "Todo" SET "fk_category_def" = $1 WHERE "id" = $2 RETURNING "id", "title", "done", "description", "fk_category_def";"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    vec![LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "todo_1".to_string(),
                            done: false,
                            description: None,
                        },
                        links: CollectionOutput {
                            id: 1,
                            attributes: Category {
                                title: "category_1".to_string(),
                            },
                        },
                    }]
                );
            })
            .await;
        }
    }
}

mod impl_for_delete {
    use std::marker::PhantomData;

    use super::OneToMany;
    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        from_row::named_col_from_row::NamedColFromRow,
        links::utils::{ConventionalForeignKeyName, conventional_foreign_key_name},
        operations::{
            CollectionOutput, LinkedOutput,
            delete::{DeleteLink, DeleteLinkData, DeleteLinkPreOp, DeleteLinkSplit},
            fetch_many::LinkFetch,
            fetch_one::FetchOne,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
        },
        sqlx_query_builder::sanitize_combinator::Sanitize,
    };

    impl<Key, From, To> DeleteLinkSplit for OneToMany<Key, From, To>
    where
        Self: Clone,
        From: Collection,
        To: Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: Clone,
    {
        type Link = OneToMany<Key, From, To>;
        type InitSplitForPreOp = ();
        fn init_split(self) -> (Self::Link, Self::InitSplitForPreOp, DeleteLinkData<()>) {
            (self, (), DeleteLinkData { wheres: () })
        }
    }

    impl<Wheres, Key, From, To> DeleteLinkPreOp<Wheres> for OneToMany<Key, From, To>
    where
        Self: Clone,
        Key: Clone + AsRef<str>,
        To: TableExpressions,
        From: Collection,
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + TableExpressions + Clone,
        From: TableExpressions + Clone,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: Clone,
        Wheres: Clone,
        OneToMany<Key, From, To>: LinkFetch<
            Output = Option<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>,
        >,
    {
        type InitSplitForPreOp = ();

        type PreOp = FetchOne<From, OneToMany<Key, From, To>, Wheres>;

        fn pre_op(&self, _: Self::InitSplitForPreOp, wheres: &Wheres) -> Self::PreOp {
            FetchOne {
                base: self.from.clone(),
                links: self.clone(),
                wheres: wheres.clone(),
            }
        }
    }

    impl<Key, From, To> DeleteLink for OneToMany<Key, From, To>
    where
        Self: Clone,
        From: Collection,
        To: Collection + TableExpressions,
        Key: Clone + AsRef<str>,
        To::OutputData: Clone,
        <To::Id as CollectionId>::IdData: Clone,
    {
        type Output = Option<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        type PreOpOutput = Option<
            LinkedOutput<
                <From::Id as CollectionId>::IdData,
                From::OutputData,
                Option<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>,
            >,
        >;

        type PreOpSplitWheres = ();

        type PreOpSplitTake =
            Option<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        fn split_pre_op(
            &self,
            pre_op: Self::PreOpOutput,
        ) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake) {
            ((), pre_op.and_then(|linked| linked.links))
        }

        type InitSplitForWheres = ();

        type Wheres = ();

        fn wheres(&self, _: Self::InitSplitForWheres, _: Self::PreOpSplitWheres) -> Self::Wheres {}

        type DeleteReturnExpression = Sanitize<ConventionalForeignKeyName<Key, To>>;

        fn delete_return_expression(&self) -> Self::DeleteReturnExpression {
            Sanitize(conventional_foreign_key_name(
                self.fk_unique_id.clone(),
                &self.to,
            ))
        }

        type DeleteReturnFromRow = NamedColFromRow<
            Sanitize<ConventionalForeignKeyName<Key, To>>,
            Option<<To::Id as CollectionId>::IdData>,
        >;

        fn from_row(&self) -> Self::DeleteReturnFromRow {
            NamedColFromRow {
                name: Sanitize(conventional_foreign_key_name(
                    self.fk_unique_id.clone(),
                    &self.to,
                )),
                ty: PhantomData,
            }
        }

        fn take_mut(
            &self,
            _: <Self::DeleteReturnFromRow as crate::from_row::FromRowData>::RData,
            pre_op_split_take: &mut Self::PreOpSplitTake,
        ) -> Self::Output {
            pre_op_split_take.take()
        }

        fn take_once(
            &self,
            _: <Self::DeleteReturnFromRow as crate::from_row::FromRowData>::RData,
            pre_op_split_take: Self::PreOpSplitTake,
        ) -> Self::Output {
            pre_op_split_take
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{DefaultRelationKey, relation_one_to_many::OneToMany},
            operations::{CollectionOutput, LinkedOutput, Operation, delete::Delete},
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual, ScopedColumn},
            test_module::{self, Category, CategoryHandler, Todo},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn test_delete_link() {
            watch_sqlx_calls(async |actions| {
                let mut pool = Sqlite::in_memory_connection().await;

                sqlx::query(
                    "
                CREATE TABLE IF NOT EXISTS Category (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT,
                    fk_category_def INTEGER,
                    FOREIGN KEY (fk_category_def) REFERENCES Category(id)
                );
                INSERT INTO Category (title) VALUES ('category_1');
                INSERT INTO Todo (title, done, description, fk_category_def) VALUES ('todo_1', true, 'description_1', 1);
                ",
                )
                .execute(&mut pool)
                .await
                .unwrap();
                actions.clear();

                let s = Delete {
                    base: test_module::TodoHandler,
                    wheres: ColumnEqual {
                        col: ScopedColumn {
                            table: "Todo",
                            col: "id",
                        },
                        eq: Bind(1),
                    },
                    links: OneToMany {
                        fk_unique_id: DefaultRelationKey,
                        from: test_module::TodoHandler,
                        to: CategoryHandler,
                    },
                };

                let result = Operation::<Sqlite>::exec_operation(s, &mut pool).await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription", "Category"."id" AS "lid", "Category"."title" AS "ltitle" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" WHERE "Todo"."id" = $1;"#
                            .to_string(),
                        r#"DELETE FROM "Todo" WHERE "Todo"."id" = $1 RETURNING "id", "title", "done", "description", "fk_category_def";"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    result,
                    vec![LinkedOutput {
                        id: 1,
                        attributes: Todo {
                            title: "todo_1".to_string(),
                            done: true,
                            description: Some("description_1".to_string()),
                        },
                        links: Some(CollectionOutput {
                            id: 1,
                            attributes: Category {
                                title: "category_1".to_string(),
                            },
                        }),
                    },]
                );
            })
            .await;
        }
    }
}
