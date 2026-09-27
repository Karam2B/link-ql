use std::collections::BTreeMap;

use crate::gen_serde::json_format_side::{JsonAsArcCursor, PartialDeserialize};
use crate::gen_serde::json_serialize_side::JsonAsString;
use crate::gen_serde::{
    Deserialize, DeserializeMap, DeserializeSeq, DeserializeSpec, Deserializer, KnownKey,
    ObjectEncoding, Serialize, UnknownKey,
};
use crate::json_client::client_interface::{
    AddCollectionInput, AddLinkInput, DeleteOneInput, Direction, DynamicFieldInput, FetchManyInput,
    FetchOneInput, FirstItem, InsertManyInput, InsertManyItem, InsertManyOutput, InsertOneInput,
    InsertOneOutput, OrderBy, Pagination, SupportedDeleteLink, SupportedFilter,
    SupportedInsertLink, SupportedLinkFetchMany, SupportedLinkFetchOne, SupportedType,
    SupportedUpdateLink, UpdateOneInput,
};
use crate::sub_arc::{ArcSubStr, SubArc};

impl Serialize<JsonAsString> for InsertManyOutput
where
    Vec<InsertOneOutput>: Serialize<JsonAsString>,
{
    fn serialize(&self, ctx: &mut JsonAsString) {
        let mut object = ObjectEncoding::serialize_start(ctx);
        ObjectEncoding::serialize_pair(ctx, &mut object, "items", &self.items);
        ObjectEncoding::serialize_end(ctx, object);
    }
}

impl DeserializeSpec for SupportedType {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedType
where
    S: Deserializer<'de>,
    PartialDeserialize: Deserialize<'de, S>,
    S::Err: From<&'static str> + From<String>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let partial = PartialDeserialize::deserialize((), serialized)?;
        deserialize_supported_type_partial(&partial).map_err(S::Err::from)
    }
}

fn deserialize_supported_type_partial(
    partial: &PartialDeserialize,
) -> Result<SupportedType, String> {
    let trimmed = partial.0.as_str().trim();
    if !trimmed.starts_with('{') {
        let name: String = partial.continue_deserialize()?;
        return match name.as_str() {
            "String" => Ok(SupportedType::String),
            "Boolean" => Ok(SupportedType::Boolean),
            "Int" => Ok(SupportedType::Int),
            "Float64" => Ok(SupportedType::Float64),
            _other => Err("unsupported SupportedType".into()),
        };
    }

    let mut cursor = JsonAsArcCursor {
        inner: std::sync::Arc::from(partial.0.as_str()),
        start: 0,
    };
    let mut map = DeserializeMap::start_map(&mut cursor)?;
    let ty: String = DeserializeMap::deserialize_with_known_key(&mut cursor, &mut map, "ty", ())?;
    if ty != "Array" {
        return Err("unsupported SupportedType object ty".into());
    }
    let of: SupportedType =
        DeserializeMap::deserialize_with_known_key(&mut cursor, &mut map, "of", ())?;
    DeserializeMap::finish(&mut cursor, map)?;
    Ok(SupportedType::Array(Box::new(of)))
}

impl DeserializeSpec for DynamicFieldInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for DynamicFieldInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    SupportedType: Deserialize<'de, S>,
    bool: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let name = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "name", ())?;
        let type_info =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "type_info", ())?;
        let is_optional =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "is_optional", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(DynamicFieldInput {
            name,
            type_info,
            is_optional,
        })
    }
}

impl DeserializeSpec for AddCollectionInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for AddCollectionInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    Vec<DynamicFieldInput>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let name = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "name", ())?;
        let fields =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "fields", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(AddCollectionInput { name, fields })
    }
}

impl DeserializeSpec for InsertOneInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for InsertOneInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
    Vec<SupportedInsertLink>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let data = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "data", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(InsertOneInput { base, data, links })
    }
}

impl DeserializeSpec for InsertManyItem {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for InsertManyItem
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    PartialDeserialize: Deserialize<'de, S>,
    Vec<SupportedInsertLink>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let data = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "data", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(InsertManyItem { data, links })
    }
}

impl DeserializeSpec for InsertManyInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for InsertManyInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    Vec<InsertManyItem>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let items = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "items", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(InsertManyInput { base, items })
    }
}

impl DeserializeSpec for SupportedInsertLink {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedInsertLink
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
    i64: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "set_id" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                let id =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
                SupportedInsertLink::SetId { to, id }
            }
            "set_new" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                let value =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "value", ())?;
                SupportedInsertLink::SetNew { to, value }
            }
            _ => return Err(S::Err::from("unsupported insert link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for SupportedUpdateLink {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedUpdateLink
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
    i64: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "set_id" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                let id =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
                SupportedUpdateLink::SetId { to, id }
            }
            "set_new" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                let value =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "value", ())?;
                SupportedUpdateLink::SetNew { to, value }
            }
            "set_null" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedUpdateLink::SetNull { to }
            }
            "remove_id" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                let id =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
                SupportedUpdateLink::RemoveId { to, id }
            }
            _ => return Err(S::Err::from("unsupported update link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for SupportedLinkFetchMany {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedFilter
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "col_eq" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let eq =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "eq", ())?;
                SupportedFilter::ColEq { col, eq }
            }
            "col_ne" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let ne =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ne", ())?;
                SupportedFilter::ColNe { col, ne }
            }
            "col_gt" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let gt =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "gt", ())?;
                SupportedFilter::ColGt { col, gt }
            }
            "col_gte" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let gte =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "gte", ())?;
                SupportedFilter::ColGte { col, gte }
            }
            "col_lt" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let lt =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "lt", ())?;
                SupportedFilter::ColLt { col, lt }
            }
            "col_lte" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let lte =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "lte", ())?;
                SupportedFilter::ColLte { col, lte }
            }
            "col_like" | "col_contains" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                let value =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "value", ())?;
                SupportedFilter::ColContains { col, value }
            }
            "col_is_null" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                SupportedFilter::ColIsNull { col }
            }
            "col_is_not_null" => {
                let col =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
                SupportedFilter::ColIsNotNull { col }
            }
            "and" => {
                let filters = DeserializeMap::deserialize_with_known_key(
                    serialized,
                    &mut map,
                    "filters",
                    (),
                )?;
                SupportedFilter::And { filters }
            }
            "or" => {
                let filters = DeserializeMap::deserialize_with_known_key(
                    serialized,
                    &mut map,
                    "filters",
                    (),
                )?;
                SupportedFilter::Or { filters }
            }
            "group" => {
                let filters = DeserializeMap::deserialize_with_known_key(
                    serialized,
                    &mut map,
                    "filters",
                    (),
                )?;
                SupportedFilter::And { filters }
            }
            _ => return Err(S::Err::from("unsupported filter ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for SupportedFilter {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedLinkFetchMany
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "one_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedLinkFetchMany::OneToMany { to }
            }
            "many_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedLinkFetchMany::ManyToMany { to }
            }
            "timestamp" => SupportedLinkFetchMany::Timestamp,
            _ => return Err(S::Err::from("unsupported fetch link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for AddLinkInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for AddLinkInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "one_to_many" => {
                let from =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "from", ())?;
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                AddLinkInput::OneToMany { from, to }
            }
            "many_to_many" => {
                let from =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "from", ())?;
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                AddLinkInput::ManyToMany { from, to }
            }
            "timestamp" => {
                let collection = DeserializeMap::deserialize_with_known_key(
                    serialized,
                    &mut map,
                    "collection",
                    (),
                )?;
                AddLinkInput::Timestamp { collection }
            }
            _ => return Err(S::Err::from("unsupported add link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for SupportedLinkFetchOne {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedLinkFetchOne
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "one_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedLinkFetchOne::OneToMany { to }
            }
            "many_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedLinkFetchOne::ManyToMany { to }
            }
            "timestamp" => SupportedLinkFetchOne::Timestamp,
            _ => return Err(S::Err::from("unsupported fetch one link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl DeserializeSpec for FetchOneInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for FetchOneInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    i64: Deserialize<'de, S>,
    Vec<SupportedFilter>: Deserialize<'de, S>,
    Vec<SupportedLinkFetchOne>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let id = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
        let filters =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "filters", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(FetchOneInput {
            base,
            id,
            filters,
            links,
        })
    }
}

impl DeserializeSpec for FetchManyInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for FetchManyInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    Vec<SupportedLinkFetchMany>: Deserialize<'de, S>,
    Vec<SupportedFilter>: Deserialize<'de, S>,
    Pagination: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let filters =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "filters", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        let pagination =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "pagination", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(FetchManyInput {
            base,
            filters,
            links,
            pagination,
        })
    }
}

impl DeserializeSpec for Pagination {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for Pagination
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    i64: Deserialize<'de, S>,
    Option<FirstItem>: Deserialize<'de, S>,
    Vec<OrderBy>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let limit = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "limit", ())?;
        let first_item =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "first_item", ())?;
        let order_by =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "order_by", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(Pagination {
            limit,
            first_item,
            order_by,
        })
    }
}

impl DeserializeSpec for FirstItem {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for FirstItem
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    i64: Deserialize<'de, S>,
    BTreeMap<ArcSubStr, PartialDeserialize>: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let id = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
        let data = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "data", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(FirstItem { id, data })
    }
}

impl DeserializeSpec for BTreeMap<ArcSubStr, PartialDeserialize> {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for BTreeMap<ArcSubStr, PartialDeserialize>
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: UnknownKey<S> + Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map_access = DeserializeMap::start_map(serialized)?;
        let mut ret = BTreeMap::new();
        while DeserializeMap::map_has_next(serialized, &map_access) {
            let (key, value) =
                DeserializeMap::deserialize_with_unknown_key(serialized, &mut map_access, (), ())?;
            ret.insert(key, value);
        }
        DeserializeMap::finish(serialized, map_access)?;
        Ok(ret)
    }
}

impl DeserializeSpec for OrderBy {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for OrderBy
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    Direction: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let col = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "col", ())?;
        let direction =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "direction", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(OrderBy { col, direction })
    }
}

impl DeserializeSpec for Direction {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for Direction
where
    S: Deserializer<'de>,
    String: Deserialize<'de, S>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        match String::deserialize((), serialized)?.as_str() {
            "asc" => Ok(Direction::Asc),
            "desc" => Ok(Direction::Desc),
            _ => Err(S::Err::from("unsupported order direction")),
        }
    }
}

impl DeserializeSpec for UpdateOneInput {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for UpdateOneInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    PartialDeserialize: Deserialize<'de, S>,
    Vec<SupportedUpdateLink>: Deserialize<'de, S>,
    i64: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let id = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
        let data = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "data", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(UpdateOneInput {
            base,
            id,
            data,
            links,
        })
    }
}

impl DeserializeSpec for DeleteOneInput {
    type Handler = ();
}

impl DeserializeSpec for SupportedDeleteLink {
    type Handler = ();
}

impl<'de, S> Deserialize<'de, S> for SupportedDeleteLink
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    ArcSubStr: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
    S::Err: From<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let ty: ArcSubStr =
            DeserializeMap::deserialize_with_known_key(serialized, &mut map, "ty", ())?;
        let out = match ty.as_str() {
            "one_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedDeleteLink::OneToMany { to }
            }
            "many_to_many" => {
                let to =
                    DeserializeMap::deserialize_with_known_key(serialized, &mut map, "to", ())?;
                SupportedDeleteLink::ManyToMany { to }
            }
            _ => return Err(S::Err::from("unsupported delete link ty")),
        };
        DeserializeMap::finish(serialized, map)?;
        Ok(out)
    }
}

impl<'de, S> Deserialize<'de, S> for DeleteOneInput
where
    S: Deserializer<'de>,
    S: DeserializeMap<'de>,
    S: DeserializeSeq<'de>,
    ArcSubStr: Deserialize<'de, S>,
    Vec<SupportedDeleteLink>: Deserialize<'de, S>,
    i64: Deserialize<'de, S>,
    S: KnownKey<&'static str>,
{
    fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
        let mut map = DeserializeMap::start_map(serialized)?;
        let base = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "base", ())?;
        let id = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "id", ())?;
        let links = DeserializeMap::deserialize_with_known_key(serialized, &mut map, "links", ())?;
        DeserializeMap::finish(serialized, map)?;
        Ok(DeleteOneInput { base, id, links })
    }
}
