use std::sync::Arc;

use crate::{
    database_extention::DatabaseExt,
    gen_serde::json_format_side::PartialDeserialize,
    json_client::{
        ToBind,
        client_interface::SupportedFilter,
        compat::filters::{FilterAnd, FilterOr, ScopedIsNull},
        dynamic_collection::DynamicCollection,
    },
    sqlx_query_builder::{
        basic_expressions::{
            ColumnContains, ColumnEqual, ColumnGreaterThan, ColumnGreaterThanOrEqual,
            ColumnLessThan, ColumnLessThanOrEqual, ColumnNotEqual, ScopedColumn,
        },
        trait_objects::BoxedExpression,
        Expression,
    },
    sub_arc::ArcSubStr,
};

type DynCol = ScopedColumn<Arc<str>, Arc<str>>;
type DynBind<S> = Box<dyn ToBind<S> + Send>;

fn field_by_col<'a, S>(
    col: &ArcSubStr,
    base: &'a DynamicCollection<S>,
) -> Result<&'a crate::json_client::dynamic_collection::DynamicField<S>, ()>
where
    S: DatabaseExt,
{
    base.fields
        .iter()
        .find(|f| f.name.as_str() == col.as_str())
        .ok_or(())
}

fn scoped_col<S>(col: &ArcSubStr, base: &DynamicCollection<S>) -> Result<DynCol, ()>
where
    S: DatabaseExt,
{
    let field = field_by_col(col, base)?;
    Ok(ScopedColumn {
        table: Arc::clone(&base.collection_name.pascal_case),
        col: Arc::clone(&field.name.snake_case),
    })
}

fn field_bind<S>(
    col: ArcSubStr,
    partial: PartialDeserialize,
    base: &DynamicCollection<S>,
) -> Result<(DynCol, DynBind<S>), ()>
where
    S: DatabaseExt,
{
    let field = field_by_col(&col, base)?;
    let bind = (field.type_info.to_bind)(partial)?;
    Ok((scoped_col(&col, base)?, bind))
}

fn string_contains_bind<S>(
    col: ArcSubStr,
    partial: PartialDeserialize,
    base: &DynamicCollection<S>,
) -> Result<(DynCol, DynBind<S>), ()>
where
    S: DatabaseExt,
    String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
{
    let field = field_by_col(&col, base)?;
    if (field.type_info.type_name)() != std::any::type_name::<String>() {
        return Err(());
    }
    let needle: String = partial.continue_deserialize().map_err(|_| ())?;
    let pattern = format!("%{}%", needle);
    Ok((scoped_col(&col, base)?, Box::new(pattern)))
}

pub fn parse_one_supported_filter<'q, S>(
    filter: SupportedFilter,
    base: &DynamicCollection<S>,
) -> Result<Box<dyn BoxedExpression<S> + Send>, ()>
where
    S: DatabaseExt,
    ColumnEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnNotEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnGreaterThan<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnGreaterThanOrEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnLessThan<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnLessThanOrEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnContains<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ScopedIsNull: for<'e> Expression<'e, S>,
    String: for<'a> sqlx::Encode<'a, S> + sqlx::Type<S>,
{
    Ok(match filter {
        SupportedFilter::ColEq { col, eq } => {
            let (col, bind) = field_bind(col, eq, base)?;
            Box::new(ColumnEqual { col, eq: bind })
        }
        SupportedFilter::ColNe { col, ne } => {
            let (col, bind) = field_bind(col, ne, base)?;
            Box::new(ColumnNotEqual { col, ne: bind })
        }
        SupportedFilter::ColGt { col, gt } => {
            let (col, bind) = field_bind(col, gt, base)?;
            Box::new(ColumnGreaterThan { col, gt: bind })
        }
        SupportedFilter::ColGte { col, gte } => {
            let (col, bind) = field_bind(col, gte, base)?;
            Box::new(ColumnGreaterThanOrEqual { col, ge: bind })
        }
        SupportedFilter::ColLt { col, lt } => {
            let (col, bind) = field_bind(col, lt, base)?;
            Box::new(ColumnLessThan { col, lt: bind })
        }
        SupportedFilter::ColLte { col, lte } => {
            let (col, bind) = field_bind(col, lte, base)?;
            Box::new(ColumnLessThanOrEqual { col, le: bind })
        }
        SupportedFilter::ColContains { col, value } => {
            let (col, bind) = string_contains_bind(col, value, base)?;
            Box::new(ColumnContains { col, like: bind })
        }
        SupportedFilter::ColIsNull { col } => {
            let field = field_by_col(&col, base)?;
            Box::new(ScopedIsNull {
                table: Arc::clone(&base.collection_name.pascal_case),
                col: Arc::clone(&field.name.snake_case),
                is_not: false,
            })
        }
        SupportedFilter::ColIsNotNull { col } => {
            let field = field_by_col(&col, base)?;
            Box::new(ScopedIsNull {
                table: Arc::clone(&base.collection_name.pascal_case),
                col: Arc::clone(&field.name.snake_case),
                is_not: true,
            })
        }
        SupportedFilter::And { filters } => {
            let parsed = parse_supported_filter(filters, base)?;
            if parsed.is_empty() {
                return Err(());
            }
            if parsed.len() == 1 {
                return Ok(parsed.into_iter().next().unwrap());
            }
            Box::new(FilterAnd(parsed))
        }
        SupportedFilter::Or { filters } => {
            let parsed = parse_supported_filter(filters, base)?;
            if parsed.is_empty() {
                return Err(());
            }
            if parsed.len() == 1 {
                return Ok(parsed.into_iter().next().unwrap());
            }
            Box::new(FilterOr(parsed))
        }
    })
}

pub fn parse_supported_filter<'q, S>(
    input: Vec<SupportedFilter>,
    base: &DynamicCollection<S>,
) -> Result<Vec<Box<dyn BoxedExpression<S> + Send>>, ()>
where
    S: DatabaseExt,
    ColumnEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnNotEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnGreaterThan<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnGreaterThanOrEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnLessThan<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnLessThanOrEqual<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ColumnContains<DynCol, DynBind<S>>: for<'e> Expression<'e, S>,
    ScopedIsNull: for<'e> Expression<'e, S>,
    String: for<'a> sqlx::Encode<'a, S> + sqlx::Type<S>,
{
    input
        .into_iter()
        .map(|filter| parse_one_supported_filter(filter, base))
        .collect()
}
