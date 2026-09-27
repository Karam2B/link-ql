use crate::links::{LinkedToBase, LinkedViaIds};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct OneToManyInverse<Id, F, T> {
    pub fk_unique_id: Id,
    pub from: F,
    pub to: T,
}

impl<Id, F, T> LinkedViaIds for OneToManyInverse<Id, F, T> {}

impl<Id, F, T> LinkedToBase for OneToManyInverse<Id, F, T> {
    type Base = F;
}

mod impl_link_fetch {
    use std::collections::HashSet;

    use crate::{
        collections::{Collection, CollectionId, SingleColumnId},
        from_row::FromRowData,
        links::fetch_linked_records::{
            FetchOneToManyInverseLinked, OneToManyInverseLinkedMap,
        },
        links::relation_one_to_many_inverse::OneToManyInverse,
        operations::{
            CollectionOutput, ManyLinkOutput, OperationOutput,
            fetch_many::LinkFetch,
            operations_expressions_crossover::{
                ExpressionsForOperation, IdentifierColNames, TableExpressions,
            },
        },
    };

    impl<Key, From, To> LinkFetch for OneToManyInverse<Key, From, To>
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
        FetchOneToManyInverseLinked<Key, From, To>: OperationOutput<
            Output = OneToManyInverseLinkedMap<
                <From::Id as CollectionId>::IdData,
                <To::Id as CollectionId>::IdData,
                To::OutputData,
            >,
        >,
    {
        type SelectItems = From::Id;

        fn non_aggregating_select_items(&self) -> Self::SelectItems {
            self.from.id()
        }

        type Join = ();

        fn non_duplicating_join_expressions(&self) -> Self::Join {}

        type Wheres = ();

        fn where_expressions(&self) -> Self::Wheres {}

        type Op = FetchOneToManyInverseLinked<Key, From, To>;

        type Output =
            ManyLinkOutput<CollectionOutput<<To::Id as CollectionId>::IdData, To::OutputData>>;

        fn take_many(
            &self,
            from_id: <Self::SelectItems as FromRowData>::RData,
            op: &mut <Self::Op as OperationOutput>::Output,
        ) -> Self::Output
        where
            Self::SelectItems: FromRowData,
        {
            ManyLinkOutput {
                many_output: op.remove(&from_id).unwrap_or_default(),
            }
        }

        type OpInput = Vec<<From::Id as CollectionId>::IdData>;

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
            input.push(*from_id);
        }

        fn operation_construct(&self, input: Self::OpInput) -> Self::Op
        where
            Self::SelectItems: FromRowData,
        {
            let mut seen = HashSet::new();
            let from_ids = input.into_iter().filter(|id| seen.insert(*id)).collect();
            FetchOneToManyInverseLinked::new(self.clone(), from_ids)
        }
    }

    #[cfg(test)]
    mod test {
        use sqlx::Sqlite;

        use crate::{
            connect_in_memory::ConnectInMemory,
            links::{DefaultRelationKey, relation_one_to_many_inverse::OneToManyInverse},
            operations::{CollectionOutput, LinkedOutput, Operation, fetch_one::FetchOne},
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
            test_module::{Category, CategoryHandler, Todo, TodoHandler},
            track_sqlx_query::watch_sqlx_calls,
        };

        #[tokio::test(flavor = "current_thread")]
        async fn fetch_one_returns_many_todos() {
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
                    INSERT INTO Category (title) VALUES ('work');
                    INSERT INTO Todo (title, done, description, fk_category_def) VALUES ('todo_1', 1, NULL, 1);
                    INSERT INTO Todo (title, done, description, fk_category_def) VALUES ('todo_2', 0, 'desc', 1);
                    ",
                )
                .execute(&mut conn)
                .await
                .unwrap();
                actions.clear();

                let output = Operation::<Sqlite>::exec_operation(
                    FetchOne {
                        base: CategoryHandler,
                        wheres: ColumnEqual { col: "id", eq: Bind(1) },
                        links: OneToManyInverse {
                            fk_unique_id: DefaultRelationKey,
                            from: CategoryHandler,
                            to: TodoHandler,
                        },
                    },
                    &mut conn,
                )
                .await;

                pretty_assertions::assert_eq!(
                    actions.take(),
                    vec![
                        r#"SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle", "Category"."id" AS "lid" FROM "Category" WHERE "id" = $1;"#
                            .to_string(),
                        r#"SELECT "Todo"."fk_category_def" AS "from_id", "Todo"."id", "Todo"."title", "Todo"."done", "Todo"."description" FROM "Todo" WHERE "Todo"."fk_category_def" IN ($1);"#
                            .to_string(),
                    ]
                );

                pretty_assertions::assert_eq!(
                    output,
                    Some(LinkedOutput {
                        id: 1,
                        attributes: Category {
                            title: "work".to_string(),
                        },
                        links: crate::operations::ManyLinkOutput {
                            many_output: vec![
                                CollectionOutput {
                                    id: 1,
                                    attributes: Todo {
                                        title: "todo_1".to_string(),
                                        done: true,
                                        description: None,
                                    },
                                },
                                CollectionOutput {
                                    id: 2,
                                    attributes: Todo {
                                        title: "todo_2".to_string(),
                                        done: false,
                                        description: Some("desc".to_string()),
                                    },
                                },
                            ],
                        },
                    })
                );
            })
            .await;
        }
    }
}

pub use crate::links::fetch_linked_records::FetchOneToManyInverseLinked;
