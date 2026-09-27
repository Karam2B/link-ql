use linked_sql::connect_in_memory::ConnectInMemory;
use linked_sql::json_client::client_interface::Client;
use linked_sql::track_sqlx_query::{watch_sqlx_calls, without_pragma};
use sqlx::Sqlite;

#[tokio::test(flavor = "current_thread")]
async fn test_main_example() {
    watch_sqlx_calls(async |actions| {
        let pool = Sqlite::in_memory_pool().await;
        let (client, executor) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        actions.spawn(executor.run());

        client
            .exec(
                r#"
        {
            "op": "add_collection",
            "body": {
                "name": "todo",
                "fields": [
                    {
                        "name": "title",
                        "type_info": "String",
                        "is_optional": false
                    },
                    {
                        "name": "description",
                        "type_info": "String",
                        "is_optional": true
                    },
                    {
                        "name": "done",
                        "type_info": "Boolean",
                        "is_optional": false
                    }
                ]
            }
        }
    "#
                .to_string(),
            )
            .await;

        client
            .exec(
                r#"
        {
            "op": "add_collection",
            "body": {
                "name": "category",
                "fields": [
                    { "name": "title", "type_info": "String", "is_optional": false }
                ]
            }
        }
    "#
                .to_string(),
            )
            .await;

        client
            .exec(
                r#"
        {
            "op": "add_link",
            "body": {
                "ty": "one_to_many",
                "from": "todo",
                "to": "category"
            }
        }
    "#
                .to_string(),
            )
            .await;

        client
            .exec(
                r#"
        {
            "op": "insert_one",
            "body": {
                "base": "category",
                "data": { "title": "cat_1" },
                "links": []
            }
        }
    "#
                .to_string(),
            )
            .await;
        client
            .exec(
                r#"
        {
            "op": "insert_one",
            "body": {
                "base": "category",
                "data": { "title": "cat_2" },
                "links": []
            }
        }
    "#
                .to_string(),
            )
            .await;

        for (title, done, category) in [
            ("first_todo", false, None),
            ("second_todo", false, None),
            ("third_todo", false, Some(2)),
            ("fourth_todo", false, None),
            ("fifth_todo", false, None),
            ("sixth_todo", true, None),
            ("seventh_todo", false, None),
            ("eighth_todo", false, None),
        ] {
            let links = match category {
                Some(id) => format!(r#"{{ "ty": "set_id", "to": "category", "id": {id} }}"#),
                None => String::new(),
            };
            client
                .exec(format!(
                    r#"
        {{
            "op": "insert_one",
            "body": {{
                "base": "todo",
                "data": {{
                    "title": "{title}",
                    "description": "description",
                    "done": {done}
                }},
                "links": [{links}]
            }}
        }}
    "#
                ))
                .await;
        }

        actions.clear();

        let result = client
            .exec(
                r#"
        {
            "op": "fetch_many",
            "body": {
                "base": "todo",
                "filters": [
                    { "ty": "col_like", "col": "title", "value": "i" }
                ],
                "links": [
                    { "ty": "one_to_many", "to": "category" }
                ],
                "pagination": {
                    "limit": 3,
                    "first_item": { "id": 3, "data": {} },
                    "order_by": []
                }
            }
        }
    "#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"items":[{"id":3,"attributes":{"description":"description","done":false,"title":"third_todo"},"links":[{"id":2,"attributes":{"title":"cat_2"}}]},{"id":5,"attributes":{"description":"description","done":false,"title":"fifth_todo"},"links":[null]},{"id":6,"attributes":{"description":"description","done":true,"title":"sixth_todo"},"links":[null]}],"next_item":{"id":8,"attributes":{}}}}"#
        );

        pretty_assertions::assert_eq!(
            without_pragma(actions.take()),
            vec![
                r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."description" AS "bdescription", "Todo"."done" AS "bdone", "Category"."id" AS "l0id", "Category"."title" AS "l0title" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" WHERE "Todo"."title" LIKE $1 AND ("Todo"."id") >= ($2) LIMIT $3;"#.to_string(),
            ]
        );
    })
    .await;
}
