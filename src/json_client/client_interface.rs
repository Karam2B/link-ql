use std::collections::BTreeMap;

use crate::gen_serde::Serialize;
use crate::gen_serde::json_format_side::PartialDeserialize;
use crate::gen_serde::json_serialize_side::JsonAsString;
use crate::json_client::dynamic_collection::CollectionToSerialize;
use crate::operations::{CollectionOutput, LinkedOutput};
use crate::sub_arc::ArcSubStr;

//*******************
//*
//* SupportedType
//*
//*******************
#[derive(Debug)]
pub enum SupportedType {
    String,
    Boolean,
    Int,
    Float64,
    Array(Box<SupportedType>),
}

//*******************
//*
//* SupportedFilter
//*
//*******************
#[derive(Debug)]
pub enum SupportedFilter {
    ColEq {
        col: ArcSubStr,
        eq: PartialDeserialize,
    },
    ColNe {
        col: ArcSubStr,
        ne: PartialDeserialize,
    },
    ColGt {
        col: ArcSubStr,
        gt: PartialDeserialize,
    },
    ColGte {
        col: ArcSubStr,
        gte: PartialDeserialize,
    },
    ColLt {
        col: ArcSubStr,
        lt: PartialDeserialize,
    },
    ColLte {
        col: ArcSubStr,
        lte: PartialDeserialize,
    },
    ColContains {
        col: ArcSubStr,
        value: PartialDeserialize,
    },
    ColIsNull {
        col: ArcSubStr,
    },
    ColIsNotNull {
        col: ArcSubStr,
    },
    And {
        filters: Vec<SupportedFilter>,
    },
    Or {
        filters: Vec<SupportedFilter>,
    },
}

//*******************
//*
//* AddCollection
//*
//*******************
#[derive(Debug)]
pub struct AddCollectionInput {
    pub name: ArcSubStr,
    pub fields: Vec<DynamicFieldInput>,
}

#[derive(Debug)]
pub struct DynamicFieldInput {
    pub name: ArcSubStr,
    pub type_info: SupportedType,
    pub is_optional: bool,
}

pub type AddCollectionOutput = ();

#[derive(Debug)]
pub enum AddCollectionError {
    CollectionAlreadyExists,
    InvalidCollectionInput,
}

//*******************
//*
//* AddLink
//*
//*******************
#[derive(Debug)]
pub enum AddLinkInput {
    OneToMany { from: ArcSubStr, to: ArcSubStr },
    ManyToMany { from: ArcSubStr, to: ArcSubStr },
    Timestamp { collection: ArcSubStr },
}

pub type AddLinkOutput = ();

#[derive(Debug)]
pub enum AddLinkError {
    LinkAlreadyExists,
    CollectionNotFound,
}

//*******************
//*
//* InsertOne
//*
//*******************
pub use crate::gen_serde::SerializedJson;
pub use crate::operations::fetch_many::ManyOutput;

#[derive(Debug)]
pub enum SupportedInsertLink {
    SetId {
        to: ArcSubStr,
        id: i64,
    },
    SetNew {
        to: ArcSubStr,
        value: PartialDeserialize,
    },
}

#[derive(Debug)]
pub struct InsertOneInput {
    pub base: ArcSubStr,
    pub data: PartialDeserialize,
    pub links: Vec<SupportedInsertLink>,
}

pub type InsertOneOutput =
    LinkedOutput<i64, CollectionToSerialize, Vec<Box<dyn Serialize<JsonAsString> + Send>>>;

#[derive(Debug)]
pub enum InsertOneError {
    CollectionNotFound,
    InvalidData,
    InvalidLink,
    LinkNotSetUpForThisBase,
}

//*******************
//*
//* InsertMany
//*
//*******************
#[derive(Debug)]
pub struct InsertManyItem {
    pub data: PartialDeserialize,
    pub links: Vec<SupportedInsertLink>,
}

#[derive(Debug)]
pub struct InsertManyInput {
    pub base: ArcSubStr,
    pub items: Vec<InsertManyItem>,
}

#[derive(Debug)]
pub struct InsertManyOutput {
    pub items: Vec<InsertOneOutput>,
}

#[derive(Debug)]
pub enum InsertManyError {
    CollectionNotFound,
    InvalidData,
    InvalidLink,
    LinkNotSetUpForThisBase,
}

//*******************
//*
//* FetchMany
//*
//*******************
#[derive(Debug)]
pub enum SupportedLinkFetchMany {
    OneToMany { to: ArcSubStr },
    ManyToMany { to: ArcSubStr },
    Timestamp,
}

#[derive(Debug)]
pub struct FetchManyInput {
    pub base: ArcSubStr,
    pub filters: Vec<SupportedFilter>,
    pub links: Vec<SupportedLinkFetchMany>,
    pub pagination: Pagination,
}

#[derive(Debug)]
pub struct Pagination {
    pub limit: i64,
    pub first_item: Option<FirstItem>,
    pub order_by: Vec<OrderBy>,
}

#[derive(Debug)]
pub struct FirstItem {
    pub id: i64,
    pub data: BTreeMap<ArcSubStr, PartialDeserialize>,
}

#[derive(Debug)]
pub struct OrderBy {
    pub col: ArcSubStr,
    pub direction: Direction,
}

#[derive(Debug)]
pub enum Direction {
    Asc,
    Desc,
}

pub type FetchManyItem =
    LinkedOutput<i64, CollectionToSerialize, Vec<Box<dyn Serialize<JsonAsString> + Send>>>;

pub type FetchManyOutput = ManyOutput<FetchManyItem, CollectionOutput<i64, CollectionToSerialize>>;

#[derive(Debug)]
pub enum FetchManyError {
    CollectionNotFound,
    InvalidData,
    LinkNotSetUpForThisBase,
    InvalidFilter,
    InvalidLink,
    InvalidOrderBy,
    InvalidFirstItem,
}

//*******************
//*
//* FetchOne
//*
//*******************
#[derive(Debug)]
pub enum SupportedLinkFetchOne {
    OneToMany { to: ArcSubStr },
    ManyToMany { to: ArcSubStr },
    Timestamp,
}

#[derive(Debug)]
pub struct FetchOneInput {
    pub base: ArcSubStr,
    pub id: i64,
    pub filters: Vec<SupportedFilter>,
    pub links: Vec<SupportedLinkFetchOne>,
}

pub type FetchOneOutput =
    LinkedOutput<i64, CollectionToSerialize, Vec<Box<dyn Serialize<JsonAsString> + Send>>>;

#[derive(Debug)]
pub enum FetchOneError {
    CollectionNotFound,
    NotFound,
    InvalidFilter,
    InvalidLink,
}

//*******************
//*
//* UpdateOne
//*
//*******************
#[derive(Debug)]
pub enum SupportedUpdateLink {
    SetId {
        to: ArcSubStr,
        id: i64,
    },
    SetNew {
        to: ArcSubStr,
        value: PartialDeserialize,
    },
    SetNull {
        to: ArcSubStr,
    },
    RemoveId {
        to: ArcSubStr,
        id: i64,
    },
}

#[derive(Debug)]
pub struct UpdateOneInput {
    pub base: ArcSubStr,
    pub id: i64,
    pub data: PartialDeserialize,
    pub links: Vec<SupportedUpdateLink>,
}

pub type UpdateOneOutput =
    LinkedOutput<i64, CollectionToSerialize, Vec<Box<dyn Serialize<JsonAsString> + Send>>>;

#[derive(Debug)]
pub enum UpdateOneError {
    CollectionNotFound,
    InvalidData,
    NotFound,
    InvalidLink,
}

//*******************
//*
//* DeleteOne
//*
//*******************
#[derive(Debug)]
pub enum SupportedDeleteLink {
    OneToMany { to: ArcSubStr },
    ManyToMany { to: ArcSubStr },
}

#[derive(Debug)]
pub struct DeleteOneInput {
    pub base: ArcSubStr,
    pub id: i64,
    pub links: Vec<SupportedDeleteLink>,
}

pub type DeleteOneOutput =
    LinkedOutput<i64, CollectionToSerialize, Vec<Box<dyn Serialize<JsonAsString> + Send>>>;

#[derive(Debug)]
pub enum DeleteOneError {
    CollectionNotFound,
    NotFound,
    InvalidLink,
}

//*******************
//*
//* Client
//*
//*******************
pub use super::ops::OperationError as ClientOperationError;
pub use super::ops::OperationInput as ClientOperationInput;
pub use super::ops::OperationOutput as ClientOperationOutput;

pub struct Client {
    pub(crate) sender: tokio::sync::mpsc::UnboundedSender<(
        ClientOperationInput,
        oneshot::Sender<Result<ClientOperationOutput, ClientOperationError>>,
    )>,
}
