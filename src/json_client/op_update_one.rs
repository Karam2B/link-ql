use std::sync::Arc;

use sqlx::ColumnIndex;

use crate::{
    collections::Collection,
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    gen_serde::{
        Deserialize, deserialize,
        json_format_side::{JsonAsArcCursor, JsonFormat},
    },
    json_client::{
        DynManyToMany, DynOneToMany,
        client_interface::{SupportedUpdateLink, UpdateOneError, UpdateOneInput, UpdateOneOutput},
        dynamic_collection::{
            CollectionToSerialize, DynamicCollection, DynamicInsertInput, DynamicUpdateInput,
        },
        op_update_one_trait_extension::{JsonUpdateOneLink, JsonUpdateOneToConsume},
        sqlx_executor::{FromTo, LinkInformations, SqlxExecutorData},
    },
    links::{
        DefaultRelationKey,
        relation_many_to_many::{ManyToMany, RemoveJunctionId, SetJunctionId},
        relation_one_to_many::OneToMany,
        update_links::{SetId, SetNew},
    },
    operations::{
        Operation,
        insert::AbortOperation,
        operations_expressions_crossover::ExpressionsForOperation,
        update::{Update, UpdateLink, UpdateLinkData, UpdateLinkSplit},
    },
    sqlx_query_builder::basic_expressions::{Bind, ColumnEqual},
};

type DynCollection<S> = Arc<DynamicCollection<S>>;

/// Links that add columns to the UPDATE SET clause (e.g. one_to_many `set_null` clears an FK).
fn update_link_contributes_set_clause(
    link: &SupportedUpdateLink,
    rel: &LinkInformations,
    base: &Arc<str>,
) -> bool {
    match link {
        SupportedUpdateLink::SetNull { to } | SupportedUpdateLink::SetNew { to, .. } => {
            rel.one_to_many.contains(&FromTo {
                from: Arc::clone(base),
                to: to.detach(),
            })
        }
        SupportedUpdateLink::SetId { to, .. } => {
            let forward = FromTo {
                from: Arc::clone(base),
                to: to.detach(),
            };
            rel.one_to_many.contains(&forward) || rel.many_to_many.contains(&forward)
        }
        SupportedUpdateLink::RemoveId { to, .. } => rel.many_to_many.contains(&FromTo {
            from: Arc::clone(base),
            to: to.detach(),
        }),
    }
}

pub fn update_one<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: UpdateOneInput,
) -> impl Future<Output = Result<UpdateOneOutput, UpdateOneError>> + 'static + Send + use<S>
where
    i64: sqlx::Type<S> + for<'q> sqlx::Decode<'q, S> + for<'q> sqlx::Encode<'q, S>,
    for<'a> &'a str: sqlx::ColumnIndex<<S as sqlx::Database>::Row>,
    S: sqlx::Database + DatabaseExt + ExecutorTrait + Send + Sync + 'static,
    DynCollection<S>: for<'r> FromRowAlias<'r, S::Row, RData = CollectionToSerialize>,
    DynamicUpdateInput<S>: for<'d> Deserialize<'d, JsonAsArcCursor, Handler = DynCollection<S>>,
    DynamicInsertInput<S>: for<'d> Deserialize<'d, JsonAsArcCursor, Handler = DynCollection<S>>,
    SetId<DynOneToMany<S>, Option<i64>>: UpdateLinkSplit<
        Link: JsonUpdateOneLink<S>
                  + UpdateLink<
            InitSplitForPreOp: Send + 'static,
            InitSplitForWheres: Send + 'static,
            InitSplitForUpdateValues: Send + 'static,
            InitSplitPostOp: Send + 'static,
        >,
    >,
    SetJunctionId<DefaultRelationKey, DynCollection<S>, DynCollection<S>>: UpdateLinkSplit<
        Link: JsonUpdateOneLink<S>
                  + UpdateLink<
            InitSplitForPreOp: Send + 'static,
            InitSplitForWheres: Send + 'static,
            InitSplitForUpdateValues: Send + 'static,
            InitSplitPostOp: Send + 'static,
        >,
    >,
    RemoveJunctionId<DefaultRelationKey, DynCollection<S>, DynCollection<S>>: UpdateLinkSplit<
        Link: JsonUpdateOneLink<S>
                  + UpdateLink<
            InitSplitForPreOp: Send + 'static,
            InitSplitForWheres: Send + 'static,
            InitSplitForUpdateValues: Send + 'static,
            InitSplitPostOp: Send + 'static,
        >,
    >,
    SetNew<DynOneToMany<S>, DynamicInsertInput<S>>: UpdateLinkSplit<
        Link: JsonUpdateOneLink<S>
                  + UpdateLink<
            InitSplitForPreOp: Send + 'static,
            InitSplitForWheres: Send + 'static,
            InitSplitForUpdateValues: Send + 'static,
            InitSplitPostOp: Send + 'static,
        >,
    >,
    Vec<JsonUpdateOneToConsume<S>>:
        UpdateLinkSplit<Link = Vec<Box<dyn JsonUpdateOneLink<S> + Send>>>,
{
    async move {
        let cols = this.collections.read().await;
        let base_gaurd = cols
            .get(input.base.as_str())
            .ok_or(UpdateOneError::CollectionNotFound)?
            .read()
            .await;
        let base = base_gaurd.clone();
        let mut all_gaurds = vec![base_gaurd];

        let data: DynamicUpdateInput<S> = deserialize(
            Arc::from(input.data.0.as_str()),
            Arc::clone(&base),
            JsonFormat,
        )
        .map_err(|_| UpdateOneError::InvalidData)?;

        let rel_guard = this.link_info.read().await;

        if data.0.is_empty()
            && !input.links.iter().any(|link| {
                update_link_contributes_set_clause(
                    link,
                    &rel_guard,
                    &base.collection_name.snake_case,
                )
            })
        {
            return Err(UpdateOneError::InvalidData);
        }

        let mut links = Vec::<JsonUpdateOneToConsume<S>>::new();

        for link in input.links {
            match link {
                SupportedUpdateLink::SetId { to, id } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(UpdateOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);

                    let forward = FromTo {
                        from: Arc::clone(&base.collection_name.snake_case),
                        to: Arc::clone(&to.collection_name.snake_case),
                    };

                    if rel_guard.many_to_many.contains(&forward) {
                        let (link, data) = SetJunctionId {
                            relation: ManyToMany::<false, _, _, _> {
                                relation_key: DefaultRelationKey,
                                from: base.clone(),
                                to,
                            },
                            from_id: input.id,
                            to_id: id,
                        }
                        .init_split();
                        links.push(JsonUpdateOneToConsume {
                            link: Box::new(link),
                            data: UpdateLinkData {
                                wheres: Box::new(data.wheres),
                                update_values: Box::new(data.update_values),
                                pre_op: Box::new(data.pre_op),
                                post_op: Box::new(data.post_op),
                            },
                        });
                    } else if rel_guard.one_to_many.contains(&forward) {
                        let (link, data) = SetId {
                            relation: OneToMany {
                                fk_unique_id: DefaultRelationKey,
                                from: base.clone(),
                                to,
                            },
                            id: Some(id),
                        }
                        .init_split();
                        links.push(JsonUpdateOneToConsume {
                            link: Box::new(link),
                            data: UpdateLinkData {
                                wheres: Box::new(data.wheres),
                                update_values: Box::new(data.update_values),
                                pre_op: Box::new(data.pre_op),
                                post_op: Box::new(data.post_op),
                            },
                        });
                    } else {
                        return Err(UpdateOneError::InvalidLink);
                    }
                }
                SupportedUpdateLink::SetNew { to, value } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(UpdateOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);
                    let link_data: DynamicInsertInput<S> =
                        deserialize(Arc::from(value.0.as_str()), Arc::clone(&to), JsonFormat)
                            .map_err(|_| UpdateOneError::InvalidData)?;
                    let (link, data) = SetNew {
                        relation: OneToMany {
                            fk_unique_id: DefaultRelationKey,
                            from: base.clone(),
                            to,
                        },
                        data: link_data,
                    }
                    .init_split();
                    links.push(JsonUpdateOneToConsume {
                        link: Box::new(link),
                        data: UpdateLinkData {
                            wheres: Box::new(data.wheres),
                            update_values: Box::new(data.update_values),
                            pre_op: Box::new(data.pre_op),
                            post_op: Box::new(data.post_op),
                        },
                    })
                }
                SupportedUpdateLink::SetNull { to } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(UpdateOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);
                    let (link, data) = SetId {
                        relation: OneToMany {
                            fk_unique_id: DefaultRelationKey,
                            from: base.clone(),
                            to,
                        },
                        id: None,
                    }
                    .init_split();
                    links.push(JsonUpdateOneToConsume {
                        link: Box::new(link),
                        data: UpdateLinkData {
                            wheres: Box::new(data.wheres),
                            update_values: Box::new(data.update_values),
                            pre_op: Box::new(data.pre_op),
                            post_op: Box::new(data.post_op),
                        },
                    })
                }
                SupportedUpdateLink::RemoveId { to, id } => {
                    let to_gaurd = cols
                        .get(to.as_str())
                        .ok_or(UpdateOneError::InvalidLink)?
                        .read()
                        .await;
                    let to = to_gaurd.clone();
                    all_gaurds.push(to_gaurd);

                    let forward = FromTo {
                        from: Arc::clone(&base.collection_name.snake_case),
                        to: Arc::clone(&to.collection_name.snake_case),
                    };

                    if !rel_guard.many_to_many.contains(&forward) {
                        return Err(UpdateOneError::InvalidLink);
                    }

                    let (link, data) = RemoveJunctionId {
                        relation: ManyToMany::<false, _, _, _> {
                            relation_key: DefaultRelationKey,
                            from: base.clone(),
                            to,
                        },
                        from_id: input.id,
                        to_id: id,
                    }
                    .init_split();
                    links.push(JsonUpdateOneToConsume {
                        link: Box::new(link),
                        data: UpdateLinkData {
                            wheres: Box::new(data.wheres),
                            update_values: Box::new(data.update_values),
                            pre_op: Box::new(data.pre_op),
                            post_op: Box::new(data.post_op),
                        },
                    })
                }
            }
        }

        let mut conn = this.pool.acquire().await.unwrap();

        let out = Operation::<S>::exec_operation(
            Update {
                base: Arc::clone(&base),
                partial: data,
                wheres: ColumnEqual {
                    col: base.id().scoped(),
                    eq: Bind(input.id),
                },
                links,
                infalibility: AbortOperation,
            },
            &mut conn,
        )
        .await;

        drop(all_gaurds);
        drop(rel_guard);
        drop(cols);

        let Some(updated) = out.into_iter().find(|row| row.id == input.id) else {
            return Err(UpdateOneError::NotFound);
        };

        Ok(UpdateOneOutput {
            id: updated.id,
            attributes: updated.attributes,
            links: updated.links,
        })
    }
}
