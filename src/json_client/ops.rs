use super::client_interface::*;
use super::op_fetch_many_trait_extension::JsonLinkFetchMany;
use super::op_fetch_one_trait_extension::JsonLinkFetchOne;

macro_rules! ops {
        ($([$snake_case:ident, $pascal_case:ident]),*) => {
            paste::paste!{
                #[derive(Debug)]
                #[non_exhaustive]
                pub enum OperationOutput {
                    $(
                        $pascal_case ([<$pascal_case Output>]),
                    )*
                }

                #[derive(Debug)]
                #[non_exhaustive]
                pub enum OperationInput {
                    $(
                        $pascal_case ([<$pascal_case Input>]),
                    )*
                }

                #[derive(Debug)]
                #[non_exhaustive]
                pub enum OperationError {
                    $(
                        $pascal_case ([<$pascal_case Error>]),
                    )*
                }

                impl Client {
                    $(
                        pub fn
                            $snake_case(&self, input: [<$pascal_case Input>])
                        -> impl Future<Output = Result<[<$pascal_case Output>], [<$pascal_case Error>]>> {
                            let (tx, rx) = oneshot::async_channel::<Result<OperationOutput, OperationError>>();
                            self.sender.send((OperationInput::$pascal_case(input), tx)).unwrap();
                            async move {
                                let output = rx.await.expect("disconnected_channel");
                                let mapp = match output {
                                    Ok(OperationOutput::$pascal_case(e)) => Ok(e),
                                    Err(OperationError::$pascal_case(e)) => Err(e),
                                    _ => panic!("invalid mapping"),
                                };

                                return mapp;
                            }
                        }
                    )*
                }

                impl<S> $crate::json_client::sqlx_executor::SqlxExecutor<S>
                where
                    S: ::std::marker::Sync,
                    S: $crate::fix_executor::ExecutorTrait,
                    S: $crate::database_extention::DatabaseExt,
                    bool: for<'d> ::sqlx::Decode<'d, S> + ::sqlx::Type<S> + for<'q> ::sqlx::Encode<'q, S>,
                    std::string::String:
                        ::sqlx::Type<S> + for<'q> ::sqlx::Encode<'q, S> + for<'d> ::sqlx::Decode<'d, S>,
                    $crate::json_client::dynamic_collection::DynamicCollection<S>:
                        $crate::operations::operations_expressions_crossover::MigrateExpression<Migrate: $crate::sqlx_query_builder::Expression<'static, S>>,
                    for<'a> S::Arguments<'a>: ::sqlx::IntoArguments<'a, S>,
                    for<'a> &'a str: ::sqlx::ColumnIndex<<S as ::sqlx::Database>::Row>,
                    i64: for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    f64: for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    ::sqlx::types::Json<Vec<std::string::String>>:
                        for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    ::sqlx::types::Json<Vec<bool>>:
                        for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    ::sqlx::types::Json<Vec<i64>>:
                        for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    ::sqlx::types::Json<Vec<f64>>:
                        for<'r> ::sqlx::Decode<'r, S>
                        + for<'q> ::sqlx::Encode<'q, S>
                        + ::sqlx::Type<S>,
                    $crate::links::relation_one_to_many::OneToMany<
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_many_trait_extension::JsonLinkFetchMany<S>,
                    $crate::links::relation_one_to_many_inverse::OneToManyInverse<
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_many_trait_extension::JsonLinkFetchMany<S>,
                    $crate::links::relation_many_to_many::ManyToMany<
                        false,
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_many_trait_extension::JsonLinkFetchMany<S>,
                    $crate::links::timestamp::Timestamp<
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_many_trait_extension::JsonLinkFetchMany<S>,
                    $crate::links::relation_one_to_many::OneToMany<
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_one_trait_extension::JsonLinkFetchOne<S>,
                    $crate::links::relation_one_to_many_inverse::OneToManyInverse<
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_one_trait_extension::JsonLinkFetchOne<S>,
                    $crate::links::relation_many_to_many::ManyToMany<
                        false,
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_one_trait_extension::JsonLinkFetchOne<S>,
                    $crate::links::timestamp::Timestamp<
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::json_client::op_fetch_one_trait_extension::JsonLinkFetchOne<S>,
                    $crate::links::relation_one_to_many::OneToMany<
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::operations::operations_expressions_crossover::MigrateExpression<
                        Migrate: for<'q> $crate::sqlx_query_builder::Expression<'q, S>,
                    >,
                    $crate::links::relation_many_to_many::ManyToMany<
                        false,
                        $crate::links::DefaultRelationKey,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::operations::operations_expressions_crossover::MigrateExpression<
                        Migrate: for<'q> $crate::sqlx_query_builder::Expression<'q, S>,
                    >,
                    $crate::links::timestamp::Timestamp<
                        std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>,
                    >: $crate::operations::operations_expressions_crossover::MigrateExpression<
                        Migrate: for<'q> $crate::sqlx_query_builder::Expression<'q, S>,
                    >,
                    std::sync::Arc<$crate::json_client::dynamic_collection::DynamicCollection<S>>:
                        for<'r> $crate::from_row::FromRowAlias<
                            'r,
                            S::Row,
                            RData = $crate::json_client::dynamic_collection::CollectionToSerialize,
                        >,

                {
                    pub fn run(mut self) -> impl Future<Output = ::std::convert::Infallible> {
                        async move {
                            loop {
                                let operation = self.reciever.recv().await.unwrap();

                                paste::paste!{
                                match operation.0 {
                                    $(OperationInput::$pascal_case(input) => {
                                        let future = $crate::json_client::[<op_ $snake_case>]::[<$snake_case>](self.data.clone(), input);
                                        let operation_sender = operation.1;
                                        let dispatch = ::tracing::dispatcher::Dispatch::default();
                                        let span = ::tracing::Span::current();
                                        tokio::spawn(async move {
                                            let _dispatch =
                                                ::tracing::dispatcher::set_default(&dispatch);
                                            let _span = span.enter();
                                            operation_sender
                                                .send(
                                                    future
                                                        .await
                                                        .map(OperationOutput::$pascal_case)
                                                        .map_err(OperationError::$pascal_case),
                                                )
                                                .unwrap();
                                        });
                                    })*
                                }}
                            }
                        }
                    }
                }

                impl $crate::json_client::string_client::StringClient {
                    pub fn exec(&self, input: String) -> impl Future<Output = String> {
                        use crate::gen_serde::Serialize;
                        use crate::gen_serde::Deserialize;
                        use crate::gen_serde::json_format_side::JsonAsArcCursor;

                        async move {
                            let mut cursor = JsonAsArcCursor {
                                inner: input.as_str().into(),
                                start: 0,
                            };

                            let input = match $crate::json_client::string_client::StringClientInput::deserialize((), &mut cursor) {
                                Ok(input) => input,
                                Err(e) => return String::from(r#"{"error":"invalid_input"}"#),
                            };

                            match input.op.as_str() {
                                $(
                                    stringify!($snake_case) => {
                                        let body = match input.body.continue_deserialize() {
                                            Ok(body) => body,
                                            Err(e) => return String::from(r#"{"error":"invalid_body"}"#),
                                        };
                                        let output = match self.inner.$snake_case(body).await {
                                            Ok(output) => output,
                                            Err(e) => return format!("{{\"error\":\"{:?}\"}}", e),
                                        };

                                        let mut serialized = $crate::gen_serde::json_serialize_side::JsonAsString(String::new());
                                        $crate::json_client::string_client::StringClientOutput { output }.serialize(&mut serialized);

                                        return serialized.0;
                                    }
                                )*
                                _ => return String::from(r#"{"error":"unsupported_operation"}"#),
                            }
                        }
                    }
                }
            }
        };
    }

ops!(
    [add_collection, AddCollection],
    [add_link, AddLink],
    [fetch_many, FetchMany],
    [fetch_one, FetchOne],
    [insert_one, InsertOne],
    [insert_many, InsertMany],
    [update_one, UpdateOne],
    [delete_one, DeleteOne]
);
