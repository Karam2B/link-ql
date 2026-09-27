pub mod fetch_linked_records;
pub mod junction_table_op;
pub mod relation_many_to_many;
pub mod relation_one_to_many;
pub mod relation_one_to_many_inverse;
pub mod timestamp;
pub mod update_links;
mod utils;

pub trait Link<Base> {
    type Spec;
    fn spec(self) -> Self::Spec;
}

#[derive(Clone)]
pub struct DefaultRelationKey;

impl AsRef<str> for DefaultRelationKey {
    fn as_ref(&self) -> &str {
        "_def"
    }
}

impl crate::sqlx_query_builder::sanitize_combinator::SanitizeWrite for DefaultRelationKey {
    fn write_content<S: crate::database_extention::DatabaseExt>(&self, stmt: &mut String) {
        S::sanitize(self.as_ref(), stmt);
    }
}

/// link back Link::Spec to Base
pub trait LinkedToBase {
    type Base;
}

pub trait LinkedViaId {}
pub trait LinkedViaIds {}

mod many_links {
    use crate::from_row::{FromRowAlias, FromRowData, FromRowError, RowNumAliased, RowStrAliased};
    use crate::operations::OperationOutput;
    use crate::operations::operations_expressions_crossover::ExpressionsForOperation;
    use sqlx::Row;

    #[allow(dead_code)]
    pub struct ManyLinks<T>(pub T);

    impl<L0, L1> FromRowData for ManyLinks<(L0, L1)>
    where
        L0: FromRowData,
        L1: FromRowData,
    {
        type RData = (L0::RData, L1::RData);
    }

    impl<'r, R, L0, L1> FromRowAlias<'r, R> for ManyLinks<(L0, L1)>
    where
        R: Row,
        L0: FromRowAlias<'r, R>,
        L1: FromRowAlias<'r, R>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
            Ok((self.0.0.no_alias(row)?, self.0.1.no_alias(row)?))
        }
        fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            Ok((
                self.0.0.num_alias(RowNumAliased {
                    row: row.row,
                    str_alias: row.alias,
                    num_alias: Some(0),
                })?,
                self.0.1.num_alias(RowNumAliased {
                    row: row.row,
                    str_alias: row.alias,
                    num_alias: Some(1),
                })?,
            ))
        }
        fn num_alias(&self, _: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            panic!("should not nest multiple links")
        }
    }

    impl<L0, L1> ExpressionsForOperation for ManyLinks<(L0, L1)>
    where
        L0: ExpressionsForOperation,
        L1: ExpressionsForOperation,
    {
        type Identifier = (L0::Identifier, L1::Identifier);
        fn identifier(&self) -> Self::Identifier {
            (self.0.0.identifier(), self.0.1.identifier())
        }
        type Scoped = (L0::Scoped, L1::Scoped);
        fn scoped(&self) -> Self::Scoped {
            (self.0.0.scoped(), self.0.1.scoped())
        }
        type ScopedAliased = (L0::NumScopedAliased, L1::NumScopedAliased);
        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            (
                self.0.0.num_scoped_aliased(0, alias),
                self.0.1.num_scoped_aliased(1, alias),
            )
        }
        type NumScopedAliased = (L0::NumScopedAliased, L1::NumScopedAliased);
        fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {
            panic!("should not nest multiple links")
        }
    }

    impl<L0, L1> OperationOutput for ManyLinks<(L0, L1)>
    where
        L0: OperationOutput,
        L1: OperationOutput,
    {
        type Output = (L0::Output, L1::Output);
    }

    impl<T> FromRowData for ManyLinks<Vec<T>>
    where
        T: FromRowData,
    {
        type RData = Vec<T::RData>;
    }

    impl<'r, R, T> FromRowAlias<'r, R> for ManyLinks<Vec<T>>
    where
        R: Row,
        T: FromRowAlias<'r, R>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
            let mut r = vec![];
            for each in self.0.iter() {
                r.push(each.no_alias(row)?);
            }
            Ok(r)
        }
        fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            let mut r = vec![];
            for (num, each) in self.0.iter().enumerate() {
                r.push(each.num_alias(RowNumAliased {
                    row: row.row,
                    str_alias: row.alias,
                    num_alias: Some(num),
                })?);
            }
            Ok(r)
        }
        fn num_alias(&self, _: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            panic!("should not nest multiple links")
        }
    }

    impl<T> ExpressionsForOperation for ManyLinks<Vec<T>>
    where
        T: ExpressionsForOperation,
    {
        type Identifier = Vec<T::Identifier>;
        fn identifier(&self) -> Self::Identifier {
            self.0.iter().map(|t| t.identifier()).collect()
        }

        type Scoped = Vec<T::Scoped>;

        fn scoped(&self) -> Self::Scoped {
            self.0.iter().map(|t| t.scoped()).collect()
        }

        type ScopedAliased = Vec<T::NumScopedAliased>;

        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            self
                .0
                .iter()
                .enumerate()
                .map(|(num, t)| t.num_scoped_aliased(num, alias))
                .collect()
        }

        type NumScopedAliased = Vec<T::NumScopedAliased>;

        fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {
            panic!("should not nest multiple links")
        }
    }

    impl<T> OperationOutput for ManyLinks<Vec<T>>
    where
        T: OperationOutput,
    {
        type Output = Vec<T::Output>;
    }

    #[linked_sql_macros::skip]
    mod tests {
        #[tokio::test(flavor = "current_thread")]
        async fn main_with_two_links() {
            watch_sqlx_calls(async |actions| {
            let mut pool = Sqlite::in_memory_connection().await;

            sqlx::query(
                "
                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO Todo (title, done, description, created_at, updated_at) VALUES
                    ('todo_1', false, 'description_1', 'created_at_1', 'updated_at_1'),
                    ('todo_2', true, 'description_2', 'created_at_2', 'updated_at_2'),
                    ('todo_3', false, 'description_3', 'created_at_3', 'updated_at_3');
            ",
            )
            .execute(&mut pool)
            .await
            .unwrap();

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![format!("SELECT \"Todo\" {members} {first_link} {second_link}", 
                members = r#""Todo"."id" as "iid", "Todo"."title" as "btitle", "Todo"."done" as "bdone", "Todo"."description" as "bdescription""#,
                first_link = r#""Todo"."created_at" as "l0created_at", "Todo"."updated_at" as "l0updated_at""#,
                second_link = r#""Todo"."created_at" as "l1created_at", "Todo"."updated_at" as "l1updated_at""#
            )]
            );

            let output = Operation::<Sqlite>::exec_operation(
                FetchOne {
                    base: TodoHandler,
                    links: TwoLinks(
                        Timestamp {
                            collection: TodoHandler,
                        },
                        Timestamp {
                            collection: TodoHandler,
                        },
                    ),
                    wheres: (),
                },
                &mut pool,
            )
            .await;

            pretty_assertions::assert_eq!(
                output,
                Some(LinkedOutput {
                    id: 2,
                    attributes: Todo {
                        title: String::from("todo_2"),
                        done: true,
                        description: Some(String::from("description_2")),
                    },
                    links: ()
                })
            );
        })
        .await;
        }
    }
}
