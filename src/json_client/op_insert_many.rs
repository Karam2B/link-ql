use std::sync::Arc;

use sqlx::ColumnIndex;

use crate::{
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    gen_serde::{
        Deserialize, deserialize,
        json_format_side::{JsonAsArcCursor, JsonFormat},
    },
    json_client::{
        DynManyToMany, DynOneToMany,
        client_interface::{InsertManyError, InsertManyInput, InsertManyOutput, InsertOneError},
        dynamic_collection::{CollectionToSerialize, DynamicCollection, DynamicInsertInput},
        op_insert_one::exec_insert_one,
        op_insert_one_trait_extension::{JsonInsertOneLink, JsonInsertOneToConsume},
        sqlx_executor::SqlxExecutorData,
    },
    links::update_links::{SetId, SetNew},
    operations::insert::{InsertLinkConsumeData, InsertOneLink},
};

type DynCollection<S> = Arc<DynamicCollection<S>>;

pub fn insert_many<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: InsertManyInput,
) -> impl Future<Output = Result<InsertManyOutput, InsertManyError>> + 'static + Send + use<S>
where
    i64: sqlx::Type<S> + for<'q> sqlx::Decode<'q, S> + for<'q> sqlx::Encode<'q, S>,
    for<'a> &'a str: sqlx::ColumnIndex<<S as sqlx::Database>::Row>,
    S: sqlx::Database + DatabaseExt + ExecutorTrait + Send + Sync + 'static,
    DynCollection<S>: for<'r> FromRowAlias<'r, S::Row, RData = CollectionToSerialize>,
    DynamicInsertInput<S>: for<'d> Deserialize<'d, JsonAsArcCursor, Handler = DynCollection<S>>,
    SetId<DynOneToMany<S>, i64>: InsertLinkConsumeData<
        Link: JsonInsertOneLink<S>
                  + InsertOneLink<InsertValuesData: Send, PreOpData: Send, PostOpData: Send>,
    >,
    SetId<DynManyToMany<S>, i64>: InsertLinkConsumeData<
        Link: JsonInsertOneLink<S>
                  + InsertOneLink<InsertValuesData: Send, PreOpData: Send, PostOpData: Send>,
    >,
    SetNew<DynOneToMany<S>, DynamicInsertInput<S>>: InsertLinkConsumeData<
        Link: JsonInsertOneLink<S>
                  + InsertOneLink<InsertValuesData: Send, PreOpData: Send, PostOpData: Send>,
    >,
    Vec<JsonInsertOneToConsume<S>>:
        InsertLinkConsumeData<Link = Vec<Box<dyn JsonInsertOneLink<S> + Send>>>,
{
    async move {
        if input.items.is_empty() {
            return Err(InsertManyError::InvalidData);
        }

        let cols = this.collections.read().await;
        let base_gaurd = cols
            .get(input.base.as_str())
            .ok_or(InsertManyError::CollectionNotFound)?
            .read()
            .await;
        let base = base_gaurd.clone();

        let mut conn = this.pool.acquire().await.unwrap();
        let mut items = Vec::with_capacity(input.items.len());

        for item in input.items {
            let data: DynamicInsertInput<S> = deserialize(
                Arc::from(item.data.0.as_str()),
                Arc::clone(&base),
                JsonFormat,
            )
            .map_err(|_| InsertManyError::InvalidData)?;

            let out = exec_insert_one(&this, Arc::clone(&base), data, item.links, &mut conn)
                .await
                .map_err(|err| match err {
                    InsertOneError::CollectionNotFound => InsertManyError::CollectionNotFound,
                    InsertOneError::InvalidData => InsertManyError::InvalidData,
                    InsertOneError::InvalidLink => InsertManyError::InvalidLink,
                    InsertOneError::LinkNotSetUpForThisBase => {
                        InsertManyError::LinkNotSetUpForThisBase
                    }
                })?;

            items.push(out);
        }

        drop(base_gaurd);
        drop(cols);

        Ok(InsertManyOutput { items })
    }
}
