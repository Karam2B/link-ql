use crate::gen_serde::ObjectEncoding;

#[derive(Debug, Clone)]
pub struct Timestamp<C> {
    pub collection: C,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimestampOutput {
    pub created_at: String,
    pub updated_at: String,
}

impl<S> crate::gen_serde::Serialize<S> for TimestampOutput
where
    S: ObjectEncoding,
    String: crate::gen_serde::Serialize<S>,
    str: crate::gen_serde::Serialize<S>,
{
    fn serialize(&self, ctx: &mut S) {
        let mut c = ctx.serialize_start();
        ctx.serialize_pair(&mut c, "created_at", &self.created_at);
        ctx.serialize_pair(&mut c, "updated_at", &self.updated_at);
        ctx.serialize_end(c);
    }
}

mod impl_on_migrate {
    use std::marker::PhantomData;

    use crate::{
        links::timestamp::Timestamp,
        operations::operations_expressions_crossover::{MigrateExpression, TableExpressions},
        sqlx_query_builder::{
            basic_expressions::{
                CurrentTimestamp, DefaultExpression, ManyStatmenets, SetExpression, TypeAsSyntax,
                sub_query_expressions::{ColumnMatchNew, ColumnNewEqualsOld},
            },
            sanitize_combinator::Sanitize,
            statements::{
                add_column_statement::AddColumn,
                create_table_statement::{ColumnDefinition, NotNull},
                create_trigger::{CreateTrigger, TriggerLifetimeAfter, TriggerOperationNameUpdate},
                update_statement::UpdateStatement,
            },
        },
    };

    type TimestampAddColumn<C> = AddColumn<
        <C as TableExpressions>::PascalCase,
        ColumnDefinition<(
            &'static str,
            TypeAsSyntax<String>,
            NotNull,
            DefaultExpression<CurrentTimestamp>,
        )>,
    >;

    type TimestampCreateTrigger<C> = CreateTrigger<
        Sanitize<(<C as TableExpressions>::SnakeCase, &'static str)>,
        TriggerLifetimeAfter,
        TriggerOperationNameUpdate<()>,
        <C as TableExpressions>::PascalCase,
        ColumnNewEqualsOld<&'static str>,
        (UpdateStatement<
            <C as TableExpressions>::PascalCase,
            (SetExpression<&'static str, CurrentTimestamp>,),
            (ColumnMatchNew<&'static str>,),
            (),
        >,),
    >;

    impl<C> MigrateExpression for Timestamp<C>
    where
        C: TableExpressions,
    {
        type Migrate = ManyStatmenets<(
            TimestampAddColumn<C>,     // updated_at
            TimestampAddColumn<C>,     // created_at
            TimestampCreateTrigger<C>, // trigger of updated_at
        )>;

        fn migrate(&self) -> Self::Migrate {
            let update_at: TimestampAddColumn<C> = AddColumn {
                table: self.collection.table_name_pascal_case(),
                col_def: ColumnDefinition((
                    "updated_at",
                    TypeAsSyntax(PhantomData::<String>),
                    NotNull,
                    DefaultExpression {
                        value: CurrentTimestamp,
                    },
                )),
            };

            let created_at: TimestampAddColumn<C> = AddColumn {
                table: self.collection.table_name_pascal_case(),
                col_def: ColumnDefinition((
                    "created_at",
                    TypeAsSyntax(PhantomData::<String>),
                    NotNull,
                    DefaultExpression {
                        value: CurrentTimestamp,
                    },
                )),
            };

            let trigger: TimestampCreateTrigger<C> = CreateTrigger {
                trigger_name: Sanitize((
                    self.collection.table_name_snake_case(),
                    "_updated_at_trigger",
                )),
                temp: false,
                if_not_exists: false,
                lifetime: TriggerLifetimeAfter,
                operation_name: TriggerOperationNameUpdate(()),
                on_table: self.collection.table_name_pascal_case(),
                for_each_row: true,
                when_expression: ColumnNewEqualsOld("updated_at"),
                statements: (UpdateStatement {
                    table_name: self.collection.table_name_pascal_case(),
                    values: (SetExpression {
                        name: "updated_at",
                        value: CurrentTimestamp,
                    },),
                    wheres: (ColumnMatchNew("id"),),
                    returning: (),
                },),
            };

            ManyStatmenets((update_at, created_at, trigger))
        }
    }

    #[cfg(test)]
    mod test {

        use crate::{
            links::timestamp::Timestamp,
            operations::operations_expressions_crossover::MigrateExpression,
            sqlx_query_builder::StatementBuilder, test_module::TodoHandler,
        };

        #[test]
        fn on_migrate() {
            let sl = StatementBuilder::new_no_data(
                Timestamp {
                    collection: TodoHandler,
                }
                .migrate(),
            )
            .unwrap();

            pretty_assertions::assert_eq!(
                sl,
                r#"ALTER TABLE "Todo" ADD COLUMN "updated_at" TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP;ALTER TABLE "Todo" ADD COLUMN "created_at" TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP;CREATE TRIGGER "todo_updated_at_trigger" AFTER UPDATE ON "Todo" FOR EACH ROW  WHEN NEW."updated_at" = OLD."updated_at" BEGIN UPDATE "Todo" SET "updated_at" = CURRENT_TIMESTAMP WHERE "id" = NEW."id"; END;"#
            );
        }
    }
}

mod timestamp_select_items {
    use sqlx::{ColumnIndex, Decode, Row, Type};

    use crate::{
        from_row::{FromRowAlias, FromRowData},
        links::timestamp::TimestampOutput,
        operations::operations_expressions_crossover::TableExpressions,
    };
    use crate::{
        operations::operations_expressions_crossover::ExpressionsForOperation,
        sqlx_query_builder::{
            basic_expressions::{AliasedScopedColumn, ScopedColumn},
            sanitize_combinator::Sanitize,
        },
    };

    #[derive(Debug, Clone)]
    pub struct TimestampSelectItems<TableName>(pub TableName);

    impl<C> ExpressionsForOperation for TimestampSelectItems<C>
    where
        C: TableExpressions,
    {
        type Identifier = [&'static str; 2];

        fn identifier(&self) -> Self::Identifier {
            ["created_at", "updated_at"]
        }

        type Scoped = [ScopedColumn<C::PascalCase, &'static str>; 2];

        fn scoped(&self) -> Self::Scoped {
            [
                ScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    col: "created_at",
                },
                ScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    col: "updated_at",
                },
            ]
        }

        type ScopedAliased = [AliasedScopedColumn<
            C::PascalCase,
            &'static str,
            Sanitize<(&'static str, &'static str)>,
        >; 2];

        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            [
                AliasedScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    column: "created_at",
                    alias: Sanitize((alias, "created_at")),
                },
                AliasedScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    column: "updated_at",
                    alias: Sanitize((alias, "updated_at")),
                },
            ]
        }

        type NumScopedAliased = [AliasedScopedColumn<
            C::PascalCase,
            &'static str,
            Sanitize<(&'static str, usize, &'static str)>,
        >; 2];

        fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
            [
                AliasedScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    column: "created_at",
                    alias: Sanitize((alias, num, "created_at")),
                },
                AliasedScopedColumn {
                    table: self.0.table_name_pascal_case(),
                    column: "updated_at",
                    alias: Sanitize((alias, num, "updated_at")),
                },
            ]
        }
    }

    macro_rules! impl_timestamp {
        ($($table:ty),*) => {
            $(
        impl ExpressionsForOperation for TimestampSelectItems<$table> {
            type Identifier = [&'static str; 2];

            fn identifier(&self) -> Self::Identifier {
                ["created_at", "updated_at"]
            }

            type Scoped = [ScopedColumn<$table, &'static str>; 2];

            fn scoped(&self) -> Self::Scoped {
                [
                    ScopedColumn {
                        table: self.0.clone(),
                        col: "created_at",
                    },
                    ScopedColumn {
                        table: self.0.clone(),
                        col: "updated_at",
                    },
                ]
            }

            type ScopedAliased =
                [AliasedScopedColumn<$table, &'static str, Sanitize<(&'static str, &'static str)>>; 2];

            fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                [
                    AliasedScopedColumn {
                        table: self.0.clone(),
                        column: "created_at",
                        alias: Sanitize((alias, "created_at")),
                    },
                    AliasedScopedColumn {
                        table: self.0.clone(),
                        column: "updated_at",
                        alias: Sanitize((alias, "updated_at")),
                    },
                ]
            }

            type NumScopedAliased = [AliasedScopedColumn<
                $table,
                &'static str,
                Sanitize<(&'static str, usize, &'static str)>,
            >; 2];

            fn num_scoped_aliased(
                &self,
                num: usize,
                alias: &'static str,
            ) -> Self::NumScopedAliased {
                [
                    AliasedScopedColumn {
                        table: self.0.clone(),
                        column: "created_at",
                        alias: Sanitize((alias, num, "created_at")),
                    },
                    AliasedScopedColumn {
                        table: self.0.clone(),
                        column: "updated_at",
                        alias: Sanitize((alias, num, "updated_at")),
                    },
                ]
            }
        }
            )*
        };
    }

    impl<T> FromRowData for TimestampSelectItems<T> {
        type RData = TimestampOutput;
    }

    impl<'r, R, T> FromRowAlias<'r, R> for TimestampSelectItems<T>
    where
        R: Row,
        for<'s> &'s str: ColumnIndex<R>,
        String: for<'d> Decode<'d, R::Database> + Type<R::Database>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, crate::from_row::FromRowError> {
            Ok(TimestampOutput {
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            Ok(TimestampOutput {
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            Ok(TimestampOutput {
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
        }
    }

    impl_timestamp!(&'static str, String, std::sync::Arc<str>);
}

pub mod impl_fetch_many {
    use crate::{
        from_row::FromRowData,
        links::timestamp::{
            Timestamp, TimestampOutput, timestamp_select_items::TimestampSelectItems,
        },
        operations::{fetch_many::LinkFetch, operations_expressions_crossover::TableExpressions},
    };

    impl<C> LinkFetch for Timestamp<C>
    where
        TimestampSelectItems<C::PascalCase>: FromRowData<RData = TimestampOutput>,
        C: TableExpressions,
        // C: TableExpressions,
    {
        type Output = TimestampOutput;

        type SelectItems = TimestampSelectItems<C::PascalCase>;

        fn non_aggregating_select_items(&self) -> Self::SelectItems {
            TimestampSelectItems(self.collection.table_name_pascal_case())
        }

        type Join = ();

        fn non_duplicating_join_expressions(&self) -> Self::Join {}

        type Wheres = ();

        fn where_expressions(&self) -> Self::Wheres {}

        type OpInput = ();

        fn operation_initialize_input(&self) -> Self::OpInput {}

        type Op = ();

        fn operation_construct(&self, _: Self::OpInput) -> Self::Op
        where
            Self::SelectItems: crate::from_row::FromRowData,
        {
        }

        fn operation_fix_on_many(
            &self,
            _: &<Self::SelectItems as crate::from_row::FromRowData>::RData,
            _: &mut Self::OpInput,
        ) where
            Self::SelectItems: crate::from_row::FromRowData,
        {
        }

        fn take_many(
            &self,
            item: <Self::SelectItems as crate::from_row::FromRowData>::RData,
            _: &mut <Self::Op as crate::operations::OperationOutput>::Output,
        ) -> Self::Output
        where
            Self::SelectItems: crate::from_row::FromRowData,
            Self::Op: crate::operations::OperationOutput,
        {
            item
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::{Sqlite, query};

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::timestamp::Timestamp,
            operations::{Operation, fetch_many::FetchMany},
            test_module::{TodoHandler, todo_members},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn fetch_many() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                query(
                    r#"
                    CREATE TABLE "Todo" (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL,
                        done BOOLEAN NOT NULL,
                        description TEXT,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );

                    INSERT INTO "Todo" (title, done, description, created_at, updated_at) VALUES
                        ('todo_1', false, 'description_1', 'test_created', 'test_updated');
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                Operation::<Sqlite>::exec_operation(
                    FetchMany {
                        base: TodoHandler,
                        wheres: (),
                        links: Timestamp {
                            collection: TodoHandler,
                        },
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
                        r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription", "Todo"."created_at" AS "lcreated_at", "Todo"."updated_at" AS "lupdated_at" FROM "Todo" ORDER BY "Todo"."title" LIMIT $1;"#
                            .to_string(),
                    ]
                );
            })
            .await;
        }
    }
}

mod impl_on_insert {
    use crate::{
        links::timestamp::{
            Timestamp, TimestampOutput, timestamp_select_items::TimestampSelectItems,
        },
        operations::insert::{ConstraintViolation, InsertLinkConsumeData, InsertLinkData, InsertOneLink},
    };

    impl<C: Clone + crate::operations::operations_expressions_crossover::TableExpressions>
        InsertLinkConsumeData for Timestamp<C>
    {
        type Link = Timestamp<C>;

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
                self,
                InsertLinkData {
                    insert_value_data: (),
                    pre_op_data: (),
                    post_op_data: (),
                },
            )
        }
    }

    impl<C: Clone + crate::operations::operations_expressions_crossover::TableExpressions>
        InsertOneLink for Timestamp<C>
    {
        type PreOp = ();

        type PreOpData = ();

        fn pre_operation_init(&self, _: Self::PreOpData) -> Self::PreOp {}

        fn pre_op_split(
            &self,
            _: <Self::PreOp as crate::operations::OperationOutput>::Output,
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

        type InsertReturning = TimestampSelectItems<C>;

        fn insert_returning(&self) -> Self::InsertReturning {
            TimestampSelectItems(self.collection.clone())
        }

        type InsertValuesData = ();

        type InsertSets = ();

        fn insert_value(
            &self,
            _: Self::InsertValuesData,
            _: Self::PreOpToInsertValue,
        ) -> Self::InsertSets {
        }

        type FromRow = TimestampSelectItems<C::PascalCase>;

        fn from_row(&self) -> Self::FromRow {
            TimestampSelectItems(self.collection.table_name_pascal_case())
        }

        type TakeInput = TimestampOutput;

        type PostOp = ();

        type PostOpData = ();

        fn from_row_result(
            &self,
            _: Self::PostOpData,
            out: <Self::FromRow as crate::from_row::FromRowData>::RData,
            _: Self::PreOpToPostOp,
        ) -> (Self::PostOp, Self::TakeInput) {
            ((), out)
        }

        type PostOpOutput = ();

        fn post_op_output(
            &self,
            _: <Self::PostOp as crate::operations::OperationOutput>::Output,
        ) -> Result<Self::PostOpOutput, ConstraintViolation> {
            Ok(())
        }

        type Output = TimestampOutput;

        fn take(
            self,
            _: Self::PostOpOutput,
            out: Self::TakeInput,
            _: Self::PreOpToTake,
        ) -> Self::Output {
            out
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::{Sqlite, query};

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::timestamp::Timestamp,
            operations::{
                Operation,
                insert::{AbortOperation, InsertEntity, InsertOne},
            },
            test_module::{Todo, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn insert_one() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                query(
                    r#"
                    CREATE TABLE "Todo" (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        title TEXT NOT NULL,
                        done BOOLEAN NOT NULL,
                        description TEXT,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                Operation::<Sqlite>::exec_operation(
                    InsertOne {
                        handler: TodoHandler,
                        data: InsertEntity {
                            attributes: Todo {
                            title: String::from("todo"),
                            done: false,
                            description: None,
                        },
                        link: Timestamp {
                            collection: TodoHandler,
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
                        r#"INSERT INTO "Todo" ("title", "done", "description") VALUES ($1, $2, $3) RETURNING "id", "title", "done", "description", "created_at", "updated_at";"#
                            .to_string(),
                    ]
                );
            })
            .await;
        }
    }
}
