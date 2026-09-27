# LinkedSql

Robust, zero-cost and flexable ORM. Written in idomatic Rust .

**Main Example**


```Rust
    client.exec(r#"
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
    "#).await;

    client.exec(r#"
        {
            "op": "add_collection",
            "body": {
                "name": "category",
                "fields": [
                    { "name": "title", "type_info": "String", "is_optional": false }
                ]
            }
        }
    "#).await;

    client.exec(r#"
        {
            "op": "add_link",
            "body": {
                "ty": "one_to_many",
                "from": "todo",
                "to": "category"
            }
        }
    "#).await;

    { ... some dumpy data }

    let result = client.exec(r#"
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
                    "first_item": { "id": 3 }, 
                    "order_by": [] 
                }
            }
        }
    "#).await;

    assert_exec_output(result, r#"
        {
            "output":{
                "items": [
                    {
                        "id": 3,
                        "attributes": { 
                            "title": "third_todo", 
                            "description": "description", 
                            "done": false 
                        },
                        "links": [ 
                            { "id": 2, "attributes": { "title": "cat_2" } },
                        ]
                    },
                    {
                        "id": 5,
                        "attributes": { 
                            "title": "fifth_todo", 
                            "description": "description", 
                            "done": false 
                        },
                        "links": [ 
                            null
                        ]
                    },
                    {
                        "id": 6,
                        "attributes": { 
                            "title": "sixth_todo", 
                            "description": "description", 
                            "done": true 
                        },
                        "links": [ 
                            null
                        ]
                    },
                ],
                "next_item": { "id": 8 }
            }
        }
    "#);
```

## Zero-Cost
The previous example uses `JsonClient` which is suitable for creating and modifing collection at a runtime, this is done by relying on storing many trait objects on the heap. 

If you don't want to put up with that cost and know your schema at the build time you can use `linked_ql_macros`, this example will give you almost identical performance compared to writing sql statement by hand

```Rust
define_collection!(
    struct Todo {
        title: String,
        description: Option<String>,
        done: bool,
    }
);

define_collection!(
    struct Category {
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

#[tokio::test]
async fn test_zero_cost() {
    { ... initialize sqlx and load dumpy data }

    let todo = fetch_many(FetchManyOp {
        base: TodoHandler,
        filters: ColumnLikeFilter {
            col: todo_members::TitleField,
            value: Bind("i"),
        },
        links: CategoryHandler,
        pagination: Pagination {
            limit: 10,
            offset: 0,
            order_by: (),
        },
    }, &mut conn).await;

    assert_eq!(todo, ManyOutput {
        items: vec![
            LinkedOutput {
                id: 3,
                attributes: Todo {
                    title: "third_todo".to_string(),
                    description: Some("description".to_string()),
                    done: false,
                },
                links: Some(
                    CollectionOutput {
                        id: 2,
                        attributes: Category {
                            title: "cat_2".to_string(),
                        },
                    }
                )
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
            }
        ],
        next_item: NextItem {
            id: 8,
            other_fields: ()
        },
    });
}
```

These two examples are picked from "test" folder, you can check the complete code there.

## Initialize
This crate depends on a patched version of sqlx, use `patch-crate`
```
    cargo install patch-crate       # if you don't have the crate downloaed
    cargo patch-crate               # initialize target/patch/* patched crates
                                    # using original source code and /patches modification
    cargo check                     # now cargo commands work
```

