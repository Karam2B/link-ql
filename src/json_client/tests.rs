use sqlx::Sqlite;

use crate::{
    connect_in_memory::ConnectInMemory,
    json_client::client_interface::Client,
    track_sqlx_query::{assert_sql_eq, watch_sqlx_calls},
};

use super::test_utilities::{
    add_category_collection, add_todo_collection, clear_timestams, setup_todo_collection,
    setup_todo_with_category_link, todo_is_one_to_many_with_category, todo_is_timestamped,
};

#[tokio::test(flavor = "current_thread")]
async fn test_insert_one() {
    watch_sqlx_calls(async |actions| {
            let pool = Sqlite::in_memory_pool().await;
            let (client, ex) = Client::new_sqlx_db(pool);
            let client = client.into_string_client();

            actions.spawn(ex.run());

            setup_todo_collection(&client, &actions).await;

            client
                .exec(
                    r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "test_todo",
            "done": false,
            "description": "test_description"
        },
        "links": []
    }
}
"#
                    .to_string(),
                )
                .await;

            assert_sql_eq(
                actions.take(),
                vec![
                    r#"PRAGMA foreign_keys = ON;"#.to_string(),
                    r#"INSERT INTO "Todo" ("title", "description", "done") VALUES ($1, $2, $3) RETURNING "id", "title", "description", "done";"#.to_string(),
                ]
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn insert_many_inserts_batch_on_one_connection() {
    watch_sqlx_calls(async |actions| {
            let pool = Sqlite::in_memory_pool().await;
            let (client, ex) = Client::new_sqlx_db(pool);
            let client = client.into_string_client();

            actions.spawn(ex.run());

            setup_todo_collection(&client, &actions).await;

            client
                .exec(
                    r#"
{
    "op": "insert_many",
    "body": {
        "base": "todo",
        "items": [
            {
                "data": {
                    "title": "first",
                    "done": false,
                    "description": "d1"
                },
                "links": []
            },
            {
                "data": {
                    "title": "second",
                    "done": true,
                    "description": "d2"
                },
                "links": []
            }
        ]
    }
}
"#
                    .to_string(),
                )
                .await;

            assert_sql_eq(
                actions.take(),
                vec![
                    r#"PRAGMA foreign_keys = ON;"#.to_string(),
                    r#"INSERT INTO "Todo" ("title", "description", "done") VALUES ($1, $2, $3) RETURNING "id", "title", "description", "done";"#.to_string(),
                    r#"INSERT INTO "Todo" ("title", "description", "done") VALUES ($1, $2, $3) RETURNING "id", "title", "description", "done";"#.to_string(),
                ]
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn insert_many_returns_items_with_ids_and_attributes() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "insert_many",
    "body": {
        "base": "todo",
        "items": [
            {
                "data": {
                    "title": "first",
                    "done": false,
                    "description": "d1"
                },
                "links": []
            },
            {
                "data": {
                    "title": "second",
                    "done": true,
                    "description": "d2"
                },
                "links": []
            }
        ]
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"items":[{"id":1,"attributes":{"description":"d1","done":false,"title":"first"},"links":[]},{"id":2,"attributes":{"description":"d2","done":true,"title":"second"},"links":[]}]}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn insert_many_empty_items_returns_invalid_data() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "insert_many",
    "body": {
        "base": "todo",
        "items": []
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(result, r#"{"error":"InvalidData"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn exec_returns_invalid_input_for_non_json() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    let result = client.exec(r#"not json"#.to_string()).await;
    pretty_assertions::assert_eq!(result, r#"{"error":"invalid_input"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn exec_returns_invalid_body_for_malformed_add_collection() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    let result = client
        .exec(
            r#"
{
    "op": "add_collection",
    "body": ["todo", []]
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(result, r#"{"error":"invalid_body"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn add_collection_returns_null_output() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;

    pretty_assertions::assert_eq!(
        client
            .exec(
                r#"
{
    "op": "add_collection",
    "body": {
        "name": "tag",
        "fields": [
            { "name": "title", "type_info": "String", "is_optional": false }
        ]
    }
}
"#
                .to_string(),
            )
            .await,
        r#"{"output":null}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn add_collection_rejects_duplicate() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "add_collection",
    "body": {
        "name": "todo",
        "fields": [
            { "name": "title", "type_info": "String", "is_optional": false }
        ]
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(result, r#"{"error":"CollectionAlreadyExists"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn add_link_rejects_duplicate() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    let result = client
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
    pretty_assertions::assert_eq!(result, r#"{"error":"LinkAlreadyExists"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn add_link_rejects_missing_collection() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "add_link",
    "body": {
        "ty": "one_to_many",
        "from": "todo",
        "to": "missing"
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(result, r#"{"error":"CollectionNotFound"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn insert_one_category_returns_id_and_attributes() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "category_1" },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"id":1,"attributes":{"title":"category_1"},"links":[]}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn update_one_updates_todo_by_id() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "before_update",
            "done": false,
            "description": "desc"
        },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;

    let result = client
        .exec(
            r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "title": "after_update" },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"id":1,"attributes":{"description":"desc","done":false,"title":"after_update"},"links":[]}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_one_returns_todo_with_one_to_many_link() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "work" },
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
        "base": "todo",
        "data": {
            "title": "todo_1",
            "done": true,
            "description": "desc"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
            .to_string(),
        )
        .await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_one",
    "body": {
        "base": "todo",
        "id": 1,
        "filters": [],
        "links": [
            { "ty": "one_to_many", "to": "category" }
        ]
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"id":1,"attributes":{"description":"desc","done":true,"title":"todo_1"},"links":[{"id":1,"attributes":{"title":"work"}}]}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_one_from_category_returns_many_todos() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "work" },
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
        "base": "todo",
        "data": {
            "title": "todo_1",
            "done": true,
            "description": "desc"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
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
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_2",
            "done": false,
            "description": null
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
            .to_string(),
        )
        .await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_one",
    "body": {
        "base": "category",
        "id": 1,
        "filters": [],
        "links": [
            { "ty": "one_to_many", "to": "todo" }
        ]
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"id":1,"attributes":{"title":"work"},"links":[{"many_output":[{"id":1,"attributes":{"description":"desc","done":true,"title":"todo_1"}},{"id":2,"attributes":{"description":null,"done":false,"title":"todo_2"}}]}]}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_many_returns_inserted_todo_with_timestamp_link() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool.clone());
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    add_category_collection(&client).await;
    todo_is_one_to_many_with_category(&client).await;
    todo_is_timestamped(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_1",
            "done": true,
            "description": "description_1"
        },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;
    clear_timestams(pool.clone()).await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [],
        "links": [
            { "ty": "timestamp" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"items":[{"id":1,"attributes":{"description":"description_1","done":true,"title":"todo_1"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_many_col_eq_filter() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool.clone());
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    todo_is_timestamped(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "done_todo",
            "done": true
        },
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
        "base": "todo",
        "data": {
            "title": "open_todo",
            "done": false
        },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;
    clear_timestams(pool).await;

    let matching = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_eq", "col": "done", "eq": true }
        ],
        "links": [
            { "ty": "timestamp" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(
        matching,
        r#"{"output":{"items":[{"id":1,"attributes":{"description":null,"done":true,"title":"done_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );

    let not_done = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_eq", "col": "done", "eq": false }
        ],
        "links": [
            { "ty": "timestamp" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(
        not_done,
        r#"{"output":{"items":[{"id":2,"attributes":{"description":null,"done":false,"title":"open_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_many_rejects_unknown_filter_field() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    todo_is_timestamped(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_eq", "col": "missing", "eq": "x" }
        ],
        "links": [
            { "ty": "timestamp" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(result, r#"{"error":"InvalidFilter"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_many_rejects_filter_type_mismatch() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    todo_is_timestamped(&client).await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_eq", "col": "done", "eq": "not_a_bool" }
        ],
        "links": [
            { "ty": "timestamp" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(result, r#"{"error":"InvalidFilter"}"#);
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_many_col_ne_contains_and_composite_filters() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool.clone());
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    add_todo_collection(&client).await;
    todo_is_timestamped(&client).await;

    client
        .exec(
            r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": { "title": "urgent_todo", "done": true },
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
        "base": "todo",
        "data": { "title": "open_todo", "done": false },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;

    clear_timestams(pool).await;

    let not_done = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_ne", "col": "done", "ne": true }
        ],
        "links": [{ "ty": "timestamp" }],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(
        not_done,
        r#"{"output":{"items":[{"id":2,"attributes":{"description":null,"done":false,"title":"open_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );

    let contains = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            { "ty": "col_contains", "col": "title", "value": "urgent" }
        ],
        "links": [{ "ty": "timestamp" }],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(
        contains,
        r#"{"output":{"items":[{"id":1,"attributes":{"description":null,"done":true,"title":"urgent_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );

    let composite = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [
            {
                "ty": "or",
                "filters": [
                    { "ty": "col_eq", "col": "done", "eq": true },
                    { "ty": "col_contains", "col": "title", "value": "open" }
                ]
            }
        ],
        "links": [{ "ty": "timestamp" }],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;
    pretty_assertions::assert_eq!(
        composite,
        r#"{"output":{"items":[{"id":1,"attributes":{"description":null,"done":true,"title":"urgent_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]},{"id":2,"attributes":{"description":null,"done":false,"title":"open_todo"},"links":[{"created_at":"demo created_at","updated_at":"demo updated_at"}]}],"next_item":null}}"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn add_collection_int_float_array_and_filter_by_int() {
    let pool = Sqlite::in_memory_pool().await;
    let (client, ex) = Client::new_sqlx_db(pool);
    let client = client.into_string_client();
    let _executor = tokio::spawn(ex.run());

    client
        .exec(
            r#"
{
    "op": "add_collection",
    "body": {
        "name": "item",
        "fields": [
            { "name": "label", "type_info": "String", "is_optional": false },
            { "name": "priority", "type_info": "Int", "is_optional": false },
            { "name": "score", "type_info": "Float64", "is_optional": true },
            { "name": "tags", "type_info": { "ty": "Array", "of": "String" }, "is_optional": true }
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
    "op": "insert_one",
    "body": {
        "base": "item",
        "data": {
            "label": "low",
            "priority": 1,
            "score": 1.5,
            "tags": ["a", "b"]
        },
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
        "base": "item",
        "data": {
            "label": "high",
            "priority": 10,
            "score": 9.9,
            "tags": ["z"]
        },
        "links": []
    }
}
"#
            .to_string(),
        )
        .await;

    let result = client
        .exec(
            r#"
{
    "op": "fetch_many",
    "body": {
        "base": "item",
        "filters": [
            {
                "ty": "group",
                "filters": [
                    { "ty": "col_gt", "col": "priority", "gt": 5 },
                    { "ty": "col_gte", "col": "score", "gte": 9.0 }
                ]
            }
        ],
        "links": [],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
            .to_string(),
        )
        .await;

    pretty_assertions::assert_eq!(
        result,
        r#"{"output":{"items":[{"id":2,"attributes":{"label":"high","priority":10,"score":9.9,"tags":["z"]},"links":[]}],"next_item":null}}"#
    );
}

mod insert_one {
    use sqlx::Sqlite;

    use crate::{
        connect_in_memory::ConnectInMemory,
        json_client::client_interface::Client,
        track_sqlx_query::{assert_sql_eq, watch_sqlx_calls},
    };

    use crate::json_client::test_utilities::{
        add_category_collection, add_todo_collection, setup_todo_with_category_link,
        todo_is_one_to_many_with_category,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn set_id_links_existing_category() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "existing_category" },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_with_category",
            "done": true,
            "description": "linked"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"linked","done":true,"title":"todo_with_category"},"links":[{"id":1,"attributes":{"title":"existing_category"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn set_new_creates_category_and_links() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        let result = client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_with_new_category",
            "done": false,
            "description": "set_new"
        },
        "links": [
            { "ty": "set_new", "to": "category", "value": { "title": "new_category" } }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"set_new","done":false,"title":"todo_with_new_category"},"links":[{"id":1,"attributes":{"title":"new_category"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn set_id_sql() {
        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool);
                let client = client.into_string_client();

                actions.spawn(ex.run());

                setup_todo_with_category_link(&client, &actions).await;

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_for_set_id" },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Category\" (\"title\") VALUES ($1) RETURNING \"id\", \"title\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_set_id",
            "done": true
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Todo\" (\"title\", \"description\", \"done\", \"fk_category_def\") VALUES ($1, $2, $3, $4) RETURNING \"id\", \"title\", \"description\", \"done\", \"fk_category_def\";".to_string(),
                        "SELECT \"Category\".\"id\" AS \"iid\", \"Category\".\"title\" AS \"btitle\" FROM \"Category\" WHERE \"id\" = $1;".to_string(),
                    ]
                );
            })
            .await;
    }
}

mod update_one {
    use sqlx::Sqlite;

    use crate::{
        connect_in_memory::ConnectInMemory,
        json_client::client_interface::Client,
        track_sqlx_query::{assert_sql_eq, watch_sqlx_calls},
    };

    use crate::json_client::test_utilities::{
        add_category_collection, add_todo_collection, setup_todo_with_category_link,
        todo_is_one_to_many_with_category,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn set_id_links_existing_category() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "existing_category" },
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
        "base": "todo",
        "data": {
            "title": "todo_to_update",
            "done": false,
            "description": "before"
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "title": "linked_todo" },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"before","done":false,"title":"linked_todo"},"links":[{"id":1,"attributes":{"title":"existing_category"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn set_new_creates_category_and_links() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_to_update",
            "done": true,
            "description": "set_new"
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "description": "updated" },
        "links": [
            { "ty": "set_new", "to": "category", "value": { "title": "new_category" } }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"updated","done":true,"title":"todo_to_update"},"links":[{"id":1,"attributes":{"title":"new_category"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn empty_data_with_set_new_links_category() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_to_update",
            "done": true,
            "description": "set_new_only"
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": {},
        "links": [
            { "ty": "set_new", "to": "category", "value": { "title": "new_category" } }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"set_new_only","done":true,"title":"todo_to_update"},"links":[{"id":1,"attributes":{"title":"new_category"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn empty_data_without_set_contributing_links_returns_invalid_data() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_only",
            "done": false,
            "description": null
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": {},
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(result, r#"{"error":"InvalidData"}"#);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn set_null_clears_category_link() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_to_clear" },
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
        "base": "todo",
        "data": {
            "title": "todo_linked",
            "done": false,
            "description": "before_null"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": {},
        "links": [
            { "ty": "set_null", "to": "category" }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"before_null","done":false,"title":"todo_linked"},"links":[null]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn link_sql() {
        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool);
                let client = client.into_string_client();

                actions.spawn(ex.run());

                setup_todo_with_category_link(&client, &actions).await;

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_for_update_set_id" },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Category\" (\"title\") VALUES ($1) RETURNING \"id\", \"title\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_before_update",
            "done": false,
            "description": "desc"
        },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Todo\" (\"title\", \"description\", \"done\") VALUES ($1, $2, $3) RETURNING \"id\", \"title\", \"description\", \"done\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "title": "todo_after_update" },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        r#"UPDATE "Todo" SET "title" = $1, "fk_category_def" = $2 WHERE "Todo"."id" = $3 RETURNING "id", "title", "description", "done", "fk_category_def";"#
                            .to_string(),
                        r#"SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;"#
                            .to_string(),
                    ]
                );
            })
            .await;

        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool);
                let client = client.into_string_client();

                actions.spawn(ex.run());

                setup_todo_with_category_link(&client, &actions).await;

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_set_new",
            "done": true,
            "description": "desc"
        },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Todo\" (\"title\", \"description\", \"done\") VALUES ($1, $2, $3) RETURNING \"id\", \"title\", \"description\", \"done\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "description": "updated_desc" },
        "links": [
            { "ty": "set_new", "to": "category", "value": { "title": "new_cat" } }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Category\" (\"title\") VALUES ($1) RETURNING \"id\", \"title\";".to_string(),
                        "UPDATE \"Todo\" SET \"description\" = $1, \"fk_category_def\" = $2 WHERE \"Todo\".\"id\" = $3 RETURNING \"id\", \"title\", \"description\", \"done\", \"fk_category_def\";".to_string(),
                    ]
                );
            })
            .await;

        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool);
                let client = client.into_string_client();

                actions.spawn(ex.run());

                setup_todo_with_category_link(&client, &actions).await;

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_to_null" },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Category\" (\"title\") VALUES ($1) RETURNING \"id\", \"title\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_to_null",
            "done": false,
            "description": "desc"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Todo\" (\"title\", \"description\", \"done\", \"fk_category_def\") VALUES ($1, $2, $3, $4) RETURNING \"id\", \"title\", \"description\", \"done\", \"fk_category_def\";".to_string(),
                        "SELECT \"Category\".\"id\" AS \"iid\", \"Category\".\"title\" AS \"btitle\" FROM \"Category\" WHERE \"id\" = $1;".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": {},
        "links": [
            { "ty": "set_null", "to": "category" }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "UPDATE \"Todo\" SET \"fk_category_def\" = NULL WHERE \"Todo\".\"id\" = $1 RETURNING \"id\", \"title\", \"description\", \"done\", \"fk_category_def\";".to_string(),
                    ]
                );
            })
            .await;
    }
}

mod delete_one {
    use sqlx::Sqlite;

    use crate::{
        connect_in_memory::ConnectInMemory,
        json_client::client_interface::Client,
        track_sqlx_query::{assert_sql_eq, watch_sqlx_calls},
    };

    use crate::json_client::test_utilities::{
        add_category_collection, add_todo_collection, setup_todo_with_category_link,
        todo_is_one_to_many_with_category,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn deletes_todo_without_links() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_to_delete",
            "done": false,
            "description": "gone"
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "delete_one",
    "body": {
        "base": "todo",
        "id": 1,
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"gone","done":false,"title":"todo_to_delete"},"links":[]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn one_to_many_returns_category_fk() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        add_todo_collection(&client).await;
        add_category_collection(&client).await;
        todo_is_one_to_many_with_category(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_for_delete" },
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
        "base": "todo",
        "data": {
            "title": "linked_todo",
            "done": true,
            "description": "with_cat"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "delete_one",
    "body": {
        "base": "todo",
        "id": 1,
        "links": [
            { "ty": "one_to_many", "to": "category" }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"with_cat","done":true,"title":"linked_todo"},"links":[{"id":1,"attributes":{"title":"cat_for_delete"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn link_sql() {
        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool);
                let client = client.into_string_client();

                actions.spawn(ex.run());

                setup_todo_with_category_link(&client, &actions).await;

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "cat_for_delete_sql" },
        "links": []
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Category\" (\"title\") VALUES ($1) RETURNING \"id\", \"title\";".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_for_delete_sql",
            "done": false,
            "description": "desc"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        "INSERT INTO \"Todo\" (\"title\", \"description\", \"done\", \"fk_category_def\") VALUES ($1, $2, $3, $4) RETURNING \"id\", \"title\", \"description\", \"done\", \"fk_category_def\";".to_string(),
                        "SELECT \"Category\".\"id\" AS \"iid\", \"Category\".\"title\" AS \"btitle\" FROM \"Category\" WHERE \"id\" = $1;".to_string(),
                    ]
                );

                client
                    .exec(
                        r#"
{
    "op": "delete_one",
    "body": {
        "base": "todo",
        "id": 1,
        "links": [
            { "ty": "one_to_many", "to": "category" }
        ]
    }
}
"#
                        .to_string(),
                    )
                    .await;

                assert_sql_eq(
                    actions.take(),
                    vec![
                        r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."description" AS "bdescription", "Todo"."done" AS "bdone", "Category"."id" AS "lid", "Category"."title" AS "ltitle" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" WHERE "Todo"."id" = $1;"#
                            .to_string(),
                        r#"DELETE FROM "Todo" WHERE "Todo"."id" = $1 RETURNING "id", "title", "description", "done", "fk_category_def";"#
                            .to_string(),
                    ]
                );
            })
            .await;
    }
}

mod many_to_many {
    use sqlx::Sqlite;

    use crate::{connect_in_memory::ConnectInMemory, json_client::client_interface::Client};

    use crate::json_client::test_utilities::{
        add_tag_collection, add_todo_collection, todo_is_many_to_many_with_tag,
    };

    async fn setup_todo_tag_link(client: &crate::json_client::string_client::StringClient) {
        add_todo_collection(client).await;
        add_tag_collection(client).await;
        todo_is_many_to_many_with_tag(client).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn insert_set_id_links_existing_tag() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "todo_with_tag",
            "done": true,
            "description": "linked"
        },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"linked","done":true,"title":"todo_with_tag"},"links":[{"id":1,"attributes":{"title":"urgent"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_one_returns_linked_tags() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
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
        "base": "tag",
        "data": { "title": "home" },
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
        "base": "todo",
        "data": {
            "title": "todo_a",
            "done": true,
            "description": "a"
        },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 },
            { "ty": "set_id", "to": "tag", "id": 2 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "fetch_one",
    "body": {
        "base": "todo",
        "id": 1,
        "filters": [],
        "links": [
            { "ty": "many_to_many", "to": "tag" }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"a","done":true,"title":"todo_a"},"links":[{"many_output":[{"id":1,"attributes":{"title":"urgent"}},{"id":2,"attributes":{"title":"home"}}]}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_many_returns_linked_tags() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
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
        "base": "todo",
        "data": {
            "title": "todo_a",
            "done": true,
            "description": "a"
        },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [],
        "links": [
            { "ty": "many_to_many", "to": "tag" }
        ],
        "pagination": { "limit": 10, "first_item": null, "order_by": [] }
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"items":[{"id":1,"attributes":{"description":"a","done":true,"title":"todo_a"},"links":[{"many_output":[{"id":1,"attributes":{"title":"urgent"}}]}]}],"next_item":null}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn update_set_id_adds_tag_link() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
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
        "base": "todo",
        "data": {
            "title": "todo_to_update",
            "done": false,
            "description": "before"
        },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "title": "linked_todo" },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"before","done":false,"title":"linked_todo"},"links":[{"id":1,"attributes":{"title":"urgent"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn update_remove_id_removes_tag_link() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
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
        "base": "tag",
        "data": { "title": "home" },
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
        "base": "todo",
        "data": {
            "title": "todo_linked",
            "done": false,
            "description": "with_tags"
        },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 },
            { "ty": "set_id", "to": "tag", "id": 2 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": {},
        "links": [
            { "ty": "remove_id", "to": "tag", "id": 1 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"with_tags","done":false,"title":"todo_linked"},"links":[{"id":1,"attributes":{"title":"urgent"}}]}}"#
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn delete_returns_linked_tag_ids() {
        let pool = Sqlite::in_memory_pool().await;
        let (client, ex) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();
        let _executor = tokio::spawn(ex.run());

        setup_todo_tag_link(&client).await;

        client
            .exec(
                r#"
{
    "op": "insert_one",
    "body": {
        "base": "tag",
        "data": { "title": "urgent" },
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
        "base": "tag",
        "data": { "title": "home" },
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
        "base": "todo",
        "data": {
            "title": "linked_todo",
            "done": true,
            "description": "with_tags"
        },
        "links": [
            { "ty": "set_id", "to": "tag", "id": 1 },
            { "ty": "set_id", "to": "tag", "id": 2 }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        let result = client
            .exec(
                r#"
{
    "op": "delete_one",
    "body": {
        "base": "todo",
        "id": 1,
        "links": [
            { "ty": "many_to_many", "to": "tag" }
        ]
    }
}
"#
                .to_string(),
            )
            .await;

        pretty_assertions::assert_eq!(
            result,
            r#"{"output":{"id":1,"attributes":{"description":"with_tags","done":true,"title":"linked_todo"},"links":[{"many_output":[{"id":1,"attributes":{"title":"urgent"}},{"id":2,"attributes":{"title":"home"}}]}]}}"#
        );
    }
}
mod comprehensive {
    use sqlx::Sqlite;

    mod assert_helpers {
        use crate::gen_serde::pretty_json;

        pub fn pretty_exec_output(value: impl AsRef<str>) -> String {
            pretty_json(value.as_ref())
        }

        pub fn pretty_sql(value: impl AsRef<str>) -> String {
            value
                .as_ref()
                .split(';')
                .filter_map(|statement| {
                    let statement = statement.split_whitespace().collect::<Vec<_>>().join(" ");
                    if statement.is_empty() || statement.starts_with("PRAGMA ") {
                        None
                    } else {
                        Some(format!("{statement};"))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }

        pub fn assert_exec_eq(actual: impl AsRef<str>, expected: impl AsRef<str>) {
            pretty_assertions::assert_eq!(pretty_exec_output(actual), pretty_exec_output(expected),);
        }

        pub fn assert_sql_drain(drain: Vec<String>, expected: impl AsRef<str>) {
            pretty_assertions::assert_eq!(
                pretty_sql(crate::track_sqlx_query::without_pragma(drain).join("\n"),),
                pretty_sql(expected),
            );
        }
    }

    use assert_helpers::{assert_exec_eq, assert_sql_drain};

    use crate::{
        connect_in_memory::ConnectInMemory,
        json_client::client_interface::Client,
        track_sqlx_query::{assert_sql_eq, watch_sqlx_calls},
    };

    use super::super::test_utilities::{
        add_category_collection, add_tag_collection, add_todo_collection, clear_timestams,
        todo_is_many_to_many_with_tag, todo_is_one_to_many_with_category, todo_is_timestamped,
    };

    /// End-to-end walkthrough with readable JSON/SQL diffs via pretty helpers.
    #[tokio::test(flavor = "current_thread")]
    async fn all_crud_operations_with_sql() {
        watch_sqlx_calls(async |actions| {
                let pool = Sqlite::in_memory_pool().await;
                let (client, ex) = Client::new_sqlx_db(pool.clone());
                let client = client.into_string_client();
                actions.spawn(ex.run());

                add_todo_collection(&client).await;
                assert_sql_drain(
                    actions.take(),
                    r#"
PRAGMA foreign_keys = ON;
CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "description" TEXT, "done" BOOLEAN NOT NULL);
"#,
                );

                add_category_collection(&client).await;
                assert_sql_drain(
                    actions.take(),
                    r#"CREATE TABLE "Category" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);"#,
                );

                add_tag_collection(&client).await;
                assert_sql_drain(
                    actions.take(),
                    r#"CREATE TABLE "Tag" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);"#,
                );

                todo_is_timestamped(&client).await;
                actions.take();

                todo_is_one_to_many_with_category(&client).await;
                assert_sql_drain(
                    actions
                        .take()
                        .into_iter()
                        .filter(|sql| sql.contains("fk_category_def"))
                        .collect(),
                    r#"ALTER TABLE "Todo" ADD COLUMN "fk_category_def" INTEGER REFERENCES "Category"("id") ON DELETE SET NULL;"#,
                );

                todo_is_many_to_many_with_tag(&client).await;
                assert_sql_drain(
                    actions.take(),
                    r#"CREATE TABLE "ct_todo_tag_def" ("todo_id" INTEGER NOT NULL  REFERENCES "Todo"("id") ON DELETE CASCADE, "tag_id" INTEGER NOT NULL  REFERENCES "Tag"("id") ON DELETE CASCADE, PRIMARY KEY ("todo_id", "tag_id"));"#,
                );

                actions.clear();

                assert_exec_eq(
                    client
                        .exec(
                            r#"
{
    "op": "insert_one",
    "body": {
        "base": "category",
        "data": { "title": "work" },
        "links": []
    }
}
"#
                            .to_string(),
                        )
                        .await,
                    r#"
{
    "output": {
        "id": 1,
        "attributes": { "title": "work" },
        "links": []
    }
}
"#,
                );
                assert_sql_drain(
                    actions.take(),
                    r#"INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";"#,
                );

                assert_exec_eq(
                    client
                        .exec(
                            r#"
{
    "op": "insert_one",
    "body": {
        "base": "todo",
        "data": {
            "title": "alpha",
            "done": true,
            "description": "a"
        },
        "links": [
            { "ty": "set_id", "to": "category", "id": 1 }
        ]
    }
}
"#
                            .to_string(),
                        )
                        .await,
                    r#"
{
    "output": {
        "id": 1,
        "attributes": {
            "description": "a",
            "done": true,
            "title": "alpha"
        },
        "links": [
            { "id": 1, "attributes": { "title": "work" } }
        ]
    }
}
"#,
                );
                assert_sql_drain(
                    actions.take(),
                    r#"
INSERT INTO "Todo" ("title", "description", "done", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "description", "done", "fk_category_def";
SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;
"#,
                );

                clear_timestams(pool.clone()).await;

                assert_exec_eq(
                    client
                        .exec(
                            r#"
{
    "op": "fetch_one",
    "body": {
        "base": "todo",
        "id": 1,
        "filters": [
            { "ty": "col_eq", "col": "done", "eq": true }
        ],
        "links": [
            { "ty": "one_to_many", "to": "category" }
            { "ty": "many_to_many", "to": "tag" }
            { "ty": "timestamp" }
        ]
    }
}
"#
                            .to_string(),
                        )
                        .await,
                    r#"
{
    "output": {
        "id": 1,
        "attributes": {
            "description": "a",
            "done": true,
            "title": "alpha"
        },
        "links": [
            { "id": 1, "attributes": { "title": "work" } },
            { "many_output": [] },
            {
                "created_at": "demo created_at",
                "updated_at": "demo updated_at"
            }
        ]
    }
}
"#,
                );
                assert_sql_drain(
                    actions.take(),
                    r#"
UPDATE "Todo" SET "created_at" = "demo created_at", "updated_at" = "demo updated_at";
SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."description" AS "bdescription", "Todo"."done" AS "bdone", "Category"."id" AS "l0id", "Category"."title" AS "l0title", "Todo"."id" AS "l1id", "Todo"."created_at" AS "l2created_at", "Todo"."updated_at" AS "l2updated_at" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" WHERE "Todo"."id" = $1 AND "Todo"."done" = $2;
SELECT "ct_todo_tag_def"."todo_id" AS "from_id", "ct_todo_tag_def"."tag_id" AS "to_id", "Tag"."title" AS "t_title" FROM "ct_todo_tag_def" INNER JOIN "Tag" ON "ct_todo_tag_def"."tag_id" = "Tag"."id" WHERE "todo_id" IN ($1);
"#,
                );
            })
            .await;
    }
}
