//! End-to-end `StringClient` workflow: schema setup, inserts, updates, and paginated fetch.
//!
//! Run with: `cargo run --example todo_category_workflow`

use linked_sql::connect_in_memory::ConnectInMemory;
use linked_sql::json_client::client_interface::Client;
use linked_sql::track_sqlx_query::{watch_sqlx_calls, without_pragma};
use sqlx::Sqlite;

/// Pretty-print JSON op output for readable assertion diffs.
fn pretty_op_output(value: impl AsRef<str>) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(value.as_ref()).expect("op output must be valid JSON");
    serde_json::to_string_pretty(&parsed).expect("op output must be pretty-printable")
}

/// Normalize SQL for readable assertion diffs (collapse whitespace, drop PRAGMA).
fn pretty_sql_query(value: impl AsRef<str>) -> String {
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

fn assert_op_eq(actual: impl AsRef<str>, expected: impl AsRef<str>) {
    assert_eq!(pretty_op_output(actual), pretty_op_output(expected));
}

fn assert_sql_drain(drain: Vec<String>, expected: impl AsRef<str>) {
    assert_eq!(
        pretty_sql_query(without_pragma(drain).join("\n")),
        pretty_sql_query(expected),
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    watch_sqlx_calls(async |actions| {
        let pool = Sqlite::in_memory_pool().await;
        let (client, executor) = Client::new_sqlx_db(pool);
        let client = client.into_string_client();

        actions.spawn(executor.run());

        assert_op_eq(
            client
                .exec(
                    r#"
{
    "op": "add_collection",
    "body": {
        "name": "todo",
        "fields": [
            { "name": "title", "type_info": "String", "is_optional": false },
            { "name": "description", "type_info": "String", "is_optional": true },
            { "name": "done", "type_info": "Boolean", "is_optional": false }
        ]
    }
}
"#
                    .to_string(),
                )
                .await,
            r#"{"output":null}"#,
        );
        assert_sql_drain(
            actions.take(),
            r#"CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "description" TEXT, "done" BOOLEAN NOT NULL);"#,
        );

        assert_op_eq(
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
                .await,
            r#"{"output":null}"#,
        );
        assert_sql_drain(
            actions.take(),
            r#"CREATE TABLE "Category" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);"#,
        );

        assert_op_eq(
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
                .await,
            r#"{"output":null}"#,
        );
        assert_sql_drain(
            actions.take(),
            r#"ALTER TABLE "Todo" ADD COLUMN "fk_category_def" INTEGER  REFERENCES "Category"("id") ON DELETE SET NULL;"#,
        );

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
        "base": "category",
        "data": { "title": "personal" },
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
        "data": { "title": "alpha", "done": false, "description": "first" },
        "links": [{ "ty": "set_id", "to": "category", "id": 1 }]
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
        "data": { "title": "bravo", "done": true, "description": "second" },
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
        "data": { "title": "charlie", "done": false, "description": "third" },
        "links": [{ "ty": "set_id", "to": "category", "id": 2 }]
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
        "data": { "title": "delta", "done": true, "description": "fourth" },
        "links": [
            { "ty": "set_new", "to": "category", "value": { "title": "shopping" } }
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
        "data": { "title": "echo", "done": false, "description": "fifth" },
        "links": []
    }
}
"#
                .to_string(),
            )
            .await;

        assert_sql_drain(
            actions.take(),
            r#"
INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";
INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";
INSERT INTO "Todo" ("title", "description", "done", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "description", "done", "fk_category_def";
SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;
INSERT INTO "Todo" ("title", "description", "done") VALUES ($1, $2, $3) RETURNING "id", "title", "description", "done";
INSERT INTO "Todo" ("title", "description", "done", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "description", "done", "fk_category_def";
SELECT "Category"."id" AS "iid", "Category"."title" AS "btitle" FROM "Category" WHERE "id" = $1;
INSERT INTO "Category" ("title") VALUES ($1) RETURNING "id", "title";
INSERT INTO "Todo" ("title", "description", "done", "fk_category_def") VALUES ($1, $2, $3, $4) RETURNING "id", "title", "description", "done", "fk_category_def";
INSERT INTO "Todo" ("title", "description", "done") VALUES ($1, $2, $3) RETURNING "id", "title", "description", "done";
"#,
        );

        assert_op_eq(
            client
                .exec(
                    r#"
{
    "op": "update_one",
    "body": {
        "base": "todo",
        "id": 1,
        "data": { "done": true, "description": "marked done" },
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
        "attributes": {
            "description": "marked done",
            "done": true,
            "title": "alpha"
        },
        "links": []
    }
}
"#,
        );

        assert_sql_drain(
            actions.take(),
            r#"UPDATE "Todo" SET "done" = $1, "description" = $2 WHERE "Todo"."id" = $3 RETURNING "id", "title", "description", "done";"#,
        );

        assert_op_eq(
            client
                .exec(
                    r#"
{
    "op": "fetch_many",
    "body": {
        "base": "todo",
        "filters": [],
        "links": [
            { "ty": "one_to_many", "to": "category" }
        ],
        "pagination": {
            "limit": 3,
            "first_item": null,
            "order_by": [
                { "col": "title", "direction": "asc" }
            ]
        }
    }
}
"#
                    .to_string(),
                )
                .await,
            r#"
{
    "output": {
        "items": [
            {
                "id": 1,
                "attributes": {
                    "description": "marked done",
                    "done": true,
                    "title": "alpha"
                },
                "links": [
                    { "id": 1, "attributes": { "title": "work" } }
                ]
            },
            {
                "id": 2,
                "attributes": {
                    "description": "second",
                    "done": true,
                    "title": "bravo"
                },
                "links": [null]
            },
            {
                "id": 3,
                "attributes": {
                    "description": "third",
                    "done": false,
                    "title": "charlie"
                },
                "links": [
                    { "id": 2, "attributes": { "title": "personal" } }
                ]
            }
        ],
        "next_item": {
            "id": 4,
            "attributes": { "title": "delta" }
        }
    }
}
"#,
        );
        assert_sql_drain(
            actions.take(),
            r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."description" AS "bdescription", "Todo"."done" AS "bdone", "Category"."id" AS "l0id", "Category"."title" AS "l0title" FROM "Todo" LEFT JOIN "Category" ON "Todo"."fk_category_def" = "Category"."id" ORDER BY "todo"."title" ASC LIMIT $1;"#,
        );
    })
    .await;
}
