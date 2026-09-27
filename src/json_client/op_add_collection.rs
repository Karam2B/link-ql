use std::{future::Future, sync::Arc};

use sqlx::IntoArguments;

use crate::{
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    json_client::{
        client_interface::{AddCollectionError, AddCollectionInput, AddCollectionOutput},
        dynamic_collection::{DynamicCollection, FieldName},
        sqlx_executor::SqlxExecutorData,
    },
    operations::operations_expressions_crossover::MigrateExpression,
    sqlx_query_builder::{Expression, StatementBuilder},
};

pub fn add_collection<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: AddCollectionInput,
) -> impl Future<Output = Result<AddCollectionOutput, AddCollectionError>> + 'static + Send + use<S>
where
    S: Sync + DatabaseExt + ExecutorTrait,
    bool: for<'d> sqlx::Decode<'d, S> + sqlx::Type<S> + for<'q> sqlx::Encode<'q, S>,
    std::string::String: sqlx::Type<S> + for<'q> sqlx::Encode<'q, S> + for<'d> sqlx::Decode<'d, S>,
    i64: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    f64: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    sqlx::types::Json<Vec<String>>:
        for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    sqlx::types::Json<Vec<bool>>:
        for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    sqlx::types::Json<Vec<i64>>:
        for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    sqlx::types::Json<Vec<f64>>:
        for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    DynamicCollection<S>: MigrateExpression<Migrate: Expression<'static, S>>,
    for<'a> &'a str: sqlx::ColumnIndex<<S as sqlx::Database>::Row>,
    for<'a> S::Arguments<'a>: IntoArguments<'a, S>,
{
    async move {
        let dc = DynamicCollection::try_from(input)
            .map_err(|_| AddCollectionError::InvalidCollectionInput)?;

        let collection_key: Arc<str> = Arc::clone(&dc.collection_name.snake_case);

        {
            let collections = this.collections.read().await;
            if collections.get(collection_key.as_ref()).is_some() {
                return Err(AddCollectionError::CollectionAlreadyExists);
            }
        }

        let mig = StatementBuilder::<S>::new_no_data(dc.migrate()).expect("bug: {}");

        let mut conn = this.pool.acquire().await.expect("bug: {}");

        S::execute(&mut conn, mig.as_str())
            .await
            .expect("bug: migration should never fail");

        let mut collections = this.collections.write().await;
        let mut migration = this.migration.write().await;

        migration.push(mig);
        collections.insert(collection_key, tokio::sync::RwLock::new(Arc::new(dc)));

        Ok(())
    }
}
