//! Central place for all relating changes in regards to the old lib
//! finishing the refactor means to delete this file, and move all its
//! contents to the appropriate places
//!
//! coments on the top level are instructions for finishing
//! the refactor WHEN the refactor is ready
//!
//! "TODO" comments indicate the refactor is not done, if you are instructed to
//! refactor and you find any "TODO" comments, you should abort and
//! ask for clarification

/// TODO: how the current refactor affects the test_module module
#[linked_sql_macros::skip]
#[path = "test_module.rs"]
pub mod test_module_old;

pub mod test_module {
    use crate::sqlx_query_builder::valid_syntax::MembersExpressions;

    macro_rules! define_collection {
        (
            struct $pascal_case:ident $size:literal {$(
                $member:ident: $type:ty,
            )*}
        ) => {
            const _: () = {
                if $size == 0 {
                    panic!("size must be greater than 0");
                }
            };

            pub struct [<$pascal_case Handler>];

            pub struct $pascal_case {
                $(
                    pub $member: $type,
                )*
            }

            // impl TableExpressions for Collection
            const _: () = {
                use crate::sqlx_query_builder::valid_syntax::TableExpressions;

                impl<'a> TableExpressions<'a> for paste::paste! { [<$pascal_case Handler>] } {
                    type PascalCase = &'static str;
                    type SnakeCase = &'static str;
                    fn pascal_case(&self) -> Self::PascalCase {
                        paste::paste! {
                            stringify!($pascal_case)
                        }
                    }
                    fn snake_case(&self) -> Self::SnakeCase {
                        paste::paste! {
                            stringify!([<$pascal_case:snake>])
                        }
                    }
                }
            };
        };
    }

    define_collection! {
        struct Todo 3 {
            title: String,
            done: bool,
            description: Option<String>,
        }
    }

    pub struct ScopedMembers {
        pub table: &'static str,
        pub alias: &'static str,
        pub members: [&'static str; 3],
    }

    impl<'a> MembersExpressions<'a> for TodoHandler {
        type Identifier = [&'static str; 3];

        fn identifier(&'a self) -> Self::Identifier {
            ["title", "done", "description"]
        }

        type Scoped = ScopedMembers;

        fn scoped(&'a self) -> Self::Scoped {
            ScopedMembers {
                table: "Todo",
                alias: todo!(),
                members: ["title", "done", "description"],
            }
        }

        type ScopedAliased = ScopedMembers;

        fn scoped_aliased(&'a self, alias: &'static str) -> Self::ScopedAliased {
            ScopedMembers {
                table: "Todo",
                alias,
                members: ["title", "done", "description"],
            }
        }

        type NumScopedAliased = [&'static str; 3];

        fn num_scoped_aliased(&'a self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
            ["title", "done", "description"]
        }

        fn members(&self) -> (String, Vec<String>) {
            (
                "Todo".to_owned(),
                vec![
                    "title".to_owned(),
                    "done".to_owned(),
                    "description".to_owned(),
                ],
            )
        }
    }

    define_collection! {
        struct Category 1 {
            title: String,
        }
    }

    define_collection! {
        struct Tag 1 {
            title: String,
        }
    }
}

/// TODO: how the current refactor affects the operations module
/// when refactor is ready remove the content of operations and place
/// the content of operations_v2
#[path = "../operations_v2/mod.rs"]
pub mod operations;
#[linked_sql_macros::skip]
#[path = "operations/mod.rs"]
pub mod operations_old;

/// TODO: how the current refactor affects the json_client module
#[path = "../json_client/mod.rs"]
pub mod json_client;

pub mod valid_syntax {
    //! Traits that return 'impl Expression' types that produce correct SQL.
    //!
    //! 'impl Expression' types often do not gaurd against invalid composition,
    //! for example:
    //!
    //! ```rust,compile_fail
    //! struct ColumnEqual<C, E> {
    //!     column: C,
    //!     expression: E,
    //! }
    //!
    //! impl<'a, S, C, E> Expression<'a, S> for ColumnEqual<C, E>
    //! where
    //!     C: Expression<'a, S>,
    //!     E: Expression<'a, S>,
    //! {
    //!     ... some code ...
    //! }
    //!
    //! fn invalid_composition() {
    //!     let invalid = ColumnEqual {
    //!         column: "column".to_string(),
    //!         expression: NotNull,
    //!     };
    //! }
    //! ```
    //!
    //! this will compile, but produce invalid SQL
    //!
    //! # Implementation Guidelines
    //! 1. Associated types should implement 'A: Expression' or 'Join<A>: Expression'.
    //! 2. Try to reuse existing types instead of creating new ones just for your implementation.
    //!
    //! # Deprecating old operation_expressions_crossover module
    //! this is an attempt to deprecate the old operation_expressions_crossover module.
    //! the difference is that this module reduce unnecessary cloning by using lifetimes

    pub trait TableExpressions<'a> {
        type PascalCase;
        type SnakeCase;
        fn pascal_case(&'a self) -> Self::PascalCase;
        fn snake_case(&'a self) -> Self::SnakeCase;
    }

    pub trait MigrationExpressions<'a> {
        type Migrate;
        fn migrate(&'a self) -> Self::Migrate;
    }

    pub trait MembersExpressions<'a> {
        type Identifier;
        fn identifier(&'a self) -> Self::Identifier;

        type Scoped;
        fn scoped(&'a self) -> Self::Scoped;

        type ScopedAliased;
        fn scoped_aliased(&'a self, alias: &'static str) -> Self::ScopedAliased;

        type NumScopedAliased;
        fn num_scoped_aliased(&'a self, num: usize, alias: &'static str) -> Self::NumScopedAliased;

        fn members(&self) -> (String, Vec<String>);
    }

    pub trait Identifier<'a> {
        type Identifier;
        fn identifier_2(&'a self) -> Self::Identifier;

        fn members_2(&self) -> (String, Vec<String>);
    }

    impl<'a, T> Identifier<'a> for T
    where
        T: MembersExpressions<'a>,
    {
        type Identifier = T::Identifier;
        fn identifier_2(&'a self) -> Self::Identifier {
            MembersExpressions::identifier(self)
        }
        fn members_2(&self) -> (String, Vec<String>) {
            MembersExpressions::members(self)
        }
    }
}

mod dynamic_collection_impl {
    use crate::{
        database_extention::DatabaseExt,
        json_client::dynamic_collection::DynamicCollection,
        sqlx_query_builder::combinators::Nest,
        sqlx_query_builder::{
            Expression,
            statements::create_table_statement::{
                ColumnDefinition, CreateTable, NotNull, expressions::CreateTableInit,
            },
            trait_objects::BoxedExpression,
            valid_syntax::{MigrationExpressions, TableExpressions},
        },
    };

    impl<'a, S: DatabaseExt> TableExpressions<'a> for DynamicCollection<S> {
        type PascalCase = &'a str;

        type SnakeCase = &'a str;

        fn pascal_case(&'a self) -> Self::PascalCase {
            &self.collection_name.pascal_case
        }

        fn snake_case(&'a self) -> Self::SnakeCase {
            &self.collection_name.snake_case
        }
    }

    impl<'a, S: DatabaseExt> MigrationExpressions<'a> for DynamicCollection<S>
    where
        S::SinglePrimaryKeyConstaint: for<'q> Expression<'q, S> + Send,
    {
        type Migrate = CreateTable<
            CreateTableInit,
            &'a str,
            Vec<
                ColumnDefinition<(
                    Nest<&'a str>,
                    Nest<Box<dyn BoxedExpression<S> + Send>>,
                    Nest<Option<NotNull>>,
                )>,
            >,
        >;

        fn migrate(&'a self) -> Self::Migrate {
            let mut col_defs = vec![ColumnDefinition((
                Nest("id"),
                Nest(Box::new(S::single_primary_key_constaint_expression())
                    as Box<dyn BoxedExpression<S> + Send>),
                Nest(None),
            ))];

            for field in self.fields.iter() {
                let type_info = (field.type_info.type_expression)();
                col_defs.push(ColumnDefinition((
                    Nest(field.name.as_str()),
                    Nest(type_info),
                    if field.is_optional {
                        Nest(None)
                    } else {
                        Nest(Some(NotNull))
                    },
                )));
            }

            CreateTable {
                init: CreateTableInit,
                name: &self.collection_name.pascal_case,
                col_defs,
            }
        }
    }

    #[cfg(test)]
    #[test]
    fn test_dynamic_collection_migrate_expression() {
        use std::sync::Arc;

        use sqlx::Sqlite;

        use crate::{
            json_client::dynamic_collection::{CollectionName, DynamicField, FieldName, VTable},
            sqlx_query_builder::StatementBuilder,
        };

        let collection = DynamicCollection::<Sqlite> {
            collection_name: CollectionName {
                pascal_case: Arc::from("Todo"),
                snake_case: Arc::from("todo"),
            },
            fields: vec![
                DynamicField {
                    name: FieldName {
                        snake_case: Arc::from("title"),
                    },
                    type_info: VTable::new_as::<String>(),
                    is_optional: false,
                },
                DynamicField {
                    name: FieldName {
                        snake_case: Arc::from("done"),
                    },
                    type_info: VTable::new_as::<bool>(),
                    is_optional: false,
                },
                DynamicField {
                    name: FieldName {
                        snake_case: Arc::from("description"),
                    },
                    type_info: VTable::new_as::<String>(),
                    is_optional: true,
                },
            ],
        };

        let migrate = MigrationExpressions::migrate(&collection);
        let string = StatementBuilder::<Sqlite>::new(migrate).unwrap().0;

        assert_eq!(
            string,
            "CREATE TABLE IF NOT EXISTS todo (id INTEGER PRIMARY KEY, title TEXT NOT NULL, done BOOLEAN NOT NULL, description TEXT)".to_string()
        );
    }
}

mod fetch_one_impl_operation {

    use crate::{
        collections::{Collection, CollectionId},
        database_extention::DatabaseExt,
        fix_executor::ExecutorTrait,
        operations::{
            LinkedOutput,
            fetch_many::LinkFetch,
            fetch_one::FetchOne,
            v2::{Operation, OperationOutput},
        },
        sqlx_query_builder::{
            Expression, StatementBuilder,
            combinators::{Join, OptionalExpression},
            statements::select_statement::SelectStatement,
            valid_syntax::{MembersExpressions, TableExpressions},
        },
    };

    impl<Base, Links, Wheres> OperationOutput for FetchOne<Base, Links, Wheres>
    where
        Base: Collection,
        Links: LinkFetch,
    {
        type Output = Option<
            LinkedOutput<<Base::Id as CollectionId>::IdData, Base::OutputData, Links::Output>,
        >;
    }

    impl<S, Base, Links, Wheres> Operation<S> for FetchOne<Base, Links, Wheres>
    where
        S: DatabaseExt,
        S: ExecutorTrait,
        Base: Send,
        Base: Collection,
        Base::OutputData: Send,
        Base: for<'a> TableExpressions<'a, PascalCase: for<'q> Expression<'a, S>>,
        Base: for<'a> MembersExpressions<'a, ScopedAliased: OptionalExpression>,
        for<'a, 'q> Join<<Base as MembersExpressions<'a>>::ScopedAliased>: Expression<'q, S>,
        <Base::Id as CollectionId>::IdData: Send,
        Links: Send,
        Links: LinkFetch,
        Links::Output: Send,
        Wheres: Send,
    {
        fn exec_operation(
            self,
            pool: &mut S::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
                    select_items: self.base.scoped_aliased("b"),
                    from: self.base.pascal_case(),
                    joins: (),
                    wheres: (),
                    group_by: (),
                    order: (),
                    limit: (),
                })
                .unwrap();

                panic!("not implemented, {:?}", stmt);
            }
        }
    }

    #[cfg(test)]
    #[tokio::test]
    async fn test_fetch_one_operation() {
        use crate::connect_in_memory::ConnectInMemory;
        use crate::test_module::Todo;
        use crate::test_module::TodoHandler;
        use sqlx::Sqlite;

        let mut conn = Sqlite::in_memory_connection().await;

        let fetch_one = Operation::<Sqlite>::exec_operation(
            FetchOne {
                base: TodoHandler,
                links: (),
                wheres: (),
            },
            &mut conn,
        )
        .await;

        pretty_assertions::assert_eq!(
            fetch_one,
            Some(LinkedOutput {
                id: 1,
                attributes: Todo {
                    title: "Test".to_string(),
                    done: false,
                    description: None
                },
                links: ()
            })
        );
    }
}

mod many_to_many_operation {
    pub struct ManyToMany<const INVERSE: bool, Key, From, To> {
        pub relation_key: Key,
        pub from: From,
        pub to: To,
    }

    mod impl_migrate_expression {
        use crate::sqlx_query_builder::Sanitize;
        use crate::sqlx_query_builder::statements::create_table_statement::CreateTable;
        use crate::sqlx_query_builder::statements::create_table_statement::expressions::CreateTableInit;
        use crate::sqlx_query_builder::valid_syntax::many_to_many_operation::ManyToMany;
        use crate::sqlx_query_builder::valid_syntax::{MigrationExpressions, TableExpressions};

        impl<'a, const INVERSE: bool, Key, From, To> MigrationExpressions<'a>
            for ManyToMany<INVERSE, Key, From, To>
        where
            From: TableExpressions<'a>,
            To: TableExpressions<'a>,
            Key: AsRef<str>,
        {
            type Migrate = CreateTable<
                CreateTableInit,
                Sanitize<(
                    &'a str,
                    &'static str,
                    <From as TableExpressions<'a>>::SnakeCase,
                    &'static str,
                    <To as TableExpressions<'a>>::SnakeCase,
                )>,
                (),
            >;

            fn migrate(&'a self) -> Self::Migrate {
                CreateTable {
                    init: CreateTableInit,
                    // name: "key_todo_category",
                    name: Sanitize((
                        self.relation_key.as_ref(),
                        "_",
                        self.from.snake_case(),
                        "_",
                        self.to.snake_case(),
                    )),

                    // name: Sanitize((
                    //     self.relation_key.as_ref(),
                    //     "_",
                    //     self.from.snake_case(),
                    //     "_",
                    //     self.to.snake_case(),
                    // )),
                    col_defs: (),
                }
            }
        }

        #[cfg(test)]
        #[test]
        fn test_many_to_many_migrate_expression() {
            use super::super::MigrationExpressions;
            use crate::{
                sqlx_query_builder::StatementBuilder,
                test_module::{CategoryHandler, TodoHandler},
            };
            use sqlx::Sqlite;

            let many_to_many = ManyToMany::<false, _, _, _> {
                relation_key: "key",
                from: TodoHandler,
                to: CategoryHandler,
            };
            let migrate = MigrationExpressions::migrate(&many_to_many);
            let string = StatementBuilder::<Sqlite>::new(migrate).unwrap().0;

            assert_eq!(
                string,
                "CREATE TABLE IF NOT EXISTS key_todo_category ()".to_string()
            );
        }
    }
}
