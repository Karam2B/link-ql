    use sqlx::{Pool, Sqlite};

    use crate::{
        connect_in_memory::ConnectInMemory,
        json_client::{client_interface::Client, string_client::StringClient},
        track_sqlx_query::{Actions, assert_sql_eq, watch_sqlx_calls},
    };

    pub async fn setup_todo_collection(sc: &StringClient, actions: &Actions) {
        add_todo_collection(sc).await;
        actions.clear();
    }

    pub async fn setup_todo_with_category_link(sc: &StringClient, actions: &Actions) {
        add_todo_collection(sc).await;
        add_category_collection(sc).await;
        todo_is_one_to_many_with_category(sc).await;
        actions.clear();
    }

    pub async fn add_todo_collection(sc: &StringClient) {
        // add_collection
        sc.exec(
            r#"
        {
            "op": "add_collection",
            "body": {
                "name": "todo",
                "fields": [
                    { "name": "title", "type_info": "String", "is_optional": false }
                    { "name": "description", "type_info": "String", "is_optional": true }
                    { "name": "done", "type_info": "Boolean", "is_optional": false }
                ]
            }
        }
        "#
            .to_string(),
        )
        .await;
    }

    pub async fn add_category_collection(sc: &StringClient) {
        sc.exec(
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
    }

    pub async fn add_tag_collection(sc: &StringClient) {
        sc.exec(
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
        .await;
    }

    pub async fn todo_is_timestamped(sc: &StringClient) {
        sc.exec(
            r#"
        {
            "op": "add_link",
            "body": {
                "ty": "timestamp",
                "collection": "todo"
            }
        }
        "#
            .to_string(),
        )
        .await;
    }

    pub async fn category_is_timestamped(sc: &StringClient) {
        sc.exec(
            r#"
        {
            "op": "add_link",
            "body": {
                "ty": "timestamp",
                "collection": "category"
            }
        }
        "#
            .to_string(),
        )
        .await;
    }

    pub async fn todo_is_one_to_many_with_category(sc: &StringClient) {
        sc.exec(
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
    }

    pub async fn todo_is_many_to_many_with_tag(sc: &StringClient) {
        sc.exec(
            r#"
        {
            "op": "add_link",
            "body": {
                "ty": "many_to_many",
                "from": "todo",
                "to": "tag"
            }
        }
        "#
            .to_string(),
        )
        .await;
    }

    /// use this utility function to make assertions between queries easier
    pub async fn clear_timestams(pool: Pool<Sqlite>) {
        sqlx::query(
            r#"
            UPDATE "Todo" SET "created_at" = "demo created_at", "updated_at" = "demo updated_at";
            "#,
        )
        .execute(&pool)
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_todo_app() {
        watch_sqlx_calls(async |actions| {
            let pool = Sqlite::in_memory_pool().await;
            let (client, ex) = Client::new_sqlx_db(pool);

            let client = client.into_string_client();

            actions.spawn(ex.run());

            add_todo_collection(&client).await;

            assert_sql_eq(
                actions.take(),
                vec![
                    r#"PRAGMA foreign_keys = ON;"#.to_string(),
                    r#"CREATE TABLE "Todo" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL, "description" TEXT, "done" BOOLEAN NOT NULL);"#.to_string(),
                ]
            );

            add_category_collection(&client).await;
            // add_tag_collection(&client).await;

            assert_sql_eq(
                actions.take(),
                vec![r#"CREATE TABLE "Category" ("id" INTEGER PRIMARY KEY AUTOINCREMENT, "title" TEXT NOT NULL);"#.to_string()]
            );

            todo_is_one_to_many_with_category(&client).await;
            // todo_is_many_to_many_with_tag(&client).await;

            assert_sql_eq(
                actions.take(),
                vec![
                    r#"ALTER TABLE "Todo" ADD COLUMN "fk_category_def" INTEGER REFERENCES "Category"("id") ON DELETE SET NULL;"#.to_string(),
                ]
            );
        })
        .await;
    }
