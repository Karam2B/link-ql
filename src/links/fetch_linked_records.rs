//! In this module there are operations that are important
//! for implementing both many_to_many and one_to_many_inverse links

//! I tried to reuse code from operations::* but I failed for the following
//! reasons:
//!     1. to implement Collection::table_name for ConjuctionTable
//!         you need to return a &str which is not possible because the
//!         name is composed from 2 tables
//!     2. To FetchMany<ConjunctionTable<Key, From, To>, _> I don't want
//!         to simply return the two ids in the table,
//!         but attributes of one of the linked tables
//!     2a. creating new link to only be used in new collection -- that is only used in implementing a link
//!           seems crazy and unnecessary
//!     2a. designing ConjuctionTable to automatically fetch attributes from one of the tables
//!           seems like violation of "Least Surprise Principle"
//!     3. ConjuctionTable is the only use case I can think of where I
//!         have 2 fields as an id, which if does not need to implment
//!         `Collection` trait would mean that `Collection` trait is
//!         over-engineered, and need to be simplified
//!     4. I still can work on optimizing this module if ConjuctionTable is dead end
//!
use std::collections::HashMap;

use sqlx::{Decode, Row, Type};

use crate::{
    collections::Collection, database_extention::DatabaseExt, from_row::FromRowAlias,
    operations::CollectionOutput,
};

pub type LinkedRecordsMap<ParentId, ChildId, ChildOutput> =
    HashMap<ParentId, Vec<CollectionOutput<ChildId, ChildOutput>>>;

pub type ManyToManyLinkedMap<FromId, ToId, ToOutput> = LinkedRecordsMap<FromId, ToId, ToOutput>;

pub type OneToManyInverseLinkedMap<FromId, ToId, ToOutput> =
    LinkedRecordsMap<FromId, ToId, ToOutput>;

fn rows_to_linked_map<S, FromId, ToId, ToOutput, To>(
    rows: Vec<S::Row>,
    to: To,
    to_id: To::Id,
) -> LinkedRecordsMap<FromId, ToId, ToOutput>
where
    S: DatabaseExt,
    FromId: Copy + Clone + std::hash::Hash + Eq + for<'r> Decode<'r, S> + Type<S>,
    To: Collection,
    To::OutputData: Send,
    ToId: Send,
    To: for<'r> FromRowAlias<'r, S::Row, RData = ToOutput>,
    To::Id: for<'r> FromRowAlias<'r, S::Row, RData = ToId>,
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    let mut map = HashMap::new();

    for row in rows {
        let from_id = row.try_get::<FromId, _>("from_id").unwrap();
        let id = to_id.no_alias(&row).unwrap();
        let attributes = to.no_alias(&row).unwrap();
        map.entry(from_id)
            .or_insert_with(Vec::new)
            .push(CollectionOutput { id, attributes });
    }

    map
}

pub trait ManyToManyJunctionNames {
    fn junction_table_as_str(&self) -> String;
    fn from_junction_col_as_str(&self) -> String;
    fn to_junction_col_as_str(&self) -> String;
}

mod impl_many_to_many_junction_names {
    use crate::{
        collections::{Collection, SingleColumnId},
        links::{fetch_linked_records::ManyToManyJunctionNames, relation_many_to_many::ManyToMany},
    };

    impl<const INVERSE: bool, Key, From, To> ManyToManyJunctionNames
        for ManyToMany<INVERSE, Key, From, To>
    where
        Key: Clone + AsRef<str>,
        From: Collection<Id: SingleColumnId> + Clone,
        To: Collection<Id: SingleColumnId> + Clone,
    {
        fn junction_table_as_str(&self) -> String {
            return format!(
                "ct_{}_{}{}",
                self.from.table_name_lower_case(),
                self.to.table_name_lower_case(),
                self.relation_key.as_ref()
            );
        }

        fn from_junction_col_as_str(&self) -> String {
            return format!("{}_id", self.from.table_name_lower_case());
        }

        fn to_junction_col_as_str(&self) -> String {
            return format!("{}_id", self.to.table_name_lower_case());
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{
            links::{
                DefaultRelationKey, fetch_linked_records::ManyToManyJunctionNames,
                relation_many_to_many::ManyToMany,
            },
            test_module::{TagHandler, TodoHandler},
        };

        #[test]
        fn junction_names_for_todo_tag() {
            let link = ManyToMany::<false, _, _, _> {
                relation_key: DefaultRelationKey,
                from: TodoHandler,
                to: TagHandler,
            };

            assert_eq!(link.junction_table_as_str(), "ct_todo_tag_def");
            assert_eq!(link.from_junction_col_as_str(), "todo_id");
            assert_eq!(link.to_junction_col_as_str(), "tag_id");
        }
    }
}

mod impl_insert_junction_row {
    use sqlx::{Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        links::{fetch_linked_records::ManyToManyJunctionNames, relation_many_to_many::ManyToMany},
        operations::Operation,
        sqlx_query_builder::{
            Join, StatementBuilder,
            basic_expressions::Bind,
            statements::insert_statement::{InsertStatement, One},
        },
    };

    #[derive(Clone)]
    pub struct InsertJunctionRow<Key, From, To> {
        link: ManyToMany<false, Key, From, To>,
        from_id: i64,
        to_id: i64,
    }

    impl<Key, From, To> InsertJunctionRow<Key, From, To> {
        pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
            Self {
                link,
                from_id,
                to_id,
            }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput for InsertJunctionRow<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        type Output = ();
    }

    impl<S, Key, From, To> Operation<S> for InsertJunctionRow<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId> + Clone + Send,
        <From::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        To: Collection<Id: SingleColumnId> + Clone + Send,
        <To::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        i64: for<'q> Encode<'q, S> + Type<S> + Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            let junction = self.link.junction_table_as_str();
            let from_col = self.link.from_junction_col_as_str();
            let to_col = self.link.to_junction_col_as_str();

            let (stmt, args) = StatementBuilder::<'_, S>::new(InsertStatement {
                table_name: junction,
                identifiers: Join {
                    start: "",
                    separator: ", ",
                    items: (from_col.as_str(), to_col.as_str()),
                },
                values: One(Join {
                    start: "",
                    separator: ", ",
                    items: (Bind(self.from_id), Bind(self.to_id)),
                }),
                returning: (),
            })
            .unwrap();

            S::execute(
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

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey, fetch_linked_records::InsertJunctionRow,
                relation_many_to_many::ManyToMany,
            },
            operations::Operation,
            test_module::{TagHandler, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn inserts_junction_row() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo', false, NULL);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                Operation::<Sqlite>::exec_operation(
                    InsertJunctionRow::new(
                        ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: TodoHandler,
                            to: TagHandler,
                        },
                        1,
                        1,
                    ),
                    &mut conn,
                )
                .await;

                let count: (i64,) = sqlx::query_as(
                    r#"SELECT COUNT(*) FROM "ct_todo_tag_def" WHERE "todo_id" = 1 AND "tag_id" = 1"#,
                )
                .fetch_one(&mut conn)
                .await
                .unwrap();
                assert_eq!(count.0, 1);
            })
            .await;
        }
    }
}

mod impl_delete_junction_row {
    use sqlx::{Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        links::{fetch_linked_records::ManyToManyJunctionNames, relation_many_to_many::ManyToMany},
        operations::Operation,
        sqlx_query_builder::{
            StatementBuilder,
            basic_expressions::{Bind, ColumnEqual, ScopedColumn},
            statements::delete_statement::DeleteStatement,
        },
    };

    #[derive(Clone)]
    pub struct DeleteJunctionRow<Key, From, To> {
        link: ManyToMany<false, Key, From, To>,
        from_id: i64,
        to_id: i64,
    }

    impl<Key, From, To> DeleteJunctionRow<Key, From, To> {
        pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
            Self {
                link,
                from_id,
                to_id,
            }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput for DeleteJunctionRow<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        type Output = ();
    }

    impl<S, Key, From, To> Operation<S> for DeleteJunctionRow<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId> + Clone + Send,
        <From::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        To: Collection<Id: SingleColumnId> + Clone + Send,
        <To::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
        i64: for<'q> Encode<'q, S> + Type<S> + Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            let junction = self.link.junction_table_as_str();
            let from_col = self.link.from_junction_col_as_str();
            let to_col = self.link.to_junction_col_as_str();

            let (stmt, args) = StatementBuilder::<'_, S>::new(DeleteStatement {
                table_name: junction.as_str(),
                wheres: (
                    ColumnEqual {
                        col: ScopedColumn {
                            table: junction.as_str(),
                            col: from_col,
                        },
                        eq: Bind(self.from_id),
                    },
                    ColumnEqual {
                        col: ScopedColumn {
                            table: junction.as_str(),
                            col: to_col,
                        },
                        eq: Bind(self.to_id),
                    },
                ),
                returning: (),
            })
            .unwrap();

            S::execute(
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

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey, fetch_linked_records::DeleteJunctionRow,
                relation_many_to_many::ManyToMany,
            },
            operations::Operation,
            test_module::{TagHandler, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn deletes_junction_row() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent'), ('home');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo', false, NULL);
                    INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES (1, 1), (1, 2);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                Operation::<Sqlite>::exec_operation(
                    DeleteJunctionRow::new(
                        ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: TodoHandler,
                            to: TagHandler,
                        },
                        1,
                        1,
                    ),
                    &mut conn,
                )
                .await;

                let count: (i64,) = sqlx::query_as(
                    r#"SELECT COUNT(*) FROM "ct_todo_tag_def" WHERE "todo_id" = 1"#,
                )
                .fetch_one(&mut conn)
                .await
                .unwrap();
                assert_eq!(count.0, 1);
            })
            .await;
        }
    }
}

mod impl_insert_junction_and_fetch {
    use sqlx::{Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        fix_executor::ExecutorTrait,
        from_row::FromRowAlias,
        links::{fetch_linked_records::InsertJunctionRow, relation_many_to_many::ManyToMany},
        operations::{
            LinkedOutput, Operation,
            fetch_one::FetchOne,
            operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
        },
        sqlx_query_builder::{
            Expression,
            basic_expressions::{Bind, ColumnEqual},
        },
    };

    #[derive(Clone)]
    pub struct InsertJunctionAndFetch<Key, From, To> {
        link: ManyToMany<false, Key, From, To>,
        from_id: i64,
        to_id: i64,
    }

    impl<Key, From, To> InsertJunctionAndFetch<Key, From, To> {
        pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
            Self {
                link,
                from_id,
                to_id,
            }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput for InsertJunctionAndFetch<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        type Output = LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>;
    }

    impl<S, Key, From, To> Operation<S> for InsertJunctionAndFetch<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId>
            + TableExpressions<SnakeCase: AsRef<str>, PascalCase: AsRef<str>>
            + Clone
            + Send,
        <From::Id as CollectionId>::IdData:
            Send + for<'q> Encode<'q, S> + Type<S> + Copy + ::std::convert::From<i64>,
        To: Collection<Id: SingleColumnId>
            + TableExpressions<
                SnakeCase: AsRef<str>,
                ScopedAliased: crate::sqlx_query_builder::combinators::OptionalExpression,
                InheritJoin: crate::sqlx_query_builder::combinators::OptionalExpression,
            > + ExpressionsForOperation<
                ScopedAliased: crate::sqlx_query_builder::combinators::OptionalExpression,
            >
            + Clone
            + Send,
        <To as TableExpressions>::PascalCase: AsRef<str> + for<'q> Expression<'q, S>,
        for<'q> crate::sqlx_query_builder::Join<To::ScopedAliased>: Expression<'q, S>,
        for<'q> crate::sqlx_query_builder::Join<To::InheritJoin>: Expression<'q, S>,
        for<'q> crate::sqlx_query_builder::Join<
            <To as crate::operations::operations_expressions_crossover::ExpressionsForOperation>::ScopedAliased,
        >: Expression<'q, S>,
        for<'q> crate::sqlx_query_builder::Join<
            <To::Id as crate::operations::operations_expressions_crossover::ExpressionsForOperation>::ScopedAliased,
        >: Expression<'q, S>,
        <To::Id as CollectionId>::IdData:
            ::std::convert::From<i64> + Send + 'static + for<'q> Encode<'q, S> + Type<S> + Copy,
        To::Id: Send
            + ExpressionsForOperation<
                Identifier: for<'q> Expression<'q, S> + 'static,
                ScopedAliased: crate::sqlx_query_builder::combinators::OptionalExpression,
            > + for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
        To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
        To::OutputData: Send,
        ColumnEqual<
            <To::Id as ExpressionsForOperation>::Identifier,
            Bind<<To::Id as CollectionId>::IdData>,
        >: Send,
        i64: for<'q> Encode<'q, S> + Type<S> + Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            InsertJunctionRow::new(self.link.clone(), self.from_id, self.to_id)
                .exec_operation(&mut *pool)
                .await;

            let to_id = <To::Id as CollectionId>::IdData::from(self.to_id);
            FetchOne {
                base: self.link.to.clone(),
                links: (),
                wheres: ColumnEqual {
                    col: self.link.to.id().identifier(),
                    eq: Bind(to_id),
                },
            }
            .exec_operation(&mut *pool)
            .await
            .expect("linked row should exist")
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey, fetch_linked_records::InsertJunctionAndFetch,
                relation_many_to_many::ManyToMany,
            },
            operations::Operation,
            test_module::{Tag, TagHandler, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn inserts_junction_and_fetches_tag() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo', false, NULL);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let out = Operation::<Sqlite>::exec_operation(
                    InsertJunctionAndFetch::new(
                        ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: TodoHandler,
                            to: TagHandler,
                        },
                        1,
                        1,
                    ),
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES ($1, $2);"#
                            .to_string(),
                        r#"SELECT "Tag"."id" AS "iid", "Tag"."title" AS "btitle" FROM "Tag" WHERE "id" = $1;"#
                            .to_string(),
                    ]
                );

                assert_eq!(out.id, 1);
                assert_eq!(
                    out.attributes,
                    Tag {
                        title: "urgent".to_string(),
                    }
                );
            })
            .await;
        }
    }
}

mod impl_select_junction_to_ids {
    use sqlx::{Decode, Encode, Row, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        links::{fetch_linked_records::ManyToManyJunctionNames, relation_many_to_many::ManyToMany},
        operations::{Operation, operations_expressions_crossover::TableExpressions},
        sqlx_query_builder::{
            StatementBuilder,
            basic_expressions::{Bind, ColumnEqual, ScopedColumn},
            statements::select_statement::SelectStatement,
        },
    };

    #[derive(Clone)]
    #[allow(dead_code)]
    pub struct SelectJunctionToIds<Key, From, To> {
        link: ManyToMany<false, Key, From, To>,
        from_id: i64,
    }

    impl<Key, From, To> SelectJunctionToIds<Key, From, To> {
        pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64) -> Self {
            Self { link, from_id }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput for SelectJunctionToIds<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        type Output = Vec<i64>;
    }

    impl<S, Key, From, To> Operation<S> for SelectJunctionToIds<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId> + TableExpressions + Clone + Send,
        <From::Id as CollectionId>::IdData: for<'q> Encode<'q, S> + Type<S> + Send + Copy,
        To: Collection<Id: SingleColumnId> + TableExpressions + Clone + Send,
        <To::Id as CollectionId>::IdData:
            for<'r> Decode<'r, S> + for<'q> Encode<'q, S> + Type<S> + Send + Copy,
        i64: for<'r> Decode<'r, S> + for<'q> Encode<'q, S> + Type<S> + Send,
        for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            let junction = self.link.junction_table_as_str();
            let from_col = self.link.from_junction_col_as_str();
            let to_col = self.link.to_junction_col_as_str();

            let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
                from: junction.as_str(),
                select_items: (ScopedColumn {
                    table: junction.as_str(),
                    col: to_col.as_str(),
                },),
                joins: (),
                wheres: (ColumnEqual {
                    col: ScopedColumn {
                        table: junction.as_str(),
                        col: from_col,
                    },
                    eq: Bind(self.from_id),
                },),
                group_by: (),
                order: (),
                limit: (),
            })
            .unwrap();

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
                .map(|row| row.try_get::<i64, _>(to_col.as_str()).unwrap())
                .collect()
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{
                DefaultRelationKey, fetch_linked_records::SelectJunctionToIds,
                relation_many_to_many::ManyToMany,
            },
            operations::Operation,
            test_module::{TagHandler, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn selects_linked_tag_ids() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                sqlx::query(
                    r#"
                    CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);
                    CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "done" BOOLEAN NOT NULL, "description" TEXT);
                    CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));
                    INSERT INTO "Tag" ("title") VALUES ('urgent'), ('home');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES ('todo', false, NULL);
                    INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES (1, 1), (1, 2);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let ids = Operation::<Sqlite>::exec_operation(
                    SelectJunctionToIds::new(
                        ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: TodoHandler,
                            to: TagHandler,
                        },
                        1,
                    ),
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "ct_todo_tag_def"."tag_id" FROM "ct_todo_tag_def" WHERE "ct_todo_tag_def"."todo_id" = $1;"#
                            .to_string(),
                    ]
                );

                assert_eq!(ids, vec![1, 2]);
            })
            .await;
        }
    }
}

mod impl_fetch_one_to_many_inverse_linked {
    use std::collections::HashMap;

    use sqlx::{Decode, Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        from_row::FromRowAlias,
        links::{
            fetch_linked_records::OneToManyInverseLinkedMap,
            relation_one_to_many_inverse::OneToManyInverse,
        },
        operations::{
            Operation,
            operations_expressions_crossover::{ExpressionsForOperation, IdentifierColNames},
        },
        sqlx_query_builder::{
            Expression, OpExpression, StatementBuilder,
            basic_expressions::{Bind, ColumnIn, ScopedColumn},
            statements::select_statement::SelectStatement,
        },
    };

    use super::rows_to_linked_map;

    struct InverseLinkedSelect {
        fk_col: String,
        to_table: String,
        to_cols: Vec<String>,
    }

    impl OpExpression for InverseLinkedSelect {
}

    impl<'q, S> Expression<'q, S> for InverseLinkedSelect
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.sanitize(&self.to_table);
            ctx.syntax(".");
            ctx.sanitize(&self.fk_col);
            ctx.syntax(r#" AS "from_id", "#);
            ctx.sanitize(&self.to_table);
            ctx.syntax(".");
            ctx.sanitize("id");
            for col in &self.to_cols {
                ctx.syntax(", ");
                ctx.sanitize(&self.to_table);
                ctx.syntax(".");
                ctx.sanitize(col);
            }
        }
    }

    pub struct FetchOneToManyInverseLinked<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        pub link: OneToManyInverse<Key, From, To>,
        pub from_ids: Vec<<From::Id as CollectionId>::IdData>,
        fk_col: String,
        to_table: String,
        to_cols: Vec<String>,
    }

    impl<Key, From, To> Clone for FetchOneToManyInverseLinked<Key, From, To>
    where
        Key: Clone,
        From: Collection + Clone,
        To: Collection + Clone,
        <From::Id as CollectionId>::IdData: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                link: self.link.clone(),
                from_ids: self.from_ids.clone(),
                fk_col: self.fk_col.clone(),
                to_table: self.to_table.clone(),
                to_cols: self.to_cols.clone(),
            }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput
        for FetchOneToManyInverseLinked<Key, From, To>
    where
        From: Collection,
        To: Collection,
    {
        type Output = OneToManyInverseLinkedMap<
            <From::Id as CollectionId>::IdData,
            <To::Id as CollectionId>::IdData,
            To::OutputData,
        >;
    }

    impl<Key, From, To> FetchOneToManyInverseLinked<Key, From, To>
    where
        Key: Clone + AsRef<str>,
        From: Collection
            + crate::operations::operations_expressions_crossover::TableExpressions<
                SnakeCase: AsRef<str>,
            > + Clone,
        To: Collection
            + crate::operations::operations_expressions_crossover::TableExpressions<
                PascalCase: AsRef<str>,
            > + crate::operations::operations_expressions_crossover::ExpressionsForOperation<
                Identifier: crate::operations::operations_expressions_crossover::IdentifierColNames,
            > + Clone,
    {
        pub fn new(
            link: OneToManyInverse<Key, From, To>,
            from_ids: Vec<<From::Id as CollectionId>::IdData>,
        ) -> Self {
            Self {
                fk_col: format!(
                    "fk_{}{}",
                    link.from.table_name_snake_case().as_ref(),
                    link.fk_unique_id.as_ref(),
                ),
                to_table: link.to.table_name_pascal_case().as_ref().to_string(),
                to_cols: link.to.identifier().col_names(),
                link,
                from_ids,
            }
        }
    }

    impl<S, Key, From, To> Operation<S> for FetchOneToManyInverseLinked<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId> + Clone + Send,
        <From::Id as CollectionId>::IdData: Copy
            + Clone
            + std::hash::Hash
            + Eq
            + Send
            + for<'q> Encode<'q, S>
            + Type<S>
            + for<'r> Decode<'r, S>,
        To: Collection<Id: SingleColumnId + ExpressionsForOperation> + Clone + Send,
        To::OutputData: Send,
        <To::Id as CollectionId>::IdData: Send,
        To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
        To::Id: for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
        <To::Id as ExpressionsForOperation>::Identifier: for<'q> Expression<'q, S>,
        for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            if self.from_ids.is_empty() {
                return HashMap::new();
            }

            let to_table = self.to_table.clone();
            let fk_col = self.fk_col.clone();

            let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
                select_items: (InverseLinkedSelect {
                    fk_col: fk_col.clone(),
                    to_table: to_table.clone(),
                    to_cols: self.to_cols,
                },),
                from: to_table.clone(),
                joins: (),
                wheres: (ColumnIn {
                    col: ScopedColumn {
                        table: to_table.clone(),
                        col: fk_col.clone(),
                    },
                    values: self.from_ids.into_iter().map(Bind).collect::<Vec<_>>(),
                },),
                group_by: (),
                order: (),
                limit: (),
            })
            .unwrap();

            let rows = S::fetch_all(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: args,
                },
            )
            .await
            .unwrap();

            let to = self.link.to.clone();
            let to_id = to.id();
            rows_to_linked_map::<S, _, _, _, _>(rows, to, to_id)
        }
    }

    #[cfg(test)]
    mod test {
        use crate::{
            links::{
                DefaultRelationKey, fetch_linked_records::FetchOneToManyInverseLinked,
                relation_one_to_many_inverse::OneToManyInverse,
            },
            test_module::{CategoryHandler, TodoHandler},
        };

        #[test]
        fn new_builds_fetch_operation() {
            let op = FetchOneToManyInverseLinked::new(
                OneToManyInverse {
                    fk_unique_id: DefaultRelationKey,
                    from: CategoryHandler,
                    to: TodoHandler,
                },
                vec![1],
            );
            assert_eq!(op.from_ids, vec![1]);
        }
    }
}

mod impl_fetch_many_to_many_linked {
    use std::collections::HashMap;

    use sqlx::{Decode, Encode, Type};

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        database_extention::DatabaseExt,
        execute::Executable,
        fix_executor::ExecutorTrait,
        from_row::FromRowAlias,
        links::{fetch_linked_records::ManyToManyLinkedMap, relation_many_to_many::ManyToMany},
        operations::{
            Operation,
            operations_expressions_crossover::{
                ExpressionsForOperation, IdentifierColNames, TableExpressions,
            },
        },
        sqlx_query_builder::{
            Expression, OpExpression, StatementBuilder,
            basic_expressions::{Bind, ColumnIn, JoinExpression, ScopedColumn},
            statements::select_statement::SelectStatement,
        },
    };

    use super::rows_to_linked_map;

    struct ManyToManyLinkedSelect {
        junction_table: String,
        from_col: String,
        to_table: String,
        to_cols: Vec<String>,
    }

    impl OpExpression for ManyToManyLinkedSelect {
}

    impl<'q, S> Expression<'q, S> for ManyToManyLinkedSelect
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.sanitize(&self.junction_table);
            ctx.syntax(".");
            ctx.sanitize(&self.from_col);
            ctx.syntax(r#" AS "from_id", "#);
            ctx.sanitize(&self.to_table);
            ctx.syntax(".");
            ctx.sanitize("id");
            for col in &self.to_cols {
                ctx.syntax(", ");
                ctx.sanitize(&self.to_table);
                ctx.syntax(".");
                ctx.sanitize(col);
            }
        }
    }

    pub struct FetchManyToManyLinked<Key, From, To>
    where
        From: Collection,
        To: Collection + ExpressionsForOperation,
    {
        pub link: ManyToMany<false, Key, From, To>,
        pub from_ids: Vec<<From::Id as CollectionId>::IdData>,
        junction_table: String,
        from_col: String,
        to_col: String,
        to_table: String,
        to_cols: Vec<String>,
    }

    impl<Key, From, To> Clone for FetchManyToManyLinked<Key, From, To>
    where
        Key: Clone,
        From: Collection + Clone,
        To: Collection + ExpressionsForOperation + Clone,
        <From::Id as CollectionId>::IdData: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                link: self.link.clone(),
                from_ids: self.from_ids.clone(),
                junction_table: self.junction_table.clone(),
                from_col: self.from_col.clone(),
                to_col: self.to_col.clone(),
                to_table: self.to_table.clone(),
                to_cols: self.to_cols.clone(),
            }
        }
    }

    impl<Key, From, To> crate::operations::OperationOutput for FetchManyToManyLinked<Key, From, To>
    where
        From: Collection,
        To: Collection + ExpressionsForOperation,
    {
        type Output = ManyToManyLinkedMap<
            <From::Id as CollectionId>::IdData,
            <To::Id as CollectionId>::IdData,
            To::OutputData,
        >;
    }

    impl<Key, From, To> FetchManyToManyLinked<Key, From, To>
    where
        Key: Clone + AsRef<str>,
        From: Collection<Id: SingleColumnId>
            + crate::operations::operations_expressions_crossover::TableExpressions<
                SnakeCase: AsRef<str>,
            > + Clone,
        To: Collection<Id: SingleColumnId>
            + crate::operations::operations_expressions_crossover::TableExpressions<
                SnakeCase: AsRef<str>,
                PascalCase: AsRef<str>,
            > + crate::operations::operations_expressions_crossover::ExpressionsForOperation<
                Identifier: crate::operations::operations_expressions_crossover::IdentifierColNames,
            > + Clone,
    {
        pub fn new(
            link: ManyToMany<false, Key, From, To>,
            from_ids: Vec<<From::Id as CollectionId>::IdData>,
        ) -> Self {
            let junction_table = format!(
                "ct_{}_{}{}",
                link.from.table_name_snake_case().as_ref(),
                link.to.table_name_snake_case().as_ref(),
                link.relation_key.as_ref(),
            );

            Self {
                from_col: format!("{}_id", link.from.table_name_snake_case().as_ref()),
                to_col: format!("{}_id", link.to.table_name_snake_case().as_ref()),
                to_table: link.to.table_name_pascal_case().as_ref().to_string(),
                to_cols: link.to.identifier().col_names(),
                junction_table,
                link,
                from_ids,
            }
        }
    }

    impl<S, Key, From, To> Operation<S> for FetchManyToManyLinked<Key, From, To>
    where
        S: DatabaseExt + ExecutorTrait,
        Key: Clone + AsRef<str> + Send,
        From: Collection<Id: SingleColumnId> + ExpressionsForOperation + Clone + Send,
        <From::Id as CollectionId>::IdData: Copy
            + Clone
            + std::hash::Hash
            + Eq
            + Send
            + for<'q> Encode<'q, S>
            + Type<S>
            + for<'r> Decode<'r, S>,
        To: Collection<Id: SingleColumnId + ExpressionsForOperation>
            + TableExpressions
            + Clone
            + Send,
        To::OutputData: Send,
        <To::Id as CollectionId>::IdData: Send,
        To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
        To::Id: for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
        <To::Id as ExpressionsForOperation>::Identifier: for<'q> Expression<'q, S>,
        To::PascalCase: for<'q> Expression<'q, S>,
        To::SnakeCase: AsRef<str>,
        for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            if self.from_ids.is_empty() {
                return HashMap::new();
            }

            let junction = self.junction_table.clone();
            let from_col = self.from_col.clone();
            let to_table = self.to_table.clone();

            let from_ids = self.from_ids;
            let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
                select_items: (ManyToManyLinkedSelect {
                    junction_table: junction.clone(),
                    from_col: from_col.clone(),
                    to_table: to_table.clone(),
                    to_cols: self.to_cols,
                },),
                from: junction.clone(),
                joins: (JoinExpression {
                    join_type: "INNER JOIN",
                    foreign_table: to_table.clone(),
                    foreign_column: "id",
                    local_table: junction.clone(),
                    local_column: self.to_col.clone(),
                },),
                wheres: (ColumnIn {
                    col: ScopedColumn {
                        table: junction.clone(),
                        col: from_col.clone(),
                    },
                    values: from_ids.into_iter().map(Bind).collect::<Vec<_>>(),
                },),
                group_by: (),
                order: (),
                limit: (),
            })
            .unwrap();

            let rows = S::fetch_all(
                &mut *pool,
                Executable {
                    string: &stmt,
                    arguments: args,
                },
            )
            .await
            .unwrap();

            let to = self.link.to.clone();
            let to_id = self.link.to.id();
            rows_to_linked_map::<S, _, _, _, _>(rows, to, to_id)
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::relation_many_to_many::test_support::{
                migrate_todo_tag_fixtures, todo_to_tag_link,
            },
            operations::Operation,
            track_sqlx_query::watch_sqlx_calls,
        };

        use super::FetchManyToManyLinked;

        #[tokio::test(flavor = "current_thread")]
        async fn loads_tags_for_all_from_ids() {
            watch_sqlx_calls(async |actions| {
                let mut conn = Sqlite::in_memory_connection().await;

                let link = todo_to_tag_link();
                migrate_todo_tag_fixtures(&mut conn, &link).await;

                sqlx::query(
                    r#"
                    INSERT INTO "Tag" ("title") VALUES ('urgent'), ('home');
                    INSERT INTO "Todo" ("title", "done", "description") VALUES
                        ('todo_a', true, 'a'),
                        ('todo_b', false, 'b');
                    INSERT INTO "ct_todo_tag_def" ("todo_id", "tag_id") VALUES
                        (1, 1),
                        (1, 2),
                        (2, 1);
                    "#,
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let map = Operation::<Sqlite>::exec_operation(
                    FetchManyToManyLinked::new(link, vec![1, 2]),
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "ct_todo_tag_def"."todo_id" AS "from_id", "Tag"."id", "Tag"."title" FROM "ct_todo_tag_def" INNER JOIN "Tag" ON "ct_todo_tag_def"."tag_id" = "Tag"."id" WHERE "ct_todo_tag_def"."todo_id" IN ($1, $2);"#
                            .to_string(),
                    ]
                );

                assert_eq!(
                    map.len(),
                    2,
                    "map keys: {:?}",
                    map.keys().collect::<Vec<_>>()
                );
                assert_eq!(map.get(&1).map(|v| v.len()), Some(2));
                assert_eq!(map.get(&2).map(|v| v.len()), Some(1));
            })
            .await;
        }
    }
}

pub use impl_delete_junction_row::DeleteJunctionRow;
pub use impl_fetch_many_to_many_linked::FetchManyToManyLinked;
pub use impl_fetch_one_to_many_inverse_linked::FetchOneToManyInverseLinked;
pub use impl_insert_junction_and_fetch::InsertJunctionAndFetch;
pub use impl_insert_junction_row::InsertJunctionRow;
pub use impl_select_junction_to_ids::SelectJunctionToIds;
