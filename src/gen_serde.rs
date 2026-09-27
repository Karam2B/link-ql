#![allow(unused)]
pub trait FromSerialized<'de, Format> {
    type Deserializer: Deserializer<'de>;
    fn start(self) -> Self::Deserializer;
    fn terminate(
        bar: Self::Deserializer,
    ) -> Result<(), <Self::Deserializer as Deserializer<'de>>::Err>;
}

pub trait Serialize<Format> {
    fn serialize(&self, ctx: &mut Format);
}

mod impl_std_traits_for_trait_objects {
    use super::Serialize;
    use core::fmt;

    impl<T: fmt::Debug + Default> fmt::Debug for dyn Serialize<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut t = T::default();
            self.serialize(&mut t);
            t.fmt(f)?;
            Ok(())
        }
    }

    impl<T: fmt::Debug + Default> fmt::Debug for dyn Serialize<T> + Send {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut t = T::default();
            self.serialize(&mut t);
            t.fmt(f)?;
            Ok(())
        }
    }

    impl<T: PartialEq + Default> PartialEq for dyn Serialize<T> {
        fn eq(&self, other: &Self) -> bool {
            let mut t1 = T::default();
            self.serialize(&mut t1);
            let mut t2 = T::default();
            other.serialize(&mut t2);
            t1 == t2
        }
    }

    impl<T: Eq + Default> Eq for dyn Serialize<T> {}
}

mod impl_my_types {
    use crate::gen_serde::{
        ListEncoding, ObjectEncoding, Serialize, json_serialize_side::JsonAsString,
    };

    impl<F> Serialize<F> for Box<dyn Serialize<F>> {
        fn serialize(&self, ctx: &mut F) {
            Serialize::serialize(&**self, ctx)
        }
    }

    impl<F> Serialize<F> for Box<dyn Serialize<F> + Send> {
        fn serialize(&self, ctx: &mut F) {
            Serialize::serialize(&**self, ctx)
        }
    }

    impl<F> Serialize<F> for Box<dyn Serialize<F> + Send + Sync> {
        fn serialize(&self, ctx: &mut F) {
            Serialize::serialize(&**self, ctx)
        }
    }

    impl<F, T> Serialize<F> for Vec<T>
    where
        T: Serialize<F>,
        F: ListEncoding,
    {
        fn serialize(&self, ctx: &mut F) {
            let mut list = ctx.serialize_start();
            for value in self {
                ctx.serialize_value(&mut list, value);
            }
            ctx.serialize_end(list);
        }
    }
}

pub trait Deserializer<'de>: Sized {
    type Err;
    type Format;
}

pub trait ObjectEncoding {
    type Object;
    fn serialize_start(&mut self) -> Self::Object;
    fn serialize_pair<K, V>(
        &mut self,
        object: &mut <Self as ObjectEncoding>::Object,
        key: &K,
        value: &V,
    ) where
        Self: Sized,
        K: Serialize<Self> + ?Sized,
        V: Serialize<Self> + ?Sized;
    fn join(&mut self, object: &mut Self::Object);
    fn serialize_end(&mut self, object: Self::Object);
}

pub trait ListEncoding {
    type List;
    fn serialize_start(&mut self) -> Self::List;
    fn serialize_value<T>(&mut self, list: &mut Self::List, value: &T)
    where
        T: Serialize<Self>,
        Self: Sized;
    fn serialize_end(&mut self, list: Self::List);
}

/// Declares **how** a type is deserialized *before* choosing a [`Deserializer`]: the [`Handler`]
/// type is chosen only by this trait’s impl author (the type owner). [`Deserialize`] for a concrete
/// `S` never introduces alternative handlers per format.
pub trait DeserializeSpec: Sized {
    type Handler;
}

/// Deserialize from `S` using a [`DeserializeSpec::Handler`] value.
pub trait Deserialize<'de, S>
where
    S: Deserializer<'de>,
    Self: DeserializeSpec + Sized,
{
    fn deserialize(handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err>;
}

/// Associated “lookup view” of a key, possibly borrowing from `Key` (see [`KnownKey::info`]).
pub trait KnownKeyInfo {
    type Info<'a>
    where
        Self: 'a;
}

/// Lookup for a key known by value
pub trait KnownKey<Key>: KnownKeyInfo {
    fn info<'k>(key: &'k Key) -> Self::Info<'k>;
}

/// Lookup for a key unknown by only type
pub trait UnknownKey<S> {}

pub trait DeserializeMap<'de>: Deserializer<'de> {
    type MapAccess;
    fn start_map(&mut self) -> Result<Self::MapAccess, Self::Err>;

    fn map_has_next(&self, map: &Self::MapAccess) -> bool;

    /// deserialize pairs with unknown key value -- the deserialize impl does not know the value of the key, only knows its type
    fn deserialize_with_unknown_key<Key, Value>(
        &mut self,
        map: &mut Self::MapAccess,
        key_handler: Key::Handler,
        value_handler: Value::Handler,
    ) -> Result<(Key, Value), Self::Err>
    where
        Key: UnknownKey<Self> + Deserialize<'de, Self>,
        Value: Deserialize<'de, Self>;

    /// deserialize pairs with known key value -- the deserialize impl known the value of the key
    fn deserialize_with_known_key<Key, Value>(
        &mut self,
        map: &mut Self::MapAccess,
        key: Key,
        value_handler: Value::Handler,
    ) -> Result<Value, Self::Err>
    where
        Self: KnownKey<Key>,
        Value: DeserializeSpec + Deserialize<'de, Self>;

    fn finish(&mut self, map: Self::MapAccess) -> Result<(), Self::Err>;
}

pub trait DeserializeSeq<'de>: Deserializer<'de> {
    type SeqAccess;
    fn start_seq(&mut self) -> Result<Self::SeqAccess, Self::Err>;
    fn deserialize_value<T>(
        &mut self,
        seq: &mut Self::SeqAccess,
        handler: T::Handler,
    ) -> Result<T, Self::Err>
    where
        T: DeserializeSpec + Deserialize<'de, Self>;
    /// Returns `false` when the next non-whitespace token is the sequence closing delimiter.
    fn seq_has_next(&mut self, seq: &mut Self::SeqAccess) -> Result<bool, Self::Err>;
    fn finish(&mut self, seq: Self::SeqAccess) -> Result<(), Self::Err>;
}

pub enum MapOrSeq<Map, Seq> {
    Map(Map),
    Seq(Seq),
}

pub trait DeserializeMapOrSeq<'de>: DeserializeMap<'de> {
    type SeqAccess;
    fn start_map_or_seq(&mut self)
    -> Result<MapOrSeq<Self::MapAccess, Self::SeqAccess>, Self::Err>;

    fn deserialize_pair_unknown_key<Key, Value>(
        &mut self,
        map: &mut Self::MapAccess,
        key_handler: Key::Handler,
        value_handler: Value::Handler,
    ) -> Result<(Key, Value), Self::Err>
    where
        Key: UnknownKey<Self> + DeserializeSpec + Deserialize<'de, Self>,
        Value: DeserializeSpec + Deserialize<'de, Self>,
    {
        DeserializeMap::deserialize_with_unknown_key(self, map, key_handler, value_handler)
    }

    fn deserialize_pair_known_key<Key, Value>(
        &mut self,
        map: &mut Self::MapAccess,
        key: Key,
        value_handler: Value::Handler,
    ) -> Result<Value, Self::Err>
    where
        Self: KnownKey<Key>,
        Value: DeserializeSpec + Deserialize<'de, Self>,
    {
        DeserializeMap::deserialize_with_known_key(self, map, key, value_handler)
    }

    fn deserialize_value<T>(
        &mut self,
        seq: &mut Self::SeqAccess,
        handler: T::Handler,
    ) -> Result<T, Self::Err>
    where
        T: DeserializeSpec + Deserialize<'de, Self>;
    fn finish_map(&mut self, map: Self::MapAccess) -> Result<(), Self::Err>;
    fn finish_seq(&mut self, seq: Self::SeqAccess) -> Result<(), Self::Err>;
}

mod impl_std_types {
    use super::{Deserialize, DeserializeSeq, DeserializeSpec};

    impl<T: DeserializeSpec> DeserializeSpec for Vec<T> {
        type Handler = T::Handler;
    }

    impl<'de, S, T> Deserialize<'de, S> for Vec<T>
    where
        S: DeserializeSeq<'de>,
        T: DeserializeSpec + Deserialize<'de, S>,
        T::Handler: Clone,
        S::Err: From<&'static str>,
    {
        fn deserialize(handler: T::Handler, serialized: &mut S) -> Result<Self, S::Err> {
            let mut seq = DeserializeSeq::start_seq(serialized)?;
            let mut out = Vec::new();
            while DeserializeSeq::seq_has_next(serialized, &mut seq)? {
                out.push(DeserializeSeq::deserialize_value(
                    serialized,
                    &mut seq,
                    handler.clone(),
                )?);
            }
            DeserializeSeq::finish(serialized, seq)?;
            Ok(out)
        }
    }
}

#[cfg(test)]
mod dynamic_client_side {
    use std::sync::Arc;

    use crate::gen_serde::{
        Deserialize, DeserializeMap, DeserializeMapOrSeq, DeserializeSpec, Deserializer,
        FromSerialized, KnownKey, MapOrSeq, deserialize, json_format_side::JsonFormat,
    };

    #[derive(Debug, PartialEq, Eq)]
    pub enum StringOrInt {
        String(String),
        Int(i64),
    }

    /// Runtime configuration for [`DynamicExample`]: which JSON shape to expect for `another_field`.
    #[derive(Clone, Copy)]
    pub struct DynamicExampleHandler {
        pub another_field_is_string: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct DynamicExample {
        pub title: String,
        pub another_field: StringOrInt,
    }

    impl DeserializeSpec for DynamicExample {
        type Handler = DynamicExampleHandler;
    }

    impl<'de, S> Deserialize<'de, S> for DynamicExample
    where
        S: Deserializer<'de>,
        S: DeserializeMapOrSeq<'de>,
        String: Deserialize<'de, S>,
        i64: Deserialize<'de, S>,
        S: KnownKey<&'static str>,
        S::Err: From<&'static str>,
    {
        fn deserialize(handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map) => {
                    let title = DeserializeMap::deserialize_with_known_key(
                        serialized,
                        &mut map,
                        "title",
                        (),
                    )?;
                    let another_field = if handler.another_field_is_string {
                        StringOrInt::String(DeserializeMap::deserialize_with_known_key(
                            serialized,
                            &mut map,
                            "another_field",
                            (),
                        )?)
                    } else {
                        StringOrInt::Int(DeserializeMap::deserialize_with_known_key(
                            serialized,
                            &mut map,
                            "another_field",
                            (),
                        )?)
                    };
                    DeserializeMapOrSeq::finish_map(serialized, map)?;
                    Ok(DynamicExample {
                        title,
                        another_field,
                    })
                }
                MapOrSeq::Seq(_) => Err(S::Err::from("expected JSON object for DynamicExample")),
            }
        }
    }

    #[test]
    fn te_dynamic_example_string_branch() {
        let s: Arc<str> = Arc::from(r#"{"title":"hi","another_field":"from_json"}"#.to_string());
        let got: DynamicExample = deserialize(
            s,
            DynamicExampleHandler {
                another_field_is_string: true,
            },
            JsonFormat,
        )
        .unwrap();
        assert_eq!(
            got,
            DynamicExample {
                title: "hi".into(),
                another_field: StringOrInt::String("from_json".into()),
            }
        );
    }

    #[test]
    fn te_dynamic_example_int_branch() {
        let s: Arc<str> = Arc::from(r#"{"title":"hi","another_field":42}"#.to_string());
        let got: DynamicExample = deserialize(
            s,
            DynamicExampleHandler {
                another_field_is_string: false,
            },
            JsonFormat,
        )
        .unwrap();
        assert_eq!(
            got,
            DynamicExample {
                title: "hi".into(),
                another_field: StringOrInt::Int(42),
            }
        );
    }
}

#[cfg(test)]
mod client_side {
    use axum::Json;

    use crate::gen_serde::{
        Deserialize, DeserializeMap, DeserializeMapOrSeq, DeserializeSpec, Deserializer,
        FromSerialized, IntoSerializedString, KnownKey, MapOrSeq, ObjectEncoding, Serialize,
        deserialize,
        json_format_side::{JsonFormat, PartialDeserialize},
        json_serialize_side::JsonAsString,
    };
    use crate::sub_arc::ArcSubStr;
    use std::sync::Arc;

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct BasicObject {
        pub title: String,
    }

    impl DeserializeSpec for BasicObject {
        type Handler = ();
    }

    impl<'de, S> Deserialize<'de, S> for BasicObject
    where
        S: Deserializer<'de>,
        S: DeserializeMapOrSeq<'de>,
        String: Deserialize<'de, S>,
        S: KnownKey<&'static str>,
    {
        fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map) => {
                    let title = DeserializeMap::deserialize_with_known_key(
                        serialized,
                        &mut map,
                        "title",
                        (),
                    )?;
                    DeserializeMapOrSeq::finish_map(serialized, map)?;
                    Ok(Self { title })
                }
                MapOrSeq::Seq(mut seq) => {
                    let title = serialized.deserialize_value::<String>(&mut seq, ())?;
                    DeserializeMapOrSeq::finish_seq(serialized, seq)?;
                    Ok(Self { title })
                }
            }
        }
    }

    #[test]
    fn te_object() {
        let s: Arc<str> = Arc::from(r#"{"title":"first_title"}"#.to_string());

        let s: BasicObject = deserialize(s, (), JsonFormat).unwrap();

        pretty_assertions::assert_eq!(
            s,
            BasicObject {
                title: String::from("first_title")
            }
        );
    }

    #[test]
    fn te_array() {
        let s: Arc<str> = Arc::from(r#"["first_title"]"#.to_string());

        let s: BasicObject = deserialize(s, (), JsonFormat).unwrap();

        pretty_assertions::assert_eq!(
            s,
            BasicObject {
                title: String::from("first_title")
            }
        );
    }

    impl<Format> Serialize<Format> for BasicObject
    where
        String: Serialize<Format>,
        str: Serialize<Format>,
        Format: ObjectEncoding,
    {
        fn serialize(&self, fmt: &mut Format) {
            let mut object = Format::serialize_start(fmt);

            Format::serialize_pair(fmt, &mut object, "title", &self.title);

            Format::serialize_end(fmt, object);
        }
    }

    const _: () = {
        struct Format;
        fn _check_if_trait_o() -> Box<dyn Serialize<Format>> {
            todo!()
        }
    };

    fn te_serialize_basic_object() {
        let s = BasicObject {
            title: String::from("first_title"),
        };
        let v = IntoSerializedString::<JsonAsString>::serialize_to_string(&s);
        assert_eq!(v, r#"{"title":"first_title"}"#);
    }

    #[test]
    fn te_i64_json() {
        let s: Arc<str> = Arc::from(" 42 ".to_string());
        let v: i64 = deserialize(s, (), JsonFormat).unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn te_f64_json() {
        let s: Arc<str> = Arc::from("1.25e2".to_string());
        let v: f64 = deserialize(s, (), JsonFormat).unwrap();
        assert!((v - 125.0).abs() < 1e-9);
    }

    #[test]
    fn te_unit_null() {
        let s: Arc<str> = Arc::from("  null  ".to_string());
        let _: () = deserialize(s, (), JsonFormat).unwrap();
    }

    #[test]
    fn te_option_string() {
        let none: Arc<str> = Arc::from("null".to_string());
        let v: Option<String> = deserialize(none, (), JsonFormat).unwrap();
        assert_eq!(v, None);

        let some: Arc<str> = Arc::from("\"x\"".to_string());
        let v: Option<String> = deserialize(some, (), JsonFormat).unwrap();
        assert_eq!(v.as_deref(), Some("x"));
    }

    #[test]
    fn te_string_arc_ref_zero_copy() {
        let inner: Arc<str> = Arc::from(r#"  "plain"  "#.to_string());
        let doc = inner.clone();
        let got: ArcSubStr = deserialize(doc, (), JsonFormat).unwrap();
        assert_eq!(got.as_str(), "plain");
        assert!(Arc::ptr_eq(&inner, got.backing_arc()));
        assert_eq!(got.as_str(), &inner[3..8]);
    }

    #[test]
    fn te_string_arc_ref_escaped() {
        let inner: Arc<str> = Arc::from(r#""a\"b\\c""#.to_string());
        let doc = inner.clone();
        let got: ArcSubStr = deserialize(doc, (), JsonFormat).unwrap();
        assert_eq!(got.as_str(), r#"a"b\c"#);
        assert!(!Arc::ptr_eq(&inner, got.backing_arc()));
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Collection {
        pub identifier: String,
        pub data: PartialDeserialize,
    }

    impl DeserializeSpec for Collection {
        type Handler = ();
    }

    impl<'de, S> Deserialize<'de, S> for Collection
    where
        S: Deserializer<'de>,
        S: DeserializeMapOrSeq<'de>,
        String: Deserialize<'de, S>,
        PartialDeserialize: Deserialize<'de, S>,
        S: KnownKey<&'static str>,
        S::Err: From<&'static str>,
    {
        fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map) => {
                    let identifier = DeserializeMap::deserialize_with_known_key(
                        serialized,
                        &mut map,
                        "identifier",
                        (),
                    )?;
                    let data = DeserializeMap::deserialize_with_known_key(
                        serialized,
                        &mut map,
                        "data",
                        (),
                    )?;
                    DeserializeMapOrSeq::finish_map(serialized, map)?;
                    Ok(Collection { identifier, data })
                }
                MapOrSeq::Seq(_) => Err(S::Err::from("expected JSON object for Collection")),
            }
        }
    }

    #[test]
    fn partial_des() {
        let s: Arc<str> =
            Arc::from(r#"{"identifier":"todo", "data": {"title": "from_string_key"}}"#.to_string());

        let got: Collection = deserialize(s, (), JsonFormat).unwrap();

        assert_eq!(got.identifier, "todo");
        assert_eq!(got.data.0.as_str(), r#"{"title": "from_string_key"}"#);
        let data: BasicObject = got.data.continue_deserialize().unwrap();
        assert_eq!(
            data,
            BasicObject {
                title: "from_string_key".to_string()
            }
        );
    }

    /// Same shape as [`DeTodo`], but uses a runtime [`String`] key with [`KnownKey<String>`].
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct DeTodoStringKey {
        pub title: String,
    }

    impl DeserializeSpec for DeTodoStringKey {
        type Handler = ();
    }

    impl<'de, S> Deserialize<'de, S> for DeTodoStringKey
    where
        S: Deserializer<'de>,
        S: DeserializeMapOrSeq<'de>,
        String: Deserialize<'de, S>,
        S: KnownKey<String>,
    {
        fn deserialize(_handler: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
            match serialized.start_map_or_seq()? {
                MapOrSeq::Map(mut map) => {
                    let title = DeserializeMap::deserialize_with_known_key(
                        serialized,
                        &mut map,
                        String::from("title"),
                        (),
                    )?;
                    DeserializeMapOrSeq::finish_map(serialized, map)?;
                    Ok(Self { title })
                }
                MapOrSeq::Seq(mut seq) => {
                    let title = serialized.deserialize_value::<String>(&mut seq, ())?;
                    DeserializeMapOrSeq::finish_seq(serialized, seq)?;
                    Ok(Self { title })
                }
            }
        }
    }

    #[test]
    fn te_object_known_key_string() {
        let s: Arc<str> = Arc::from(r#"{"title":"from_string_key"}"#.to_string());
        let got: DeTodoStringKey = deserialize(s, (), JsonFormat).unwrap();
        assert_eq!(
            got,
            DeTodoStringKey {
                title: "from_string_key".into(),
            }
        );
    }
}

pub(crate) mod json_serialize_side {

    use crate::gen_serde::{ListEncoding, ObjectEncoding, Serialize};

    use super::json_format_side::JsonFormat;

    #[derive(Default)]
    pub struct JsonAsBuffer(pub Vec<u8>);
    #[derive(Debug, PartialEq, Eq, Default)]
    pub struct JsonAsString(pub String);

    fn append_json_string(out: &mut String, value: &str) {
        out.push('"');
        for ch in value.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => {
                    use std::fmt::Write;
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
        out.push('"');
    }

    impl Into<Vec<u8>> for JsonAsBuffer {
        fn into(self) -> Vec<u8> {
            self.0
        }
    }

    impl Into<String> for JsonAsString {
        fn into(self) -> String {
            self.0
        }
    }

    impl Into<Vec<u8>> for JsonAsString {
        fn into(self) -> Vec<u8> {
            self.0.into()
        }
    }

    impl Serialize<JsonAsString> for str {
        fn serialize(&self, ctx: &mut JsonAsString) {
            append_json_string(&mut ctx.0, self);
        }
    }

    impl Serialize<JsonAsString> for String {
        fn serialize(&self, ctx: &mut JsonAsString) {
            append_json_string(&mut ctx.0, self);
        }
    }

    impl Serialize<JsonAsString> for bool {
        fn serialize(&self, ctx: &mut JsonAsString) {
            ctx.0.push_str(if *self { "true" } else { "false" });
        }
    }

    impl Serialize<JsonAsString> for () {
        fn serialize(&self, ctx: &mut JsonAsString) {
            ctx.0.push_str("null");
        }
    }

    impl<T> Serialize<JsonAsString> for Option<T>
    where
        T: Serialize<JsonAsString>,
    {
        fn serialize(&self, ctx: &mut JsonAsString) {
            match self {
                None => ctx.0.push_str("null"),
                Some(value) => value.serialize(ctx),
            }
        }
    }

    impl Serialize<JsonAsString> for i64 {
        fn serialize(&self, ctx: &mut JsonAsString) {
            use std::fmt::Write;
            let _ = write!(ctx.0, "{self}");
        }
    }

    impl Serialize<JsonAsString> for f64 {
        fn serialize(&self, ctx: &mut JsonAsString) {
            use std::fmt::Write;
            let _ = write!(ctx.0, "{self}");
        }
    }

    macro_rules! impl_serialize_json_vec {
        ($t:ty) => {
            impl Serialize<JsonAsString> for sqlx::types::Json<Vec<$t>>
            where
                $t: Serialize<JsonAsString>,
            {
                fn serialize(&self, ctx: &mut JsonAsString) {
                    self.0.serialize(ctx);
                }
            }
        };
    }

    impl_serialize_json_vec!(String);
    impl_serialize_json_vec!(bool);
    impl_serialize_json_vec!(i64);
    impl_serialize_json_vec!(f64);

    impl ObjectEncoding for JsonAsString {
        /// `true` while the first key/value pair is still pending.
        type Object = bool;

        fn serialize_start(&mut self) -> Self::Object {
            self.0.push('{');
            true
        }

        fn serialize_pair<K, V>(&mut self, first: &mut Self::Object, key: &K, value: &V)
        where
            Self: Sized,
            K: Serialize<Self> + ?Sized,
            V: Serialize<Self> + ?Sized,
        {
            if !*first {
                self.0.push(',');
            }
            *first = false;
            key.serialize(self);
            self.0.push(':');
            value.serialize(self);
        }

        fn join(&mut self, _object: &mut Self::Object) {}

        fn serialize_end(&mut self, _object: Self::Object) {
            self.0.push('}');
        }
    }

    impl ListEncoding for JsonAsString {
        /// `true` while the first list element is still pending.
        type List = bool;

        fn serialize_start(&mut self) -> Self::List {
            self.0.push('[');
            true
        }

        fn serialize_value<T>(&mut self, first: &mut Self::List, value: &T)
        where
            T: Serialize<Self>,
            Self: Sized,
        {
            if !*first {
                self.0.push(',');
            }
            *first = false;
            value.serialize(self);
        }

        fn serialize_end(&mut self, _list: Self::List) {
            self.0.push(']');
        }
    }
}

pub(crate) mod json_format_side {
    use std::ops::Range;
    use std::sync::Arc;

    use crate::gen_serde::{
        Deserialize, DeserializeMap, DeserializeMapOrSeq, DeserializeSeq, DeserializeSpec,
        Deserializer, FromSerialized, KnownKey, KnownKeyInfo, MapOrSeq, UnknownKey, deserialize,
    };
    use crate::sub_arc::ArcSubStr;

    pub struct JsonAsArcCursor {
        pub inner: Arc<str>,
        pub start: usize,
    }

    pub struct JsonFormat;

    impl<'de> FromSerialized<'de, JsonFormat> for Arc<str> {
        type Deserializer = JsonAsArcCursor;

        fn start(self) -> Self::Deserializer {
            JsonAsArcCursor {
                inner: self,
                start: 0,
            }
        }

        fn terminate(
            bar: Self::Deserializer,
        ) -> Result<(), <Self::Deserializer as Deserializer<'de>>::Err> {
            let mut chars = bar.inner[bar.start..].chars().into_iter();

            while let Some(next) = chars.next() {
                if next.is_whitespace() {
                    continue;
                } else {
                    return Err(format!("expected whitespace found: {:?}", next));
                }
            }

            Ok(())
        }
    }

    impl<'de> Deserializer<'de> for JsonAsArcCursor {
        type Err = String;
        type Format = JsonFormat;
    }

    impl DeserializeSpec for String {
        type Handler = ();
    }

    impl<'de> Deserialize<'de, JsonAsArcCursor> for String {
        fn deserialize(
            _handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let mut chars = serialized.inner[serialized.start..].chars().into_iter();

            let mut output = String::new();

            let mut peek = chars.next();
            while let Some(next) = peek {
                if next.is_whitespace() {
                    serialized.start += next.len_utf8();
                    peek = chars.next();
                    continue;
                }
                if next == '"' {
                    serialized.start += next.len_utf8();
                    peek = chars.next();
                    break;
                }
                return Err(format!("expected string found: {:?}", next));
            }

            while let Some(next) = peek {
                if next == '"' {
                    serialized.start += next.len_utf8();
                    break;
                }
                if next == '\\' {
                    let adv = match chars.next() {
                        Some(e) if e == '"' || e == '\\' => {
                            output.push(e);
                            '\\'.len_utf8() + e.len_utf8()
                        }
                        Some(n) => return Err(format!("unused escape at {:?}", n))?,
                        None => return Err("terminated at escape")?,
                    };
                    serialized.start += adv;
                } else {
                    output.push(next);
                    serialized.start += next.len_utf8();
                }
                peek = chars.next();
            }

            Ok(output)
        }
    }

    impl DeserializeSpec for bool {
        type Handler = ();
    }

    impl<'de> Deserialize<'de, JsonAsArcCursor> for bool {
        fn deserialize(
            _handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let s = serialized.inner.as_ref();
            let i = skip_ws_json(s, serialized.start);
            if s.get(i..i + 4) == Some("true") {
                serialized.start = i + 4;
                return Ok(true);
            }
            if s.get(i..i + 5) == Some("false") {
                serialized.start = i + 5;
                return Ok(false);
            }
            Err(format!(
                "expected bool, found {:?}",
                s.get(i..s.len().min(i + 8))
            ))
        }
    }

    macro_rules! impl_deserialize_json_int_for {
        ($($t:ty),* $(,)?) => {$(
            impl DeserializeSpec for $t {
                type Handler = ();
            }

            impl<'de> Deserialize<'de, JsonAsArcCursor> for $t {
                fn deserialize(
                    _handler: Self::Handler,
                    serialized: &mut JsonAsArcCursor,
                ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
                    deserialize_json_number_str(serialized)?
                        .parse::<$t>()
                        .map_err(|e: std::num::ParseIntError| e.to_string())
                }
            }
        )*};
    }

    macro_rules! impl_deserialize_json_float_for {
        ($($t:ty),* $(,)?) => {$(
            impl DeserializeSpec for $t {
                type Handler = ();
            }

            impl<'de> Deserialize<'de, JsonAsArcCursor> for $t {
                fn deserialize(
                    _handler: Self::Handler,
                    serialized: &mut JsonAsArcCursor,
                ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
                    deserialize_json_number_str(serialized)?
                        .parse::<$t>()
                        .map_err(|e: std::num::ParseFloatError| e.to_string())
                }
            }
        )*};
    }

    impl_deserialize_json_int_for!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
    );
    impl_deserialize_json_float_for!(f32, f64);

    macro_rules! impl_deserialize_json_vec {
        ($t:ty) => {
            impl DeserializeSpec for sqlx::types::Json<Vec<$t>> {
                type Handler = ();
            }

            impl<'de> Deserialize<'de, JsonAsArcCursor> for sqlx::types::Json<Vec<$t>>
            where
                Vec<$t>: Deserialize<'de, JsonAsArcCursor>,
            {
                fn deserialize(
                    _handler: Self::Handler,
                    serialized: &mut JsonAsArcCursor,
                ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
                    Ok(sqlx::types::Json(Vec::<$t>::deserialize((), serialized)?))
                }
            }
        };
    }

    impl_deserialize_json_vec!(String);
    impl_deserialize_json_vec!(bool);
    impl_deserialize_json_vec!(i64);
    impl_deserialize_json_vec!(f64);

    impl DeserializeSpec for () {
        type Handler = ();
    }

    impl<'de> Deserialize<'de, JsonAsArcCursor> for () {
        fn deserialize(
            _handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let s = serialized.inner.as_ref();
            let i = skip_ws_json(s, serialized.start);
            if s.len() < i + 4 || &s[i..i + 4] != "null" {
                return Err(format!(
                    "expected null for unit type, found {:?}",
                    s.get(i..s.len().min(i + 8))
                ));
            }
            serialized.start = i + 4;
            Ok(())
        }
    }

    impl<T: DeserializeSpec> DeserializeSpec for Option<T> {
        type Handler = T::Handler;
    }

    impl<'de, T> Deserialize<'de, JsonAsArcCursor> for Option<T>
    where
        T: DeserializeSpec + Deserialize<'de, JsonAsArcCursor>,
    {
        fn deserialize(
            handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let s = serialized.inner.as_ref();
            let i = skip_ws_json(s, serialized.start);
            if s.len() >= i + 4 && &s[i..i + 4] == "null" {
                serialized.start = i + 4;
                return Ok(None);
            }
            Ok(Some(T::deserialize(handler, serialized)?))
        }
    }

    /// Holds one complete JSON value as a subslice of the original buffer (not parsed to `T` yet).
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct PartialDeserialize(pub ArcSubStr);

    impl PartialDeserialize {
        /// Parse the captured slice as `T` using the same [`JsonFormat`] rules as the outer cursor.
        pub fn continue_deserialize<T>(&self) -> Result<T, String>
        where
            T: for<'de> Deserialize<'de, JsonAsArcCursor, Handler = ()>,
        {
            let slice: Arc<str> = Arc::from(self.0.as_str());
            deserialize(slice, (), JsonFormat)
        }
        pub fn continue_deserialize_with<T>(&self, handler: T::Handler) -> Result<T, String>
        where
            T: for<'de> Deserialize<'de, JsonAsArcCursor>,
        {
            let slice: Arc<str> = Arc::from(self.0.as_str());
            deserialize(slice, handler, JsonFormat)
        }
    }

    /// Captures one complete JSON value as a [`Range`] in `inner`, advances the cursor past it.
    impl DeserializeSpec for PartialDeserialize {
        type Handler = ();
    }

    impl<'de> Deserialize<'de, JsonAsArcCursor> for PartialDeserialize {
        fn deserialize(
            _handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let s = serialized.inner.as_ref();
            let start = skip_ws_json(s, serialized.start);
            let end = end_of_json_value(s, start)?;
            let range = start..end;
            serialized.start = end;
            Ok(PartialDeserialize(ArcSubStr::new(
                Arc::clone(&serialized.inner),
                range,
            )))
        }
    }

    impl DeserializeSpec for ArcSubStr {
        type Handler = ();
    }

    impl<'de> Deserialize<'de, JsonAsArcCursor> for ArcSubStr {
        fn deserialize(
            _handler: Self::Handler,
            serialized: &mut JsonAsArcCursor,
        ) -> Result<Self, <JsonAsArcCursor as Deserializer<'de>>::Err> {
            let (parsed, end) = parse_json_arc_substr(&serialized.inner, serialized.start)?;
            serialized.start = end;
            Ok(parsed)
        }
    }

    const OPEN_BRACE: char = '{';
    const CLOSE_BRACE: char = '}';
    const OPEN_BRACE_BYTE: u8 = OPEN_BRACE as u8;
    const CLOSE_BRACE_BYTE: u8 = CLOSE_BRACE as u8;

    /// Built in [`JsonAsArcCursor::start_map`]: maps JSON object keys to half-open byte ranges of each **value** in `inner`.
    pub struct JsonMapAccess {
        pub entries: Vec<(
            Arc<str>,
            Range<usize>, // key
            Range<usize>, // value
        )>,
        /// Byte offset in `inner` just after the closing `}` of this object.
        pub after_object: usize,
    }

    fn skip_ws_json(s: &str, mut i: usize) -> usize {
        let b = s.as_bytes();
        while i < b.len() {
            match b[i] {
                b' ' | b'\t' | b'\n' | b'\r' => i += 1,
                _ => break,
            }
        }
        i
    }

    /// `open_quote_at` must point at `"`. Returns decoded contents and exclusive end index.
    fn parse_json_string(s: &str, open_quote_at: usize) -> Result<(String, usize), String> {
        let b = s.as_bytes();
        if b.get(open_quote_at) != Some(&b'"') {
            return Err(format!("expected '\"' at byte {}", open_quote_at));
        }
        let mut i = open_quote_at + 1;
        let mut out = String::new();
        while i < b.len() {
            match b[i] {
                b'"' => return Ok((out, i + 1)),
                b'\\' => {
                    i += 1;
                    match b.get(i) {
                        Some(&ch) if ch == b'"' || ch == b'\\' => {
                            out.push(ch as char);
                            i += 1;
                        }
                        Some(_) => return Err(format!("unsupported escape at {}", i)),
                        None => return Err("terminated inside escape".into()),
                    }
                }
                _ => {
                    let ch = s[i..]
                        .chars()
                        .next()
                        .ok_or_else(|| "eof in string".to_string())?;
                    i += ch.len_utf8();
                    out.push(ch);
                }
            }
        }
        Err("unterminated string".into())
    }

    /// Like [`parse_json_string`], but returns an [`ArcSubStr`] that borrows from `inner` when
    /// the JSON string contains no backslash escapes; otherwise returns decoded text in a new `Arc`.
    fn parse_json_arc_substr(inner: &Arc<str>, start: usize) -> Result<(ArcSubStr, usize), String> {
        let s = inner.as_ref();
        let mut i = skip_ws_json(s, start);
        let b = s.as_bytes();
        if b.get(i) != Some(&b'"') {
            return Err(format!("expected string at byte {}", i));
        }
        let content_start = i + 1;
        i = content_start;
        let mut decoded: Option<String> = None;

        while i < b.len() {
            match b[i] {
                b'"' => {
                    let end_after = i + 1;
                    let arc_sub = match decoded {
                        None => ArcSubStr::new(Arc::clone(inner), content_start..i),
                        Some(s_out) => {
                            let len = s_out.len();
                            ArcSubStr::new(Arc::from(s_out), 0..len)
                        }
                    };
                    return Ok((arc_sub, end_after));
                }
                b'\\' => {
                    if decoded.is_none() {
                        decoded = Some(s[content_start..i].to_string());
                    }
                    let out = decoded.as_mut().expect("linked_sql_bug: set above");
                    i += 1;
                    match b.get(i) {
                        Some(&ch) if ch == b'"' || ch == b'\\' => {
                            out.push(ch as char);
                            i += 1;
                        }
                        Some(_) => return Err(format!("unsupported escape at {}", i)),
                        None => return Err("terminated inside escape".into()),
                    }
                }
                _ => {
                    let ch = s[i..]
                        .chars()
                        .next()
                        .ok_or_else(|| "eof in string".to_string())?;
                    let step = ch.len_utf8();
                    if let Some(out) = decoded.as_mut() {
                        out.push(ch);
                    }
                    i += step;
                }
            }
        }
        Err("unterminated string".into())
    }

    fn scan_number_end(s: &str, start: usize) -> usize {
        let b = s.as_bytes();
        let mut i = start;
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E' => i += 1,
                _ => break,
            }
        }
        i
    }

    fn deserialize_json_number_str(serialized: &mut JsonAsArcCursor) -> Result<&str, String> {
        let s = serialized.inner.as_ref();
        let i0 = skip_ws_json(s, serialized.start);
        if i0 >= s.len() {
            return Err("unexpected end where number expected".into());
        }
        let b = s.as_bytes();
        match b[i0] {
            b'0'..=b'9' | b'-' | b'+' => {}
            _ => return Err(format!("expected number, found {:?}", b[i0] as char)),
        }
        let i1 = scan_number_end(s, i0);
        if i1 <= i0 {
            return Err("invalid number".into());
        }
        let slice = &s[i0..i1];
        serialized.start = i1;
        Ok(slice)
    }

    fn scan_balanced(s: &str, start: usize, open: u8, close: u8) -> Result<usize, String> {
        let b = s.as_bytes();
        if b.get(start) != Some(&open) {
            return Err(format!("expected {:?} at {}", open as char, start));
        }
        let mut depth = 1usize;
        let mut i = start + 1;
        while i < b.len() && depth > 0 {
            match b[i] {
                b'"' => {
                    let (_, end) = parse_json_string(s, i)?;
                    i = end;
                }
                o if o == open => {
                    depth += 1;
                    i += 1;
                }
                c if c == close => {
                    depth -= 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }
        if depth != 0 {
            return Err("unbalanced brackets".into());
        }
        Ok(i)
    }

    fn end_of_json_value(s: &str, start: usize) -> Result<usize, String> {
        let start = skip_ws_json(s, start);
        let b = s.as_bytes();
        if start >= b.len() {
            return Err("unexpected end of input where value expected".into());
        }
        match b[start] {
            b'"' => parse_json_string(s, start).map(|(_, end)| end),
            x if x == OPEN_BRACE_BYTE => scan_balanced(s, start, OPEN_BRACE_BYTE, CLOSE_BRACE_BYTE),
            b'[' => scan_balanced(s, start, b'[', b']'),
            b't' => {
                if s.get(start..start + 4) == Some("true") {
                    Ok(start + 4)
                } else {
                    Err(format!("expected 'true' at {}", start))
                }
            }
            b'f' => {
                if s.get(start..start + 5) == Some("false") {
                    Ok(start + 5)
                } else {
                    Err(format!("expected 'false' at {}", start))
                }
            }
            b'n' => {
                if s.get(start..start + 4) == Some("null") {
                    Ok(start + 4)
                } else {
                    Err(format!("expected 'null' at {}", start))
                }
            }
            b'0'..=b'9' | b'-' | b'+' => Ok(scan_number_end(s, start)),
            _ => Err(format!(
                "unexpected byte at start of value: {:?}",
                b[start] as char
            )),
        }
    }

    fn parse_json_object_at(s: &str, start: usize) -> Result<(JsonMapAccess, usize), String> {
        let mut i = skip_ws_json(s, start);
        let b = s.as_bytes();
        if b.get(i) != Some(&OPEN_BRACE_BYTE) {
            return Err(format!("expected '{{' at byte {}", i));
        }
        i += 1;
        i = skip_ws_json(s, i);
        let mut entries = Vec::new();
        if b.get(i) == Some(&CLOSE_BRACE_BYTE) {
            let after_object = i + 1;
            return Ok((
                JsonMapAccess {
                    entries,
                    after_object,
                },
                after_object,
            ));
        }
        loop {
            if b.get(i) != Some(&b'"') {
                return Err(format!("expected '\"' for object key at {}", i));
            }
            let key_open = i;
            let (key_decoded, j) = parse_json_string(s, i)?;
            let key_range = key_open..j;
            i = skip_ws_json(s, j);
            if b.get(i) != Some(&b':') {
                return Err(format!("expected ':' after object key at {}", i));
            }
            i += 1;
            i = skip_ws_json(s, i);
            let v_start = i;
            let v_end = end_of_json_value(s, v_start)?;
            entries.push((Arc::<str>::from(key_decoded), key_range, v_start..v_end));
            i = v_end;
            i = skip_ws_json(s, i);
            match b.get(i) {
                Some(b',') => {
                    i += 1;
                    i = skip_ws_json(s, i);
                    if b.get(i) == Some(&CLOSE_BRACE_BYTE) {
                        let after_object = i + 1;
                        return Ok((
                            JsonMapAccess {
                                entries,
                                after_object,
                            },
                            after_object,
                        ));
                    }
                }
                Some(&CLOSE_BRACE_BYTE) => {
                    let after_object = i + 1;
                    return Ok((
                        JsonMapAccess {
                            entries,
                            after_object,
                        },
                        after_object,
                    ));
                }
                _ => {
                    return Err(format!("expected comma or '}}' after value at byte {}", i));
                }
            }
        }
    }

    impl KnownKeyInfo for JsonAsArcCursor {
        type Info<'a> = &'a str;
    }

    impl KnownKey<&'static str> for JsonAsArcCursor {
        fn info<'k>(key: &'k &'static str) -> Self::Info<'k> {
            *key
        }
    }

    impl KnownKey<String> for JsonAsArcCursor {
        fn info<'k>(key: &'k String) -> Self::Info<'k> {
            key.as_str()
        }
    }

    impl UnknownKey<JsonAsArcCursor> for String {}

    impl UnknownKey<JsonAsArcCursor> for crate::sub_arc::ArcSubStr {}

    impl<'de> DeserializeMap<'de> for JsonAsArcCursor {
        type MapAccess = JsonMapAccess;

        fn start_map(&mut self) -> Result<Self::MapAccess, Self::Err> {
            let s = self.inner.as_ref();
            let (map_access, _after_obj) = parse_json_object_at(s, self.start)?;
            Ok(map_access)
        }

        fn map_has_next(&self, map: &Self::MapAccess) -> bool {
            map.entries.len() > 0
        }

        fn deserialize_with_unknown_key<Key, Value>(
            &mut self,
            map: &mut Self::MapAccess,
            key_handler: Key::Handler,
            value_handler: Value::Handler,
        ) -> Result<(Key, Value), Self::Err>
        where
            Key: UnknownKey<Self> + DeserializeSpec + Deserialize<'de, Self>,
            Value: DeserializeSpec + Deserialize<'de, Self>,
        {
            if map.entries.is_empty() {
                return Err("no object entries left to deserialize".into());
            }
            let (_k_arc, key_range, value_range) = map.entries.swap_remove(0);
            self.start = key_range.start;
            let key = Key::deserialize(key_handler, self)?;
            if self.start != key_range.end {
                return Err(format!(
                    "key JSON did not parse to expected span (got {}, expected {})",
                    self.start, key_range.end
                ));
            }
            self.start = value_range.start;
            let value = Value::deserialize(value_handler, self)?;
            if self.start != value_range.end {
                return Err(format!(
                    "value JSON did not parse to expected span (got {}, expected {})",
                    self.start, value_range.end
                ));
            }
            Ok((key, value))
        }

        fn deserialize_with_known_key<Key, Value>(
            &mut self,
            map: &mut Self::MapAccess,
            key: Key,
            value_handler: Value::Handler,
        ) -> Result<Value, Self::Err>
        where
            Self: KnownKey<Key>,
            Value: DeserializeSpec + Deserialize<'de, Self>,
        {
            let needle = <Self as KnownKey<Key>>::info(&key);
            let idx = map
                .entries
                .iter()
                .position(|(k, _, _)| k.as_ref() == needle)
                .ok_or_else(|| format!("missing key {:?}", needle))?;
            let (_k_arc, _key_range, value_range) = map.entries.swap_remove(idx);
            self.start = value_range.start;
            let value = Value::deserialize(value_handler, self)?;
            if self.start != value_range.end {
                return Err(format!(
                    "value for key {:?} did not parse to expected span (got end {}, expected {})",
                    needle, self.start, value_range.end
                ));
            }
            Ok(value)
        }

        fn finish(&mut self, map: Self::MapAccess) -> Result<(), Self::Err> {
            self.start = map.after_object;
            Ok(())
        }
    }

    const OPEN_BRACKET: char = '[';
    const CLOSE_BRACKET: char = ']';
    const OPEN_BRACKET_BYTE: u8 = OPEN_BRACKET as u8;

    impl<'de> DeserializeMapOrSeq<'de> for JsonAsArcCursor {
        type SeqAccess = ();

        fn start_map_or_seq(
            &mut self,
        ) -> Result<MapOrSeq<Self::MapAccess, Self::SeqAccess>, Self::Err> {
            let s = self.inner.as_ref();
            let i = skip_ws_json(s, self.start);
            match s.as_bytes().get(i) {
                Some(&OPEN_BRACE_BYTE) => {
                    self.start = i;
                    DeserializeMap::start_map(self).map(MapOrSeq::Map)
                }
                Some(ch) if *ch == OPEN_BRACKET_BYTE => {
                    self.start = i;
                    DeserializeSeq::start_seq(self).map(MapOrSeq::Seq)
                }
                Some(b) => Err(format!(
                    "expected JSON object or array, found byte {:?} at {}",
                    *b as char, i
                )),
                None => Err(format!(
                    "expected JSON object or array, found end of input at offset {}",
                    i
                )),
            }
        }

        fn deserialize_value<T>(
            &mut self,
            seq: &mut Self::SeqAccess,
            handler: T::Handler,
        ) -> Result<T, Self::Err>
        where
            T: DeserializeSpec + Deserialize<'de, Self>,
        {
            DeserializeSeq::deserialize_value(self, seq, handler)
        }

        fn finish_map(&mut self, map: Self::MapAccess) -> Result<(), Self::Err> {
            DeserializeMap::finish(self, map)
        }

        fn finish_seq(&mut self, seq: Self::SeqAccess) -> Result<(), Self::Err> {
            DeserializeSeq::finish(self, seq)
        }
    }

    impl<'de> DeserializeSeq<'de> for JsonAsArcCursor {
        type SeqAccess = ();

        fn start_seq(&mut self) -> Result<Self::SeqAccess, Self::Err> {
            let mut chars = self.inner[self.start..].chars().into_iter();

            let mut peek = chars.next();
            while let Some(next) = peek {
                self.start += 1;
                if !next.is_whitespace() {
                    break;
                }
                peek = chars.next();
            }

            match peek {
                Some(OPEN_BRACKET) => {
                    // Opening `[` was already counted in the loop above; do not advance again.
                }
                _ => {
                    return Err(format!("expected start of a sequence found {:?}", peek));
                }
            }
            Ok(())
        }

        fn deserialize_value<T>(
            &mut self,
            _: &mut Self::SeqAccess,
            handler: T::Handler,
        ) -> Result<T, Self::Err>
        where
            T: DeserializeSpec + Deserialize<'de, Self>,
        {
            let mut chars = self.inner[self.start..].chars().into_iter();

            while let Some(next) = chars.next() {
                if !next.is_whitespace() {
                    break;
                }
                // self.start += 1;
            }

            let value = T::deserialize(handler, self)?;
            // check for comma

            let mut chars = self.inner[self.start..].chars().into_iter();
            let mut inc = 0;
            while let Some(next) = chars.next() {
                inc += 1;
                if next == ',' {
                    self.start += inc;
                    break;
                }
                if next.is_whitespace() {
                    continue;
                }
                break; // without incrementing
            }

            Ok(value)
        }

        fn seq_has_next(&mut self, _: &mut Self::SeqAccess) -> Result<bool, Self::Err> {
            let s = self.inner.as_ref();
            let i = skip_ws_json(s, self.start);
            Ok(s.as_bytes().get(i) != Some(&CLOSE_BRACKET_BYTE))
        }

        fn finish(&mut self, _: Self::SeqAccess) -> Result<(), Self::Err> {
            loop {
                let mut chars = self.inner[self.start..].chars();
                let Some(next) = chars.next() else {
                    return Err(format!("expected {} found end of input", CLOSE_BRACKET));
                };
                if next.is_whitespace() {
                    self.start += next.len_utf8();
                    continue;
                }
                if next == CLOSE_BRACKET {
                    self.start += next.len_utf8();
                    return Ok(());
                }
                return Err(format!("expected {} found {:?}", CLOSE_BRACKET, next));
            }
        }
    }

    const CLOSE_BRACKET_BYTE: u8 = CLOSE_BRACKET as u8;

    const PRETTY_INDENT: &str = "  ";

    fn write_pretty_value(
        out: &mut String,
        s: &str,
        start: usize,
        depth: usize,
    ) -> Result<usize, String> {
        let start = skip_ws_json(s, start);
        let b = s.as_bytes();
        if start >= b.len() {
            return Err("unexpected end of input where value expected".into());
        }
        match b[start] {
            OPEN_BRACE_BYTE => write_pretty_object(out, s, start, depth),
            b'[' => write_pretty_array(out, s, start, depth),
            _ => {
                let end = end_of_json_value(s, start)?;
                out.push_str(&s[start..end]);
                Ok(end)
            }
        }
    }

    fn write_pretty_object(
        out: &mut String,
        s: &str,
        start: usize,
        depth: usize,
    ) -> Result<usize, String> {
        let (map, after) = parse_json_object_at(s, start)?;
        out.push('{');
        if map.entries.is_empty() {
            out.push('}');
            return Ok(after);
        }

        out.push('\n');
        for (idx, (_, key_range, value_range)) in map.entries.iter().enumerate() {
            out.push_str(&PRETTY_INDENT.repeat(depth + 1));
            out.push_str(&s[key_range.start..key_range.end]);
            out.push(':');

            let value_start = skip_ws_json(s, value_range.start);
            let is_complex = matches!(
                s.as_bytes().get(value_start),
                Some(&OPEN_BRACE_BYTE) | Some(b'[')
            );
            if is_complex {
                out.push('\n');
                out.push_str(&PRETTY_INDENT.repeat(depth + 1));
            } else {
                out.push(' ');
            }

            write_pretty_value(out, s, value_range.start, depth + 1)?;
            if idx + 1 < map.entries.len() {
                out.push(',');
            }
            out.push('\n');
        }

        out.push_str(&PRETTY_INDENT.repeat(depth));
        out.push('}');
        Ok(after)
    }

    fn write_pretty_array(
        out: &mut String,
        s: &str,
        start: usize,
        depth: usize,
    ) -> Result<usize, String> {
        let mut i = skip_ws_json(s, start);
        if s.as_bytes().get(i) != Some(&b'[') {
            return Err(format!("expected '[' at byte {i}"));
        }
        i += 1;
        i = skip_ws_json(s, i);

        out.push('[');
        if s.as_bytes().get(i) == Some(&b']') {
            out.push(']');
            return Ok(i + 1);
        }

        out.push('\n');
        loop {
            out.push_str(&PRETTY_INDENT.repeat(depth + 1));
            i = write_pretty_value(out, s, i, depth + 1)?;
            i = skip_ws_json(s, i);
            match s.as_bytes().get(i) {
                Some(b',') => {
                    out.push(',');
                    out.push('\n');
                    i += 1;
                }
                Some(b']') => {
                    out.push('\n');
                    out.push_str(&PRETTY_INDENT.repeat(depth));
                    out.push(']');
                    return Ok(i + 1);
                }
                _ => {
                    return Err(format!(
                        "expected comma or ']' after array value at byte {i}"
                    ));
                }
            }
        }
    }

    pub(crate) fn format_json_pretty(input: &str) -> Result<String, String> {
        let start = skip_ws_json(input, 0);
        let mut out = String::new();
        write_pretty_value(&mut out, input, start, 0)?;
        Ok(out)
    }
}

/// Deserialize `Into` from `from` using `handler`, then verify no trailing input remains.
pub fn deserialize<'de, Format, From, Into>(
    from: From,
    handler: <Into as DeserializeSpec>::Handler,
    _format: Format,
) -> Result<Into, <From::Deserializer as Deserializer<'de>>::Err>
where
    From: FromSerialized<'de, Format>,
    Into: DeserializeSpec + Deserialize<'de, From::Deserializer>,
{
    let mut serialized = from.start();
    let value = Into::deserialize(handler, &mut serialized)?;
    From::terminate(serialized)?;
    Ok(value)
}

/// Pretty-print JSON for readable test diffs.
pub fn pretty_json(input: &str) -> String {
    use std::sync::Arc;

    let partial: json_format_side::PartialDeserialize = deserialize(
        Arc::from(input.to_string()),
        (),
        json_format_side::JsonFormat,
    )
    .unwrap_or_else(|err| {
        panic!("expected valid JSON in test assertion: {err}: {input:?}");
    });

    json_format_side::format_json_pretty(partial.0.as_str()).unwrap_or_else(|err| {
        panic!("failed to pretty-print JSON: {err}: {input:?}");
    })
}

pub trait IntoSerializedString<Format> {
    fn serialize_to_string(&self) -> String;
}

pub trait IntoSerializedBuffer<Forma> {
    fn serialize_to_buffer(&self) -> Vec<u8>;
}

impl<F, T> IntoSerializedString<F> for T
where
    T: Serialize<F>,
    F: Default + Into<String>,
{
    fn serialize_to_string(&self) -> String {
        let mut string = F::default();

        self.serialize(&mut string);

        string.into()
    }
}

impl<F, T> IntoSerializedBuffer<F> for T
where
    T: Serialize<F>,
    F: Default + Into<Vec<u8>>,
{
    fn serialize_to_buffer(&self) -> Vec<u8> {
        let mut buffer = F::default();

        self.serialize(&mut buffer);

        buffer.into()
    }
}

/// Link / partial row payload as a finished JSON text blob (see [`JsonAsString`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedJson(pub std::sync::Arc<str>);

impl SerializedJson {
    pub fn new<T>(value: &T) -> Self
    where
        T: Serialize<json_serialize_side::JsonAsString>,
    {
        Self(std::sync::Arc::from(IntoSerializedString::<
            json_serialize_side::JsonAsString,
        >::serialize_to_string(value)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize<json_serialize_side::JsonAsString> for SerializedJson {
    fn serialize(&self, ctx: &mut json_serialize_side::JsonAsString) {
        ctx.0.push_str(self.as_str());
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SerializedJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_json::from_str::<serde_json::Value>(self.as_str())
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
