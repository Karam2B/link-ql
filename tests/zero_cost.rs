use linked_sql::connect_in_memory::ConnectInMemory;
use linked_sql::define_collection;
use linked_sql::links::{DefaultRelationKey, Link, relation_one_to_many::OneToMany};
use linked_sql::operations::{
    CollectionOutput, LinkedOutput, Operation,
    fetch_many::{FetchMany, ManyOutput},
    operations_expressions_crossover::ExpressionsForOperation,
};
use linked_sql::sqlx_query_builder::basic_expressions::{Bind, ColumnContains};
use linked_sql::track_sqlx_query::{watch_sqlx_calls, without_pragma};
use sqlx::Sqlite;

define_collection!(
    struct Todo 3 {
        title: String,
        description: Option<String>,
        done: bool,
    }
);

define_collection!(
    struct Category 1 {
        title: String,
    }
);

impl Link<TodoHandler> for CategoryHandler {
    type Spec = OneToMany<DefaultRelationKey, TodoHandler, CategoryHandler>;

    fn spec(self) -> Self::Spec {
        OneToMany {
            fk_unique_id: DefaultRelationKey,
            from: TodoHandler,
            to: CategoryHandler,
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_zero_cost() {
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
                description TEXT,
                done BOOLEAN NOT NULL,
                fk_category_def INTEGER REFERENCES Category(id)
            );

            INSERT INTO Category (title) VALUES ('cat_1'), ('cat_2');
            INSERT INTO Todo (title, description, done, fk_category_def) VALUES
                ('first_todo', 'description', false, NULL),
                ('second_todo', 'description', false, NULL),
                ('third_todo', 'description', false, 2),
                ('fourth_todo', 'description', false, NULL),
                ('fifth_todo', 'description', false, NULL),
                ('sixth_todo', 'description', false, NULL),
                ('seventh_todo', 'description', false, NULL),
                ('eighth_todo', 'description', false, NULL);
            "#,
        )
        .execute(&mut conn)
        .await
        .unwrap();
        actions.clear();

        let todo = Operation::<Sqlite>::exec_operation(
            FetchMany {
                base: TodoHandler,
                wheres: ColumnContains {
                    col: todo_members::title.scoped(),
                    like: Bind(String::from("%i%")),
                },
                links: Link::<TodoHandler>::spec(CategoryHandler),
                cursor_order_by: (),
                cursor_first_item: (3i64, ()),
                limit: 3,
            },
            &mut conn,
        )
        .await;

        pretty_assertions::assert_eq!(
            todo,
            ManyOutput {
                items: vec![
                    LinkedOutput {
                        id: 3,
                        attributes: Todo {
                            title: "third_todo".to_string(),
                            description: Some("description".to_string()),
                            done: false,
                        },
                        links: Some(CollectionOutput {
                            id: 2,
                            attributes: Category {
                                title: "cat_2".to_string(),
                            },
                        }),
                    },
                    LinkedOutput {
                        id: 5,
                        attributes: Todo {
                            title: "fifth_todo".to_string(),
                            description: Some("description".to_string()),
                            done: false,
                        },
                        links: None,
                    },
                    LinkedOutput {
                        id: 6,
                        attributes: Todo {
                            title: "sixth_todo".to_string(),
                            description: Some("description".to_string()),
                            done: false,
                        },
                        links: None,
                    },
                ],
                next_item: Some((8, ())),
            }
        );

        pretty_assertions::assert_eq!(
            without_pragma(actions.take()),
            vec![
                r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."description" AS "bdescription", "Todo"."done" AS "bdone", "Category"."id" AS "lid", "Category"."title" AS "ltitle" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" WHERE "Todo"."title" LIKE $1 AND ("Todo"."id") >= ($2) LIMIT $3;"#.to_string(),
            ]
        );
    })
    .await;
}
