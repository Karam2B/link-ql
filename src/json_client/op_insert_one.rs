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
        client_interface::{
            InsertManyError, InsertManyInput, InsertManyItem, InsertManyOutput, InsertOneError,
            InsertOneInput, InsertOneOutput, SupportedInsertLink,
        },
        dynamic_collection::{CollectionToSerialize, DynamicCollection, DynamicInsertInput},
        op_insert_one_trait_extension::{JsonInsertOneLink, JsonInsertOneToConsume},
        sqlx_executor::{FromTo, SqlxExecutorData},
    },
    links::{
        DefaultRelationKey,
        relation_many_to_many::ManyToMany,
        relation_one_to_many::OneToMany,
        update_links::{SetId, SetNew},
    },
    operations::{
        Operation,
        insert::{AbortOperation, InsertEntity, InsertLinkConsumeData, InsertOne, InsertOneLink},
    },
};

type DynCollection<S> = Arc<DynamicCollection<S>>;

pub(super) async fn exec_insert_one<S>(
    this: &SqlxExecutorData<S>,
    base: Arc<DynamicCollection<S>>,
    data: DynamicInsertInput<S>,
    links_input: Vec<SupportedInsertLink>,
    conn: &mut S::Connection,
) -> Result<InsertOneOutput, InsertOneError>
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
    let cols = this.collections.read().await;
    let rel_guard = this.link_info.read().await;
    let mut all_gaurds = Vec::new();
    let mut links = Vec::<JsonInsertOneToConsume<S>>::new();

    for link in links_input {
        match link {
            SupportedInsertLink::SetId { to, id } => {
                let to_gaurd = cols
                    .get(to.as_str())
                    .ok_or(InsertOneError::InvalidLink)?
                    .read()
                    .await;
                let to = to_gaurd.clone();
                all_gaurds.push(to_gaurd);

                let forward = FromTo {
                    from: Arc::clone(&base.collection_name.snake_case),
                    to: Arc::clone(&to.collection_name.snake_case),
                };

                if rel_guard.many_to_many.contains(&forward) {
                    links.push(JsonInsertOneToConsume::new(SetId {
                        relation: ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: Arc::clone(&base),
                            to,
                        },
                        id,
                    }))
                } else if rel_guard.one_to_many.contains(&forward) {
                    links.push(JsonInsertOneToConsume::new(SetId {
                        relation: OneToMany {
                            fk_unique_id: DefaultRelationKey,
                            from: Arc::clone(&base),
                            to,
                        },
                        id,
                    }))
                } else {
                    return Err(InsertOneError::InvalidLink);
                }
            }
            SupportedInsertLink::SetNew { to, value } => {
                let to_gaurd = cols
                    .get(to.as_str())
                    .ok_or(InsertOneError::InvalidLink)?
                    .read()
                    .await;
                let to = to_gaurd.clone();
                all_gaurds.push(to_gaurd);
                let link_data: DynamicInsertInput<S> =
                    deserialize(Arc::from(value.0.as_str()), Arc::clone(&to), JsonFormat)
                        .map_err(|_| InsertOneError::InvalidData)?;
                links.push(JsonInsertOneToConsume::new(SetNew {
                    relation: OneToMany {
                        fk_unique_id: DefaultRelationKey,
                        from: Arc::clone(&base),
                        to,
                    },
                    data: link_data,
                }))
            }
        }
    }

    let out = Operation::<S>::exec_operation(
        InsertOne {
            handler: base,
            data: InsertEntity {
                attributes: data,
                link: links,
            },
            infalibility: AbortOperation,
        },
        conn,
    )
    .await;

    drop(all_gaurds);
    drop(rel_guard);
    drop(cols);

    Ok(InsertOneOutput {
        id: out.id,
        attributes: out.attributes,
        links: out.links,
    })
}

pub fn insert_one<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: InsertOneInput,
) -> impl Future<Output = Result<InsertOneOutput, InsertOneError>> + 'static + Send + use<S>
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
        let cols = this.collections.read().await;
        let base_gaurd = cols
            .get(input.base.as_str())
            .ok_or(InsertOneError::CollectionNotFound)?
            .read()
            .await;
        let base = base_gaurd.clone();

        let data: DynamicInsertInput<S> = deserialize(
            Arc::from(input.data.0.as_str()),
            Arc::clone(&base),
            JsonFormat,
        )
        .map_err(|_| InsertOneError::InvalidData)?;

        let mut conn = this.pool.acquire().await.unwrap();

        let out = exec_insert_one(&this, base, data, input.links, &mut conn).await?;

        drop(base_gaurd);
        drop(cols);
        Ok(out)
    }
}
