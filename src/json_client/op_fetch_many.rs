use sqlx::{ColumnIndex, Decode, Encode, Type};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::{
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    gen_serde::{Serialize, json_serialize_side::JsonAsString},
    json_client::{
        DynOneToMany, DynTimestamp, ToBind,
        client_interface::{
            FetchManyError, FetchManyInput, FetchManyOutput, FirstItem, InsertOneInput,
            InsertOneOutput, OrderBy, Pagination, SupportedInsertLink, SupportedLinkFetchMany,
        },
        dynamic_collection::{CollectionToSerialize, DynamicCollection, VTable},
        compat::dynamic_input::NamedBindList,
        op_fetch_many_trait_extension::JsonLinkFetchMany,
        sqlx_executor::{FromTo, LinkInformations, SqlxExecutorData},
        supported_filters::parse_supported_filter,
    },
    links::{
        DefaultRelationKey, relation_many_to_many::ManyToMany,
        relation_one_to_many::OneToMany,
        relation_one_to_many_inverse::OneToManyInverse, timestamp::Timestamp,
    },
    operations::{
        CollectionOutput, Operation,
        fetch_many::{FetchMany, ManyOutput},
        operations_expressions_crossover::NamedBind,
    },
    sqlx_query_builder::{
        Join,
        trait_objects::BoxedExpression,
    },
    sub_arc::ArcSubStr,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

fn cursor_attributes_from_order_by(
    next: BTreeMap<String, Box<dyn Serialize<JsonAsString> + Send>>,
) -> CollectionToSerialize {
    CollectionToSerialize(
        next.into_iter()
            .map(|(key, value)| (Arc::<str>::from(key.as_str()), value))
            .collect(),
    )
}

pub fn fetch_many<S>(
    this: Arc<SqlxExecutorData<S>>,
    input: FetchManyInput,
) -> impl Future<Output = Result<FetchManyOutput, FetchManyError>> + 'static + Send + use<S>
where
    S: DatabaseExt + ExecutorTrait + Send + Sync,
    Arc<DynamicCollection<S>>: for<'r> FromRowAlias<'r, S::Row, RData = CollectionToSerialize>,
    OneToMany<DefaultRelationKey, Arc<DynamicCollection<S>>, Arc<DynamicCollection<S>>>:
        JsonLinkFetchMany<S>,
    OneToManyInverse<DefaultRelationKey, Arc<DynamicCollection<S>>, Arc<DynamicCollection<S>>>:
        JsonLinkFetchMany<S>,
    ManyToMany<false, DefaultRelationKey, Arc<DynamicCollection<S>>, Arc<DynamicCollection<S>>>:
        JsonLinkFetchMany<S>,
    Timestamp<Arc<DynamicCollection<S>>>: JsonLinkFetchMany<S>,
    i64: for<'q> Decode<'q, S> + for<'q> Encode<'q, S> + Type<S>,
    String: for<'q> Encode<'q, S> + Type<S>,
    for<'s> &'s str: ColumnIndex<S::Row>,
    //connection
{
    async move {
        let cols_gaurd = this.collections.read().await;
        let col_gaurd = cols_gaurd
            .get(input.base.as_str())
            .ok_or(FetchManyError::CollectionNotFound)?
            .read()
            .await;
        let rel_gaurd = this.link_info.read().await;

        let base = col_gaurd.clone();

        let mut all_gaurds = vec![col_gaurd];

        let wheres = parse_supported_filter(input.filters, &base)
            .map_err(|_| FetchManyError::InvalidFilter)?;

        let mut links = Vec::<Box<dyn JsonLinkFetchMany<S> + Send>>::new();

        for each in input.links {
            match each {
                SupportedLinkFetchMany::OneToMany { to } => {
                    let to_collection_l = cols_gaurd
                        .get(to.as_str())
                        .ok_or(FetchManyError::InvalidLink)?
                        .read()
                        .await;

                    let to_collection = to_collection_l.clone();
                    all_gaurds.push(to_collection_l);

                    let forward = FromTo {
                        from: Arc::clone(&base.collection_name.snake_case),
                        to: Arc::clone(&to_collection.collection_name.snake_case),
                    };
                    let reverse = FromTo {
                        from: Arc::clone(&to_collection.collection_name.snake_case),
                        to: Arc::clone(&base.collection_name.snake_case),
                    };

                    if rel_gaurd.one_to_many.contains(&forward) {
                        links.push(Box::new(OneToMany {
                            fk_unique_id: DefaultRelationKey,
                            from: Arc::clone(&base),
                            to: to_collection,
                        }));
                    } else if rel_gaurd.one_to_many.contains(&reverse) {
                        links.push(Box::new(OneToManyInverse {
                            fk_unique_id: DefaultRelationKey,
                            from: Arc::clone(&base),
                            to: to_collection,
                        }));
                    } else {
                        return Err(FetchManyError::InvalidLink);
                    }
                }
                SupportedLinkFetchMany::ManyToMany { to } => {
                    let to_collection_l = cols_gaurd
                        .get(to.as_str())
                        .ok_or(FetchManyError::InvalidLink)?
                        .read()
                        .await;

                    let to_collection = to_collection_l.clone();
                    all_gaurds.push(to_collection_l);

                    let forward = FromTo {
                        from: Arc::clone(&base.collection_name.snake_case),
                        to: Arc::clone(&to_collection.collection_name.snake_case),
                    };

                    if rel_gaurd.many_to_many.contains(&forward) {
                        links.push(Box::new(ManyToMany {
                            relation_key: DefaultRelationKey,
                            from: Arc::clone(&base),
                            to: to_collection,
                        }));
                    } else {
                        return Err(FetchManyError::InvalidLink);
                    }
                }
                SupportedLinkFetchMany::Timestamp => {
                    if !rel_gaurd
                        .timestamped
                        .contains(base.collection_name.snake_case.as_ref())
                    {
                        return Err(FetchManyError::InvalidLink);
                    }
                    links.push(Box::new(Timestamp {
                        collection: Arc::clone(&base),
                    }));
                }
            }
        }

        let limit = input.pagination.limit.clamp(0, 100);

        let order_by = dynamic_order_by_mod::process_order_by(&base, &input.pagination.order_by)
            .ok_or(FetchManyError::InvalidOrderBy)?;

        let first_item = process_first_item(&base, &input.pagination.first_item)
            .map_err(|_| FetchManyError::InvalidFirstItem)?;
        let first_item = first_item.map(|item| (item.id, item.attributes));

        let mut conn = this.pool.acquire().await.unwrap();

        let s = FetchMany {
            base,
            wheres,
            links,
            limit,
            cursor_order_by: order_by,
            cursor_first_item: first_item,
        };

        let out = Operation::<S>::exec_operation(s, &mut conn).await;

        let next_item = out.next_item.map(|next_item| CollectionOutput {
            id: next_item.id,
            attributes: cursor_attributes_from_order_by(next_item.ordered_by_field),
        });

        drop(rel_gaurd);
        drop(all_gaurds);

        return Ok(ManyOutput {
            items: out.items,
            next_item,
        });
    }
}

pub mod dynamic_order_by_mod {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use sqlx::{ColumnIndex, Database, Row};

    use crate::{
        database_extention::DatabaseExt,
        operations::operations_expressions_crossover::ExpressionsForOperation,
        from_row::{FromRowAlias, FromRowData, FromRowError, RowNumAliased, RowStrAliased, from_row_v2::RowAliased},
        gen_serde::{Serialize, json_serialize_side::JsonAsString},
        json_client::{
            client_interface::{Direction, OrderBy},
            dynamic_collection::{DynamicCollection, VTable},
        },
        sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
    };

    pub struct DynamicOrderBy<S>
    where
        S: DatabaseExt,
    {
        table: Arc<str>,
        col: Arc<str>,
        sqlx_ident: VTable<S>,
        is_optional: bool,
        direction: Direction,
    }

    impl<S> Clone for DynamicOrderBy<S>
    where
        S: DatabaseExt,
    {
        fn clone(&self) -> Self {
            Self {
                table: Arc::clone(&self.table),
                col: Arc::clone(&self.col),
                sqlx_ident: self.sqlx_ident.clone(),
                is_optional: self.is_optional,
                direction: match self.direction {
                    Direction::Asc => Direction::Asc,
                    Direction::Desc => Direction::Desc,
                },
            }
        }
    }

    pub struct DynamicOrderByList<S>(pub Vec<DynamicOrderBy<S>>)
    where
        S: DatabaseExt;

    pub struct DynamicOrderByListScoped<S>(pub Vec<DynamicOrderBy<S>>)
    where
        S: DatabaseExt;

    impl<S> Clone for DynamicOrderByListScoped<S>
    where
        S: DatabaseExt,
        DynamicOrderBy<S>: Clone,
    {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<S> Clone for DynamicOrderByList<S>
    where
        S: DatabaseExt,
        DynamicOrderBy<S>: Clone,
    {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<S> ExpressionsForOperation for DynamicOrderByList<S>
    where
        S: DatabaseExt,
    {
        type Identifier = Vec<DynamicOrderBy<S>>;
        fn identifier(&self) -> Self::Identifier {
            self.0.clone()
        }

        type Scoped = DynamicOrderByListScoped<S>;
        fn scoped(&self) -> Self::Scoped {
            DynamicOrderByListScoped(self.0.clone())
        }

        type ScopedAliased = ();
        fn scoped_aliased(&self, _: &'static str) -> Self::ScopedAliased {}

        type NumScopedAliased = ();
        fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {}
    }

    impl<S> ExpressionsForOperation for DynamicOrderBy<S>
    where
        S: DatabaseExt,
    {
        type Identifier = &'static str;
        fn identifier(&self) -> Self::Identifier {
            "order_by"
        }

        type Scoped = DynamicOrderBy<S>;
        fn scoped(&self) -> Self::Scoped {
            self.clone()
        }

        type ScopedAliased = ();
        fn scoped_aliased(&self, _: &'static str) -> Self::ScopedAliased {}

        type NumScopedAliased = ();
        fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {}
    }

    impl<S> OpExpression for DynamicOrderByListScoped<S>
    where
        S: DatabaseExt,
    {
        fn is_expression_present(&self) -> bool {
            !self.0.is_empty()
        }
    }

    impl<'q, S> Expression<'q, S> for DynamicOrderByListScoped<S>
    where
        S: DatabaseExt,
        DynamicOrderBy<S>: Expression<'q, S>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            if self.0.is_empty() {
                return;
            }
            let mut first = true;
            for item in self.0 {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                item.expression(ctx);
            }
        }
    }

    impl<S> FromRowData for DynamicOrderByList<S>
    where
        S: DatabaseExt,
        DynamicOrderBy<S>: FromRowData,
    {
        type RData = BTreeMap<String, Box<dyn Serialize<JsonAsString> + Send>>;
    }

    impl<'r, S> FromRowAlias<'r, S::Row> for DynamicOrderByList<S>
    where
        S: DatabaseExt + Database,
        DynamicOrderBy<S>: FromRowAlias<
            'r,
            S::Row,
            RData = (String, Box<dyn Serialize<JsonAsString> + Send>),
        >,
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            let mut ret = BTreeMap::new();
            for item in &self.0 {
                let (col, value): (String, Box<dyn Serialize<JsonAsString> + Send>) =
                    item.no_alias(row)?;
                ret.insert(col, value);
            }
            Ok(ret)
        }

        fn str_alias(
            &self,
            row: RowStrAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut ret = BTreeMap::new();
            for item in &self.0 {
                let (col, value): (String, Box<dyn Serialize<JsonAsString> + Send>) =
                    item.str_alias(row.clone())?;
                ret.insert(col, value);
            }
            Ok(ret)
        }

        fn num_alias(
            &self,
            row: RowNumAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut ret = BTreeMap::new();
            for item in &self.0 {
                let (col, value): (String, Box<dyn Serialize<JsonAsString> + Send>) =
                    item.num_alias(row.clone())?;
                ret.insert(col, value);
            }
            Ok(ret)
        }
    }

    impl<S> OpExpression for DynamicOrderBy<S> where S: DatabaseExt + Database {
}

    impl<'q, S> Expression<'q, S> for DynamicOrderBy<S>
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.sanitize(self.table.as_ref());
            ctx.syntax(".");
            ctx.sanitize(self.col.as_ref());
            ctx.syntax(" ");
            match self.direction {
                Direction::Asc => ctx.syntax("ASC"),
                Direction::Desc => ctx.syntax("DESC"),
            }
        }
    }

    impl<S> FromRowData for DynamicOrderBy<S>
    where
        S: DatabaseExt,
    {
        type RData = (String, Box<dyn Serialize<JsonAsString> + Send>);
    }

    impl<'r, S> FromRowAlias<'r, S::Row> for DynamicOrderBy<S>
    where
        S: DatabaseExt + Database,
        for<'a> &'a str: ColumnIndex<S::Row>,
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            let value = (self.sqlx_ident.decode_from_row)(self.is_optional, self.col.as_ref(), row)
                .map_err(FromRowError::ColumnNotFound)?;

            Ok((self.col.to_string(), value))
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let col_name = format!("{}{}", row.alias, self.col.as_ref());
            let value =
                (self.sqlx_ident.decode_from_row)(self.is_optional, col_name.as_str(), row.row)
                    .map_err(FromRowError::ColumnNotFound)?;

            Ok((self.col.to_string(), value))
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let col_name = format!(
                "{}{}{}",
                row.str_alias,
                row.num_alias.map(|n| n.to_string()).unwrap_or_default(),
                self.col.as_ref()
            );
            let value =
                (self.sqlx_ident.decode_from_row)(self.is_optional, col_name.as_str(), row.row)
                    .map_err(FromRowError::ColumnNotFound)?;

            Ok((self.col.to_string(), value))
        }
    }

    impl<S> FromRowData for Vec<DynamicOrderBy<S>>
    where
        S: DatabaseExt,
    {
        type RData = BTreeMap<String, Box<dyn Serialize<JsonAsString> + Send>>;
    }

    impl<'r, S> FromRowAlias<'r, S::Row> for Vec<DynamicOrderBy<S>>
    where
        S: DatabaseExt + Database,
        for<'a> &'a str: ColumnIndex<S::Row>,
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            let mut ret = BTreeMap::new();
            for each in self {
                let (col, value) = each.no_alias(row)?;
                ret.insert(col, value);
            }
            Ok(ret)
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut ret = BTreeMap::new();
            for each in self {
                let (col, value) = each.str_alias(row.clone())?;
                ret.insert(col, value);
            }
            Ok(ret)
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut ret = BTreeMap::new();
            for each in self {
                let (col, value) = each.num_alias(row.clone())?;
                ret.insert(col, value);
            }
            Ok(ret)
        }
    }

    pub fn process_order_by<S>(
        base: &DynamicCollection<S>,
        order_by: &[OrderBy],
    ) -> Option<DynamicOrderByList<S>>
    where
        S: DatabaseExt,
    {
        let mut ret = vec![];

        for each in order_by {
            let found = base
                .fields
                .iter()
                .find(|field| field.name.as_str() == each.col.as_str())?;

            ret.push(DynamicOrderBy {
                table: Arc::clone(&base.collection_name.snake_case),
                col: Arc::clone(&found.name.snake_case),
                sqlx_ident: found.type_info.clone(),
                is_optional: found.is_optional,
                direction: match each.direction {
                    Direction::Asc => Direction::Asc,
                    Direction::Desc => Direction::Desc,
                },
            });
        }

        Some(DynamicOrderByList(ret))
    }
}

fn process_first_item<S>(
    base: &DynamicCollection<S>,
    pagination: &Option<FirstItem>,
) -> Result<Option<CollectionOutput<i64, NamedBindList<S>>>, ()>
where
    S: DatabaseExt,
{
    if let Some(first_item) = pagination {
        let mut attributes = vec![];

        for (key, value) in first_item.data.iter() {
            let found = base.fields.iter().find(|f| f.name.as_str() == key.as_str());
            let found = found.ok_or(())?;
            let bind = (found.type_info.to_bind)(value.clone()).map_err(|_| ())?;

            attributes.push(NamedBind {
                table: Arc::clone(&base.collection_name.snake_case),
                name: key.detach(),
                value: bind,
            });
        }

        Ok(Some(CollectionOutput {
            id: first_item.id,
            attributes: NamedBindList(attributes),
        }))
    } else {
        Ok(None)
    }
}

// fetch_many_link_trait lives in `pub mod fetch_many` below (gen_serde, no v1).
