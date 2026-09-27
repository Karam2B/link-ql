use std::sync::Arc;

use sqlx::ColumnIndex;

use crate::{
    collections::Collection,
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    json_client::{
        DynManyToMany, DynOneToMany,
        client_interface::{DeleteOneError, DeleteOneInput, DeleteOneOutput, SupportedDeleteLink},
        dynamic_collection::{CollectionToSerialize, DynamicCollection},
        op_delete_one_trait_extension::{JsonDeleteOneLink, JsonDeleteOneToConsume},
        sqlx_executor::{FromTo, SqlxExecutorData},
    },
    links::{
        DefaultRelationKey,
        relation_many_to_many::{DeleteManyToManyLinked, ManyToMany},
        relation_one_to_many::OneToMany,
    },
    operations::{
        Operation,
        delete::{Delete, DeleteLink, DeleteLinkSplit},
        operations_expressions_crossover::ExpressionsForOperation,
    },
    sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
};

type DynCollection<S> = Arc<DynamicCollection<S>>;

pub fn delete_one<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: DeleteOneInput,
) -> impl Future<Output = Result<DeleteOneOutput, DeleteOneError>> + 'static + Send + use<S>
where
    i64: sqlx::Type<S> + for<'q> sqlx::Decode<'q, S> + for<'q> sqlx::Encode<'q, S>,
    for<'a> &'a str: sqlx::ColumnIndex<<S as sqlx::Database>::Row>,
    S: sqlx::Database + DatabaseExt + ExecutorTrait + Send + Sync + 'static,
    DynCollection<S>: for<'r> FromRowAlias<'r, S::Row, RData = CollectionToSerialize>,
    DynOneToMany<S>: DeleteLinkSplit<
            InitSplitForPreOp: Send + 'static,
            Link: JsonDeleteOneLink<S>
                      + DeleteLink<InitSplitForWheres: Send + 'static, PreOpSplitTake: Send + 'static>,
        >,
    DeleteManyToManyLinked<DefaultRelationKey, DynCollection<S>, DynCollection<S>>: DeleteLinkSplit<
            InitSplitForPreOp: Send + 'static,
            Link: JsonDeleteOneLink<S>
                      + DeleteLink<InitSplitForWheres: Send + 'static, PreOpSplitTake: Send + 'static>,
        >,
    Vec<JsonDeleteOneToConsume<S>>: DeleteLinkSplit<
            Link = Vec<Box<dyn JsonDeleteOneLink<S> + Send>>,
            InitSplitForPreOp: Send + 'static + IntoIterator<Item = Box<dyn std::any::Any + Send>>,
        >,
{
    async move {
        let cols = this.collections.read().await;
        let base_gaurd = cols
            .get(input.base.as_str())
            .ok_or(DeleteOneError::CollectionNotFound)?
            .read()
            .await;
        let base = base_gaurd.clone();
        let mut all_gaurds = vec![base_gaurd];

        let rel_guard = this.link_info.read().await;
        let mut links = Vec::<JsonDeleteOneToConsume<S>>::new();

        for link in input.links {
            match link {
                SupportedDeleteLink::OneToMany { to } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(DeleteOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);
                    links.push(JsonDeleteOneToConsume::from_split(OneToMany {
                        fk_unique_id: DefaultRelationKey,
                        from: base.clone(),
                        to,
                    }))
                }
                SupportedDeleteLink::ManyToMany { to } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(DeleteOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);

                    let forward = FromTo {
                        from: Arc::clone(&base.collection_name.snake_case),
                        to: Arc::clone(&to.collection_name.snake_case),
                    };

                    if !rel_guard.many_to_many.contains(&forward) {
                        return Err(DeleteOneError::InvalidLink);
                    }

                    links.push(JsonDeleteOneToConsume::from_split(DeleteManyToManyLinked {
                        link: ManyToMany {
                            relation_key: DefaultRelationKey,
                            from: base.clone(),
                            to,
                        },
                        from_id: input.id,
                    }))
                }
            }
        }

        let mut conn = this.pool.acquire().await.unwrap();

        let out = Operation::<S>::exec_operation(
            Delete {
                base: Arc::clone(&base),
                wheres: ColumnEqual {
                    col: base.id().scoped(),
                    eq: Bind(input.id),
                },
                links,
            },
            &mut conn,
        )
        .await;

        drop(all_gaurds);
        drop(rel_guard);
        drop(cols);

        let Some(deleted) = out.into_iter().find(|row| row.id == input.id) else {
            return Err(DeleteOneError::NotFound);
        };

        Ok(DeleteOneOutput {
            id: deleted.id,
            attributes: deleted.attributes,
            links: deleted.links,
        })
    }
}
