use crate::{
    database_extention::DatabaseExt,
    gen_serde::{
        Deserialize, Serialize,
        json_format_side::{JsonAsArcCursor, PartialDeserialize},
        json_serialize_side::JsonAsString,
    },
    json_client::{
        ToBind,
        client_interface::{AddCollectionInput, SupportedType},
    },
    sqlx_query_builder::{
        RefExpression, Sanitize, basic_expressions::TypeAsSyntax, trait_objects::BoxedExpression,
    },
    sub_arc::ArcSubStr,
};
use convert_case::{Case, Casing};
use core::fmt;
use sqlx::{ColumnIndex, Decode, Encode, Row, Type};
use std::ops::Not;
use std::sync::Arc;
use std::{any::TypeId, marker::PhantomData};

/// Registry metadata: owned `Arc<str>` so request JSON (`ArcSubStr`) can be dropped after add.
#[derive(Debug)]
pub struct DynamicCollection<S>
where
    S: sqlx::Database + DatabaseExt,
{
    pub collection_name: CollectionName,
    pub(crate) fields: Vec<DynamicField<S>>,
}

#[derive(Debug)]
pub(crate) struct DynamicField<S>
where
    S: sqlx::Database + DatabaseExt,
{
    pub name: FieldName,
    pub type_info: VTable<S>,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct CollectionName {
    pub pascal_case: Arc<str>,
    pub snake_case: Arc<str>,
}

impl CollectionName {
    pub fn new<T: AsRef<str>>(value: &T) -> Result<Self, ()> {
        let name = value.as_ref();
        if name.is_empty()
            || name.is_case(Case::Snake).not()
            || name.starts_with("ct_")
            || name.starts_with("meta_")
        {
            return Err(());
        }

        Ok(Self {
            pascal_case: Arc::from(name.to_case(Case::Pascal)),
            snake_case: Arc::from(name),
        })
    }
}

impl PartialEq for CollectionName {
    fn eq(&self, other: &Self) -> bool {
        self.snake_case == other.snake_case
    }
}

impl Eq for CollectionName {}

#[derive(Debug, Clone)]
pub struct FieldName {
    pub snake_case: Arc<str>,
}

impl FieldName {
    pub fn new<T: AsRef<str>>(value: &T) -> Result<Self, ()> {
        let name = value.as_ref();
        if name.is_empty()
            || name.is_case(Case::Snake).not()
            || name.starts_with("id")
            || name.starts_with("fk_")
        {
            return Err(());
        }

        Ok(Self {
            snake_case: Arc::from(name),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.snake_case
    }
}

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        &self.snake_case
    }
}

impl PartialEq for FieldName {
    fn eq(&self, other: &Self) -> bool {
        self.snake_case == other.snake_case
    }
}

impl Eq for FieldName {}

type DynamicRowJsonValue = Result<Box<dyn Serialize<JsonAsString> + Send>, String>;

macro_rules! vtable {
        (impl<T,S> $([$($wh:tt)*] ,)* {
            $(
                fn $fn_name:ident($($args:ident: $arg_type:ty),*) -> $return_type:ty {$body:tt}
            )*
        }) => {
            pub struct VTable<S>
            where S: DatabaseExt {
                $(
                    pub $fn_name: fn ($($args: $arg_type),*) -> $return_type,
                )*
                _s: PhantomData<S>,
            }

            impl<S> VTable<S>
                where S: DatabaseExt
            {
                pub fn new_as<T>() -> Self
                    where $( $($wh)*,)*
                {
                    Self {
                        $(
                            $fn_name: |$($args: $arg_type),*| -> $return_type {
                                $body
                            }
                        ,)*
                        _s: PhantomData,
                    }
                }
            }

            impl<S> Clone for VTable<S>
            where
                S: DatabaseExt,
            {
                fn clone(&self) -> Self {
                    Self {
                        $(
                            $fn_name: self.$fn_name,
                        )*
                        _s: PhantomData,
                    }
                }
            }
        };
    }

vtable! {
    impl<T,S>
        [ T: Send + Sync + 'static ],
        [ for<'a> &'a str: ColumnIndex<S::Row> ],
        [ S: DatabaseExt + sqlx::Database ],
        [ T: for<'q> Decode<'q, S> + for<'q> Encode<'q, S> + Type<S> + Clone ],
        [ T: crate::is_null::IsNull ],
        [ T: Serialize<JsonAsString> ],
        [
            T: for<'de> Deserialize<'de, JsonAsArcCursor, Handler = ()>
        ],
    {
        fn type_name() -> &'static str {{
            std::any::type_name::<T>()
        }}
        fn type_id() -> TypeId {{
            TypeId::of::<T>()
        }}
        fn to_bind(partial: PartialDeserialize) -> Result<Box<dyn ToBind<S> + Send>, ()> {{
            partial
                .continue_deserialize::<T>()
                .map(|value| Box::new(value) as Box<dyn ToBind<S> + Send>)
                .map_err(|_| ())
        }}
        fn type_expression() -> Box<dyn BoxedExpression<S> + Send> {{
            Box::new(TypeAsSyntax::<T>(PhantomData))
        }}
        fn decode_from_row(is_optional: bool, name: &str, row: &S::Row) -> DynamicRowJsonValue {{
            let value: Option<T> = row
                .try_get(name)
                .map_err(|e| format!("decode {:?}: {e}", name))?;
            if !is_optional && value.is_none() {
                return Err(format!("column {:?} is null", name));
            }
            Ok(Box::new(value) as Box<dyn Serialize<JsonAsString> + Send>)
        }}
        fn partial_to_row_value(is_optional: bool, partial: PartialDeserialize) -> DynamicRowJsonValue {{
            if is_optional && partial.0.as_str().trim() == "null" {
                return Ok(Box::new(None::<T>) as Box<dyn Serialize<JsonAsString> + Send>);
            }
            partial
                .continue_deserialize::<T>()
                .map(|value| Box::new(Some(value)) as Box<dyn Serialize<JsonAsString> + Send>)
                .map_err(|_| format!("invalid value for dynamic row field"))
        }}
    }
}

impl<S> fmt::Debug for VTable<S>
where
    S: DatabaseExt,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VTable({})", (self.type_name)())
    }
}

impl<S> PartialEq for VTable<S>
where
    S: DatabaseExt,
{
    fn eq(&self, other: &Self) -> bool {
        (self.type_name)() == (other.type_name)()
    }
}

impl<S> Eq for VTable<S> where S: DatabaseExt {}

pub fn vtable_for_suppored_type<S>(ty: &SupportedType) -> Result<VTable<S>, ()>
where
    S: sqlx::Database + DatabaseExt,
    String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    bool: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
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
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    use sqlx::types::Json;

    Ok(match ty {
        SupportedType::String => VTable::new_as::<String>(),
        SupportedType::Boolean => VTable::new_as::<bool>(),
        SupportedType::Int => VTable::new_as::<i64>(),
        SupportedType::Float64 => VTable::new_as::<f64>(),
        SupportedType::Array(of) => match &**of {
            SupportedType::String => VTable::new_as::<Json<Vec<String>>>(),
            SupportedType::Boolean => VTable::new_as::<Json<Vec<bool>>>(),
            SupportedType::Int => VTable::new_as::<Json<Vec<i64>>>(),
            SupportedType::Float64 => VTable::new_as::<Json<Vec<f64>>>(),
            SupportedType::Array(_) => return Err(()),
        },
    })
}

impl<S> TryFrom<AddCollectionInput> for DynamicCollection<S>
where
    S: sqlx::Database + DatabaseExt,
    String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
    bool: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S> + for<'d> sqlx::Decode<'d, S>,
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
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    type Error = ();

    fn try_from(input: AddCollectionInput) -> Result<Self, Self::Error> {
        let collection_name = CollectionName::new(&input.name)?;
        let fields = input
            .fields
            .into_iter()
            .map(|f| {
                Ok(DynamicField {
                    name: FieldName::new(&f.name)?,
                    type_info: vtable_for_suppored_type(&f.type_info)?,
                    is_optional: f.is_optional,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            collection_name,
            fields,
        })
    }
}

mod dynamic_insert_binds {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        database_extention::DatabaseExt,
        gen_serde::{
            Deserialize, DeserializeMap, DeserializeMapOrSeq, DeserializeSpec, Deserializer,
            KnownKey, MapOrSeq, UnknownKey, json_format_side::PartialDeserialize,
        },
        json_client::{
            ToBind,
            dynamic_collection::{DynamicCollection, DynamicField},
        },
        sub_arc::ArcSubStr,
    };

    use super::DynamicInsertInput;

    impl<S> DeserializeSpec for DynamicInsertInput<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type Handler = Arc<DynamicCollection<S>>;
    }

    impl<'de, D, S> Deserialize<'de, D> for DynamicInsertInput<S>
    where
        S: sqlx::Database + DatabaseExt,
        String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
        bool: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
        D: Deserializer<'de>,
        D: DeserializeMapOrSeq<'de>,
        D: DeserializeMap<'de>,
        ArcSubStr: Deserialize<'de, D> + UnknownKey<D>,
        PartialDeserialize: Deserialize<'de, D>,
        <D as Deserializer<'de>>::Err: From<String>,
    {
        fn deserialize(
            collection: Self::Handler,
            serialized: &mut D,
        ) -> Result<Self, <D as Deserializer<'de>>::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map_access) => {
                    let mut out: DynamicInsertInput<S> = HashMap::new();

                    while serialized.map_has_next(&map_access) {
                        let (key, partial): (ArcSubStr, PartialDeserialize) =
                            DeserializeMap::deserialize_with_unknown_key(
                                serialized,
                                &mut map_access,
                                (),
                                (),
                            )?;

                        let field = collection
                            .fields
                            .iter()
                            .find(|field| field.name.as_ref() == key.as_str())
                            .ok_or_else(|| format!("unknown field {:?}", key.as_str()).into())?;

                        let bind = if partial.0.as_str().trim() == "null" {
                            if !field.is_optional {
                                return Err(
                                    format!("field {:?} cannot be null", key.as_str()).into()
                                );
                            }
                            Box::new(()) as Box<dyn ToBind<S> + Send>
                        } else {
                            (field.type_info.to_bind)(partial).map_err(|_| {
                                format!("invalid value for {:?}", key.as_str()).into()
                            })?
                        };

                        let key_str = key.as_str().to_string();
                        if out.insert(key, bind).is_some() {
                            return Err(format!("duplicate field {:?}", key_str).into());
                        }
                    }

                    DeserializeMapOrSeq::finish_map(serialized, map_access)?;

                    for field in &collection.fields {
                        if !field.is_optional
                            && !out.keys().any(|key| key.as_str() == field.name.as_ref())
                        {
                            return Err(format!(
                                "missing required field {:?}",
                                field.name.as_ref()
                            )
                            .into());
                        }
                    }

                    Ok(out)
                }
                MapOrSeq::Seq(_) => Err("expected JSON object for insert data".to_string().into()),
            }
        }
    }

    pub struct DynamicUpdateInput<S>(pub HashMap<ArcSubStr, Box<dyn ToBind<S> + Send>>);

    impl<S> DeserializeSpec for DynamicUpdateInput<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type Handler = Arc<DynamicCollection<S>>;
    }

    impl<'de, D, S> Deserialize<'de, D> for DynamicUpdateInput<S>
    where
        S: sqlx::Database + DatabaseExt,
        String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
        bool: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
        D: Deserializer<'de>,
        D: DeserializeMapOrSeq<'de>,
        D: DeserializeMap<'de>,
        ArcSubStr: Deserialize<'de, D> + UnknownKey<D>,
        PartialDeserialize: Deserialize<'de, D>,
        <D as Deserializer<'de>>::Err: From<String>,
    {
        fn deserialize(
            collection: Self::Handler,
            serialized: &mut D,
        ) -> Result<Self, <D as Deserializer<'de>>::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map_access) => {
                    let mut out: DynamicUpdateInput<S> = DynamicUpdateInput(HashMap::new());

                    while serialized.map_has_next(&map_access) {
                        let (key, partial): (ArcSubStr, PartialDeserialize) =
                            DeserializeMap::deserialize_with_unknown_key(
                                serialized,
                                &mut map_access,
                                (),
                                (),
                            )?;

                        let field = collection
                            .fields
                            .iter()
                            .find(|field| field.name.as_ref() == key.as_str())
                            .ok_or_else(|| format!("unknown field {:?}", key.as_str()).into())?;

                        let bind = if partial.0.as_str().trim() == "null" {
                            if !field.is_optional {
                                return Err(
                                    format!("field {:?} cannot be null", key.as_str()).into()
                                );
                            }
                            Box::new(()) as Box<dyn ToBind<S> + Send>
                        } else {
                            (field.type_info.to_bind)(partial).map_err(|_| {
                                format!("invalid value for {:?}", key.as_str()).into()
                            })?
                        };

                        let key_str = key.as_str().to_string();
                        if out.0.insert(key, bind).is_some() {
                            return Err(format!("duplicate field {:?}", key_str).into());
                        }
                    }

                    DeserializeMapOrSeq::finish_map(serialized, map_access)?;

                    Ok(out)
                }
                MapOrSeq::Seq(_) => Err("expected JSON object for update data".to_string().into()),
            }
        }
    }
}

pub(crate) mod collection_impls {
    use std::{
        collections::HashMap,
        fmt,
        ops::{Deref, DerefMut},
        sync::Arc,
    };

    use crate::{
        collections::{Collection, SingleIncremintalInt},
        database_extention::DatabaseExt,
        gen_serde::{
            Deserialize, Serialize, deserialize,
            json_format_side::{JsonAsArcCursor, JsonFormat, PartialDeserialize},
            json_serialize_side::JsonAsString,
        },
        json_client::ToBind,
        sub_arc::ArcSubStr,
    };

    use super::DynamicCollection;

    pub(crate) type DynamicInput<S> = HashMap<ArcSubStr, Box<dyn ToBind<S> + Send>>;

    #[derive(Default)]
    pub struct CollectionToSerialize(
        pub(crate) HashMap<std::sync::Arc<str>, Box<dyn Serialize<JsonAsString> + Send>>,
    );

    impl Deref for CollectionToSerialize {
        type Target = HashMap<std::sync::Arc<str>, Box<dyn Serialize<JsonAsString> + Send>>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for CollectionToSerialize {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl fmt::Debug for CollectionToSerialize {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map().entries(self.0.iter()).finish()
        }
    }

    impl PartialEq for CollectionToSerialize {
        fn eq(&self, other: &Self) -> bool {
            self.0.len() == other.0.len()
                && self.0.iter().all(|(key, value)| {
                    other.0.get(key).is_some_and(|other| {
                        let left = value.as_ref() as &dyn Serialize<JsonAsString>;
                        let right = other.as_ref() as &dyn Serialize<JsonAsString>;
                        left == right
                    })
                })
        }
    }

    impl Eq for CollectionToSerialize {}

    impl Clone for CollectionToSerialize {
        fn clone(&self) -> Self {
            use crate::gen_serde::{SerializedJson, json_serialize_side::JsonAsString};

            Self(
                self.0
                    .iter()
                    .map(|(key, value)| {
                        let mut serialized = JsonAsString(String::new());
                        value.serialize(&mut serialized);
                        (
                            Arc::clone(key),
                            Box::new(SerializedJson(Arc::from(serialized.0.as_str())))
                                as Box<dyn Serialize<JsonAsString> + Send>,
                        )
                    })
                    .collect(),
            )
        }
    }

    impl Serialize<JsonAsString> for CollectionToSerialize {
        fn serialize(&self, ctx: &mut JsonAsString) {
            let mut keys: Vec<_> = self.0.keys().collect();
            keys.sort();
            ctx.0.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    ctx.0.push(',');
                }
                Serialize::serialize(key.as_ref(), ctx);
                ctx.0.push(':');
                Serialize::serialize(self.0.get(*key).unwrap().as_ref(), ctx);
            }
            ctx.0.push('}');
        }
    }

    impl Serialize<JsonAsString> for crate::operations::CollectionOutput<i64, CollectionToSerialize> {
        fn serialize(&self, ctx: &mut JsonAsString) {
            ctx.0.push('{');
            Serialize::serialize("id", ctx);
            ctx.0.push(':');
            self.id.serialize(ctx);
            ctx.0.push(',');
            Serialize::serialize("attributes", ctx);
            ctx.0.push(':');
            self.attributes.serialize(ctx);
            ctx.0.push('}');
        }
    }

    pub(crate) fn decode_json_scalar<T>(value: &Box<dyn Serialize<JsonAsString> + Send>) -> T
    where
        T: for<'de> Deserialize<'de, JsonAsArcCursor, Handler = ()>,
    {
        let mut json = JsonAsString::default();
        Serialize::serialize(value.as_ref(), &mut json);
        deserialize(Arc::from(json.0), (), JsonFormat).expect("decode json scalar")
    }

    pub(crate) fn dynamic_row_from_json<S>(
        json: std::sync::Arc<str>,
        dc: &std::sync::Arc<DynamicCollection<S>>,
    ) -> CollectionToSerialize
    where
        S: sqlx::Database + DatabaseExt,
        String: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
        bool: for<'q> sqlx::Encode<'q, S> + sqlx::Type<S>,
    {
        use crate::gen_serde::{
            Deserialize, DeserializeMap, DeserializeMapOrSeq, FromSerialized, MapOrSeq,
            json_format_side::PartialDeserialize,
        };
        use crate::sub_arc::ArcSubStr;

        let mut row = CollectionToSerialize::default();
        let mut cursor = json.start();

        match cursor.start_map_or_seq().expect("expected JSON object") {
            MapOrSeq::Map(mut map) => {
                while cursor.map_has_next(&mut map) {
                    let (key, partial): (ArcSubStr, PartialDeserialize) =
                        DeserializeMap::deserialize_with_unknown_key(&mut cursor, &mut map, (), ())
                            .expect("decode json field");

                    let Some(field) = dc
                        .fields
                        .iter()
                        .find(|field| field.name.as_ref() == key.as_str())
                    else {
                        continue;
                    };

                    let value = (field.type_info.partial_to_row_value)(field.is_optional, partial)
                        .expect("decode json field value");

                    row.insert(Arc::clone(&field.name.snake_case), value);
                }
                DeserializeMapOrSeq::finish_map(&mut cursor, map).expect("finish json object");
            }
            MapOrSeq::Seq(_) => panic!("expected JSON object"),
        }

        row
    }

    pub(crate) fn serialize_dynamic_row(row: &CollectionToSerialize, keys: &[&str]) -> String {
        let mut json = JsonAsString::default();
        json.0.push('{');
        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                json.0.push(',');
            }
            Serialize::serialize(*key, &mut json);
            json.0.push(':');
            let value = row
                .get(*key)
                .unwrap_or_else(|| panic!("missing field {key:?}"));
            Serialize::serialize(value.as_ref(), &mut json);
        }
        json.0.push('}');
        json.into()
    }

    impl<S> Collection for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        fn table_name(&self) -> &str {
            &self.collection_name.pascal_case
        }

        fn table_name_lower_case(&self) -> &str {
            &self.collection_name.snake_case
        }

        type InputData = DynamicInput<S>;
        type UpdateData = DynamicInput<S>;
        type OutputData = CollectionToSerialize;
        type Id = SingleIncremintalInt<Arc<str>>;

        fn id(&self) -> Self::Id {
            SingleIncremintalInt(Arc::clone(&self.collection_name.pascal_case))
        }
    }
}

pub(crate) use collection_impls::{CollectionToSerialize, DynamicInput as DynamicInsertInput};
pub(crate) use collection_impls::{
    decode_json_scalar, dynamic_row_from_json, serialize_dynamic_row,
};
pub(crate) use dynamic_insert_binds::DynamicUpdateInput;

pub(crate) mod common_expression_impls {
    use std::{ops::Not, sync::Arc};

    use crate::{
        database_extention::DatabaseExt,
        from_row::{FromRowAlias, FromRowData, FromRowError, from_row_v2::RowAliased},
        json_client::{
            ToBind,
            dynamic_collection::{
                CollectionToSerialize, DynamicCollection, DynamicField, DynamicInsertInput,
                DynamicUpdateInput,
            },
        },
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, OnInsert, OnUpdate, TableExpressions,
        },
        sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
        sqlx_query_builder::{RefExpression, Sanitize},
        sub_arc::ArcSubStr,
    };
    use sqlx::{ColumnIndex, Database, Row};

    pub struct ToBindSetMany<S> {
        pub vec: Vec<Box<dyn ToBind<S> + Send>>,
    }

    impl<S> OpExpression for ToBindSetMany<S> {
    }

    impl<'q, S> Expression<'q, S> for ToBindSetMany<S>
    where
        S: DatabaseExt,
    {
        fn expression(mut self, ctx: &mut StatementBuilder<'q, S>) {
            if self.vec.is_empty() {
                return;
            }

            let last = self.vec.pop();
            let mut first = true;

            for each in self.vec {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                ctx.bind(each);
            }

            if let Some(value) = last {
                if !first {
                    ctx.syntax(", ");
                }
                ctx.bind(value);
            }
        }
    }

    pub struct StoredMemberNames {
        pub cols: Vec<Arc<str>>,
    }

    impl crate::operations::operations_expressions_crossover::IdentifierColNames for StoredMemberNames {
        fn col_names(self) -> Vec<String> {
            self.cols.into_iter().map(|col| col.to_string()).collect()
        }
    }

    impl OpExpression for StoredMemberNames {
    }

    impl<'q, S> Expression<'q, S> for StoredMemberNames
    where
        S: DatabaseExt,
    {
        fn expression(mut self, ctx: &mut StatementBuilder<'q, S>) {
            if self.cols.is_empty() {
                return;
            }

            let last = self.cols.pop();
            let mut first = true;

            for col in self.cols {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                col.expression(ctx);
            }

            if let Some(col) = last {
                if !first {
                    ctx.syntax(", ");
                }
                col.expression(ctx);
            }
        }
    }

    impl<S> TableExpressions for DynamicCollection<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type SnakeCase = Arc<str>;
        type PascalCase = Arc<str>;

        fn table_name_snake_case(&self) -> Self::SnakeCase {
            Arc::clone(&self.collection_name.snake_case)
        }

        fn table_name_pascal_case(&self) -> Self::PascalCase {
            Arc::clone(&self.collection_name.pascal_case)
        }

        type InheritJoin = ();
        fn inherit_join(&self) -> Self::InheritJoin {}
    }

    impl<S> ExpressionsForOperation for DynamicCollection<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type Identifier = StoredMemberNames;

        fn identifier(&self) -> Self::Identifier {
            StoredMemberNames {
                cols: self
                    .fields
                    .iter()
                    .map(|field| Arc::clone(&field.name.snake_case))
                    .collect(),
            }
        }

        type Scoped = DynamicScopedMembers;

        fn scoped(&self) -> Self::Scoped {
            DynamicScopedMembers {
                table: Arc::clone(&self.collection_name.pascal_case),
                cols: self
                    .fields
                    .iter()
                    .map(|field| Arc::clone(&field.name.snake_case))
                    .collect(),
            }
        }

        type ScopedAliased = DynamicAliasedMembers;
        type NumScopedAliased = DynamicAliasedMembers;

        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            DynamicAliasedMembers {
                table: Arc::clone(&self.collection_name.pascal_case),
                cols: self
                    .fields
                    .iter()
                    .map(|field| Arc::clone(&field.name.snake_case))
                    .collect(),
                alias,
                num: None,
            }
        }

        fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
            DynamicAliasedMembers {
                table: Arc::clone(&self.collection_name.pascal_case),
                cols: self
                    .fields
                    .iter()
                    .map(|field| Arc::clone(&field.name.snake_case))
                    .collect(),
                alias,
                num: Some(num),
            }
        }
    }

    pub struct DynamicScopedMembers {
        pub table: Arc<str>,
        pub cols: Vec<Arc<str>>,
    }

    impl OpExpression for DynamicScopedMembers {
    }

    impl<'q, S> Expression<'q, S> for DynamicScopedMembers
    where
        S: DatabaseExt,
    {
        fn expression(mut self, ctx: &mut StatementBuilder<'q, S>) {
            if self.cols.is_empty() {
                return;
            }

            let last = self.cols.pop();
            let mut first = true;

            for col in &self.cols {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                ctx.sanitize(self.table.as_ref());
                ctx.syntax(".");
                ctx.sanitize(col.as_ref());
            }

            if let Some(col) = last {
                if !first {
                    ctx.syntax(", ");
                }
                ctx.sanitize(self.table.as_ref());
                ctx.syntax(".");
                ctx.sanitize(col.as_ref());
            }
        }
    }

    impl<S> OnInsert<DynamicInsertInput<S>> for DynamicCollection<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type InsertExpression = ToBindSetMany<S>;

        fn on_insert(&self, input: DynamicInsertInput<S>) -> Self::InsertExpression {
            let mut vec = Vec::with_capacity(self.fields.len());
            for field in &self.fields {
                let bind = input
                    .iter()
                    .find(|(key, _)| key.as_str() == field.name.as_ref())
                    .map(|(_, value)| value.clone_to_box())
                    .unwrap_or_else(|| Box::new(()) as Box<dyn ToBind<S> + Send>);
                vec.push(bind);
            }
            ToBindSetMany { vec }
        }

        type InsertId = ();

        fn on_insert_with_id(
            &self,
            input: DynamicInsertInput<S>,
        ) -> (Self::InsertId, Self::InsertExpression) {
            ((), self.on_insert(input))
        }
    }

    impl<S> FromRowData for DynamicCollection<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type RData = CollectionToSerialize;
    }

    impl<'r, S> FromRowAlias<'r, S::Row> for DynamicCollection<S>
    where
        S: DatabaseExt + sqlx::Database,
        for<'a> &'a str: ColumnIndex<S::Row>,
        String: for<'d> sqlx::Decode<'d, S> + sqlx::Type<S>,
        bool: for<'d> sqlx::Decode<'d, S> + sqlx::Type<S>,
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            let mut map = CollectionToSerialize::default();
            for field in &self.fields {
                let value =
                    (field.type_info.decode_from_row)(field.is_optional, field.name.as_ref(), row)
                        .map_err(|message| FromRowError::ColumnNotFound(message))?;
                map.insert(Arc::clone(&field.name.snake_case), value);
            }
            Ok(map)
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut map = CollectionToSerialize::default();
            for field in &self.fields {
                let col_name = format!("{}{}", row.alias, field.name.as_ref());
                let value = (field.type_info.decode_from_row)(
                    field.is_optional,
                    col_name.as_str(),
                    row.row,
                )
                .map_err(|message| FromRowError::ColumnNotFound(message))?;
                map.insert(Arc::clone(&field.name.snake_case), value);
            }
            Ok(map)
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            let mut map = CollectionToSerialize::default();
            for field in &self.fields {
                let col_name = format!(
                    "{}{}{}",
                    row.str_alias,
                    row.num_alias.map(|n| n.to_string()).unwrap_or_default(),
                    field.name.as_ref()
                );
                let value = (field.type_info.decode_from_row)(
                    field.is_optional,
                    col_name.as_str(),
                    row.row,
                )
                .map_err(FromRowError::ColumnNotFound)?;
                map.insert(Arc::clone(&field.name.snake_case), value);
            }
            Ok(map)
        }
    }

    fn aliased_column_name(alias: &str, num: Option<usize>, col: &str) -> String {
        match num {
            None => format!("{alias}{col}"),
            Some(num) => format!("{alias}{num}{col}"),
        }
    }

    pub struct DynamicAliasedMembers {
        pub table: Arc<str>,
        pub cols: Vec<Arc<str>>,
        pub alias: &'static str,
        pub num: Option<usize>,
    }

    impl OpExpression for DynamicAliasedMembers {
    }

    impl<'q, S> Expression<'q, S> for DynamicAliasedMembers
    where
        S: DatabaseExt,
    {
        fn expression(mut self, ctx: &mut StatementBuilder<'q, S>) {
            if self.cols.is_empty() {
                return;
            }

            let last = self.cols.pop();
            let mut first = true;

            for col in &self.cols {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                ctx.sanitize(self.table.as_ref());
                ctx.syntax(".");
                ctx.sanitize(col.as_ref());
                ctx.syntax(" AS ");
                ctx.sanitize(&aliased_column_name(self.alias, self.num, col));
            }

            if let Some(col) = last {
                if !first {
                    ctx.syntax(", ");
                }
                ctx.sanitize(self.table.as_ref());
                ctx.syntax(".");
                ctx.sanitize(col.as_ref());
                ctx.syntax(" AS ");
                ctx.sanitize(&aliased_column_name(self.alias, self.num, &col));
            }
        }
    }

    impl<S> OnUpdate<DynamicUpdateInput<S>> for DynamicCollection<S>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type UpdateExpression = DynamicUpdateSet<S>;

        fn on_update(&self, input: DynamicUpdateInput<S>) -> Self::UpdateExpression {
            DynamicUpdateSet {
                sets: input
                    .0
                    .into_iter()
                    .map(|(key, value)| (ArcSubStr::detach(&key), value))
                    .collect(),
            }
        }
    }

    pub struct DynamicUpdateSet<S> {
        pub sets: Vec<(Arc<str>, Box<dyn ToBind<S> + Send>)>,
    }

    impl<S> OpExpression for DynamicUpdateSet<S> {
        fn is_expression_present(&self) -> bool {
            !self.sets.is_empty()
        }
    }

    impl<'q, S> Expression<'q, S> for DynamicUpdateSet<S>
    where
        S: DatabaseExt,
    {
        fn expression(mut self, ctx: &mut StatementBuilder<'q, S>) {
            if self.sets.is_empty() {
                return;
            }

            let last = self.sets.pop();
            let mut first = true;

            for (name, bind) in self.sets {
                if !first {
                    ctx.syntax(", ");
                }
                first = false;
                ctx.sanitize(name.as_ref());
                ctx.syntax(" = ");
                ctx.bind(bind);
            }

            if let Some((name, bind)) = last {
                if !first {
                    ctx.syntax(", ");
                }
                ctx.sanitize(name.as_ref());
                ctx.syntax(" = ");
                ctx.bind(bind);
            }
        }
    }
}

mod arc_collection_impls {
    use std::sync::Arc;

    use crate::{
        database_extention::DatabaseExt,
        from_row::{FromRowAlias, FromRowData, FromRowError},
        json_client::dynamic_collection::{
            CollectionToSerialize, DynamicCollection, DynamicInsertInput, DynamicUpdateInput,
            common_expression_impls::{DynamicUpdateSet, ToBindSetMany},
        },
        operations::operations_expressions_crossover::{
            ExpressionsForOperation, OnInsert, OnUpdate, TableExpressions,
        },
        sub_arc::ArcSubStr,
    };
    use sqlx::{ColumnIndex, Database, Row};

    impl<S> TableExpressions for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type SnakeCase = Arc<str>;
        type PascalCase = Arc<str>;

        fn table_name_snake_case(&self) -> Self::SnakeCase {
            TableExpressions::table_name_snake_case(self.as_ref())
        }

        fn table_name_pascal_case(&self) -> Self::PascalCase {
            TableExpressions::table_name_pascal_case(self.as_ref())
        }

        type InheritJoin = ();
        fn inherit_join(&self) -> Self::InheritJoin {}
    }

    impl<S> ExpressionsForOperation for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type Identifier = super::common_expression_impls::StoredMemberNames;
        type Scoped = super::common_expression_impls::DynamicScopedMembers;
        type ScopedAliased = super::common_expression_impls::DynamicAliasedMembers;
        type NumScopedAliased = super::common_expression_impls::DynamicAliasedMembers;

        fn identifier(&self) -> Self::Identifier {
            ExpressionsForOperation::identifier(self.as_ref())
        }

        fn scoped(&self) -> Self::Scoped {
            ExpressionsForOperation::scoped(self.as_ref())
        }

        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
            ExpressionsForOperation::scoped_aliased(self.as_ref(), alias)
        }

        fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
            ExpressionsForOperation::num_scoped_aliased(self.as_ref(), num, alias)
        }
    }

    impl<S> OnInsert<DynamicInsertInput<S>> for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type InsertExpression = ToBindSetMany<S>;
        type InsertId = ();

        fn on_insert(&self, input: DynamicInsertInput<S>) -> Self::InsertExpression {
            OnInsert::on_insert(self.as_ref(), input)
        }

        fn on_insert_with_id(
            &self,
            input: DynamicInsertInput<S>,
        ) -> (Self::InsertId, Self::InsertExpression) {
            OnInsert::on_insert_with_id(self.as_ref(), input)
        }
    }

    impl<S> OnUpdate<DynamicUpdateInput<S>> for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type UpdateExpression = DynamicUpdateSet<S>;

        fn on_update(&self, input: DynamicUpdateInput<S>) -> Self::UpdateExpression {
            OnUpdate::on_update(self.as_ref(), input)
        }
    }

    impl<S> FromRowData for Arc<DynamicCollection<S>>
    where
        S: sqlx::Database + DatabaseExt,
    {
        type RData = CollectionToSerialize;
    }

    impl<'r, S> FromRowAlias<'r, S::Row> for Arc<DynamicCollection<S>>
    where
        S: DatabaseExt + sqlx::Database,
        DynamicCollection<S>: FromRowData<RData = CollectionToSerialize>,
        DynamicCollection<S>: FromRowAlias<'r, S::Row, RData = CollectionToSerialize>,
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            self.as_ref().no_alias(row)
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            self.as_ref().str_alias(row)
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, S::Row>,
        ) -> Result<Self::RData, FromRowError>
        where
            S::Row: Row,
        {
            self.as_ref().num_alias(row)
        }
    }
}

mod std_impls {
    use crate::database_extention::DatabaseExt;

    use super::{DynamicCollection, DynamicField};

    impl<S: DatabaseExt> PartialEq for DynamicCollection<S> {
        fn eq(&self, other: &Self) -> bool {
            self.collection_name == other.collection_name && self.fields == other.fields
        }
    }

    impl<S: DatabaseExt> Eq for DynamicCollection<S> {}

    impl<S: DatabaseExt> Clone for DynamicField<S> {
        fn clone(&self) -> Self {
            Self {
                name: self.name.clone(),
                type_info: self.type_info.clone(),
                is_optional: self.is_optional,
            }
        }
    }

    impl<S: DatabaseExt> PartialEq for DynamicField<S> {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
                && self.type_info == other.type_info
                && self.is_optional == other.is_optional
        }
    }

    impl<S: DatabaseExt> Eq for DynamicField<S> {}
}

mod impl_on_migrate {
    use std::ops::Not;
    use std::sync::Arc;

    use super::DynamicCollection;
    use crate::{
        database_extention::DatabaseExt,
        json_client::dynamic_collection::DynamicField,
        operations::operations_expressions_crossover::MigrateExpression,
        sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
    };
    use sqlx::ColumnIndex;

    pub struct MigrateDynamicCollection<S: DatabaseExt> {
        upper_case_name: Arc<str>,
        fields: Vec<DynamicField<S>>,
    }

    impl<S> MigrateExpression for DynamicCollection<S>
    where
        S: DatabaseExt,
        for<'a> &'a str: ColumnIndex<S::Row>,
        bool: sqlx::Type<S>,
        std::string::String: sqlx::Type<S>,
    {
        type Migrate = MigrateDynamicCollection<S>;

        fn migrate(&self) -> Self::Migrate {
            MigrateDynamicCollection {
                upper_case_name: Arc::clone(&self.collection_name.pascal_case),
                fields: self.fields.clone(),
            }
        }
    }

    impl<S: DatabaseExt> OpExpression for MigrateDynamicCollection<S> {
    }

    impl<'q, S> Expression<'q, S> for MigrateDynamicCollection<S>
    where
        S: DatabaseExt,
        S::IdExpression: Expression<'q, S>,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.syntax("CREATE TABLE ");
            ctx.sanitize(self.upper_case_name.as_ref());
            ctx.syntax(" ");
            ctx.syntax("(");

            S::id_on_create_table_expression().expression(ctx);

            for field in self.fields.into_iter() {
                ctx.syntax(&", ");
                ctx.sanitize(field.name.as_str());
                ctx.syntax(" ");
                (field.type_info.type_expression)().boxed_expression(ctx);
                if field.is_optional.not() {
                    ctx.syntax(&" NOT NULL");
                }
            }
            ctx.syntax(")");
            ctx.syntax(";");
        }
    }
}

macro_rules! default_executor {
    ($([$name:ident, $upper_case:ident]),*) => {};
}

pub use impl_on_migrate::MigrateDynamicCollection;
