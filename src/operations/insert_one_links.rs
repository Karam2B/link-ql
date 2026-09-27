//! Heterogeneous insert links: `Vec<Box<dyn …>>` → one [`InsertOneLink`] (dynamic client).

use std::{marker::PhantomData, sync::Arc};

use sqlx::Sqlite;

use crate::{
    extentions::common_expressions::Aliased,
    from_row::{FromRowAlias, FromRowData},
    gen_serde::SerializedJson,
    json_client::dynamic_collection::DynamicCollection,
    links::{
        DefaultRelationKey,
        relation_one_to_many::OneToMany,
        timestamp::{DatedSet, TimestampOutput, impl_fetch_many::TimestampSelectItems},
        update_links::SetId,
    },
    operations::{
        OperationOutput,
        insert::{ConstraintViolation, InsertLinkConsumeData, InsertLinkData, InsertOneLink},
    },
    sqlx_query_builder::Join,
};

type DynCollection = Arc<DynamicCollection<Sqlite>>;
type DynOneToMany = OneToMany<DefaultRelationKey, DynCollection, DynCollection>;
type DynSetCategoryLink = SetId<DynOneToMany, PhantomData<i64>>;
type DynTimestampLink = DatedSet<DynCollection>;

/// `Vec<Box<dyn InsertLinkConsumeErased>>` — category [`SetId`] + timestamp [`DatedSet`] on base.
pub struct InsertLinks(pub Vec<Box<dyn InsertLinkConsumeErased + Send>>);

/// [`SetId`] / [`DatedSet`] convert into consumed link state.
pub trait InsertLinkConsumeErased: Send {
    fn into_dyn_consumed(self: Box<Self>) -> DynConsumedInsertLink;
}

pub enum DynConsumedInsertLink {
    SetCategory {
        link: DynSetCategoryLink,
        insert_value_data: i64,
    },
    Timestamp {
        link: DynTimestampLink,
    },
}

pub struct InsertLinksPair {
    set_category: DynSetCategoryLink,
    category_id: i64,
    timestamp: DynTimestampLink,
}

impl InsertLinkConsumeErased for SetId<DynOneToMany, i64> {
    fn into_dyn_consumed(self: Box<Self>) -> DynConsumedInsertLink {
        let (link, data) = InsertLinkConsumeData::consume_data(*self);
        DynConsumedInsertLink::SetCategory {
            link,
            insert_value_data: data.insert_value_data,
        }
    }
}

impl InsertLinkConsumeData for DatedSet<DynCollection> {
    type Link = DatedSet<DynCollection>;

    fn consume_data(
        self,
    ) -> (
        Self::Link,
        InsertLinkData<
            <Self::Link as InsertOneLink>::PreOpData,
            <Self::Link as InsertOneLink>::InsertValuesData,
            <Self::Link as InsertOneLink>::PostOpData,
        >,
    ) {
        (
            self,
            InsertLinkData {
                pre_op_data: (),
                insert_value_data: (),
                post_op_data: (),
            },
        )
    }
}

impl InsertOneLink for DatedSet<DynCollection> {
    type PreOp = ();
    type PreOpData = ();
    fn pre_operation_init(&self, _: Self::PreOpData) -> Self::PreOp {}
    fn pre_op_split(
        &self,
        _: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        ConstraintViolation,
    > {
        Ok(((), (), ()))
    }
    type PreOpToInsertValue = ();
    type PreOpToTake = ();
    type PreOpToPostOp = ();
    type InsertNames = ();
    fn insert_names(&self) -> Self::InsertNames {}
    type InsertReturning = <TimestampSelectItems<std::sync::Arc<str>> as Aliased>::Aliased;
    fn insert_returning(&self) -> Self::InsertReturning {
        TimestampSelectItems(std::sync::Arc::clone(
            &self.collection.collection_name.pascal_case,
        ))
        .aliased("")
    }
    type InsertValuesData = ();
    type InsertValues = ();
    fn insert_value(&self, _: Self::InsertValuesData, _: ()) -> Self::InsertValues {}
    type FromRow = TimestampSelectItems<std::sync::Arc<str>>;
    fn from_row(&self) -> Self::FromRow {
        TimestampSelectItems(std::sync::Arc::clone(
            &self.collection.collection_name.pascal_case,
        ))
    }
    type TakeInput = TimestampOutput;
    type PostOp = ();
    type PostOpData = ();
    fn from_row_result(
        &self,
        _: Self::PostOpData,
        from_row: TimestampOutput,
        _: Self::PreOpToPostOp,
    ) -> (Self::PostOp, Self::TakeInput) {
        ((), from_row)
    }
    type PostOpOutput = ();
    fn post_op_output(
        &self,
        _: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        Ok(())
    }
    type Output = TimestampOutput;
    fn take(
        self,
        _: Self::PostOpOutput,
        timestamps: Self::TakeInput,
        _: Self::PreOpToTake,
    ) -> Self::Output {
        timestamps
    }
}

impl InsertLinkConsumeErased for DatedSet<DynCollection> {
    fn into_dyn_consumed(self: Box<Self>) -> DynConsumedInsertLink {
        let (link, _) = InsertLinkConsumeData::consume_data(*self);
        DynConsumedInsertLink::Timestamp { link }
    }
}

impl InsertLinkConsumeData for InsertLinks {
    type Link = InsertLinksPair;

    fn consume_data(
        self,
    ) -> (
        Self::Link,
        InsertLinkData<
            <Self::Link as InsertOneLink>::PreOpData,
            <Self::Link as InsertOneLink>::InsertValuesData,
            <Self::Link as InsertOneLink>::PostOpData,
        >,
    ) {
        let mut links = self.0.into_iter();
        let set = links
            .next()
            .expect("insert links: expected SetId")
            .into_dyn_consumed();
        let timestamp = links
            .next()
            .expect("insert links: expected timestamp DatedSet")
            .into_dyn_consumed();
        if links.next().is_some() {
            panic!("insert links: only SetId + timestamp are supported");
        }

        let (set_category, category_id) = match set {
            DynConsumedInsertLink::SetCategory {
                link,
                insert_value_data,
            } => (link, insert_value_data),
            _ => panic!("insert links: first link must be SetId"),
        };
        let timestamp = match timestamp {
            DynConsumedInsertLink::Timestamp { link } => link,
            _ => panic!("insert links: second link must be timestamp"),
        };

        (
            InsertLinksPair {
                set_category,
                category_id,
                timestamp,
            },
            InsertLinkData {
                pre_op_data: (),
                insert_value_data: (),
                post_op_data: (),
            },
        )
    }
}

impl InsertOneLink for InsertLinksPair {
    type PreOp = ();
    type PreOpData = ();
    fn pre_operation_init(&self, _: Self::PreOpData) -> Self::PreOp {}
    fn pre_op_split(
        &self,
        _: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        ConstraintViolation,
    > {
        Ok(((), (), ()))
    }
    type PreOpToInsertValue = ();
    type PreOpToTake = ();
    type PreOpToPostOp = ();

    type InsertNames = (
        <DynSetCategoryLink as InsertOneLink>::InsertNames,
        <DynTimestampLink as InsertOneLink>::InsertNames,
    );
    fn insert_names(&self) -> Self::InsertNames {
        (
            self.set_category.insert_names(),
            self.timestamp.insert_names(),
        )
    }

    type InsertReturning = (
        <DynSetCategoryLink as InsertOneLink>::InsertReturning,
        <DynTimestampLink as InsertOneLink>::InsertReturning,
    );
    fn insert_returning(&self) -> Self::InsertReturning {
        (
            self.set_category.insert_returning(),
            self.timestamp.insert_returning(),
        )
    }

    type InsertValuesData = ();
    type InsertValues = (
        <DynSetCategoryLink as InsertOneLink>::InsertValues,
        <DynTimestampLink as InsertOneLink>::InsertValues,
    );
    fn insert_value(
        &self,
        _: Self::InsertValuesData,
        _: Self::PreOpToInsertValue,
    ) -> Self::InsertValues {
        (
            self.set_category.insert_value(self.category_id, ()),
            self.timestamp.insert_value((), ()),
        )
    }

    type FromRow = PairFromRow;
    fn from_row(&self) -> Self::FromRow {
        PairFromRow {
            set: self.set_category.from_row(),
            timestamp: self.timestamp.from_row(),
        }
    }

    type TakeInput = TimestampOutput;
    type PostOp = <DynSetCategoryLink as InsertOneLink>::PostOp;
    type PostOpData = ();

    fn from_row_result(
        &self,
        _: Self::PostOpData,
        from_row: <Self::FromRow as FromRowData>::RData,
        _: Self::PreOpToPostOp,
    ) -> (Self::PostOp, Self::TakeInput) {
        let (post_op, _) = self
            .set_category
            .from_row_result((), from_row.category_fk, ());
        let timestamps = self.timestamp.clone().take((), from_row.timestamp, ());
        (post_op, timestamps)
    }

    type PostOpOutput = <DynSetCategoryLink as InsertOneLink>::PostOpOutput;

    fn post_op_output(
        &self,
        post_op: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        self.set_category.post_op_output(post_op)
    }

    type Output = Vec<SerializedJson>;

    fn take(
        self,
        category: Self::PostOpOutput,
        timestamp: Self::TakeInput,
        pre_op_to_take: Self::PreOpToTake,
    ) -> Self::Output {
        let category = self.set_category.take(category, (), pre_op_to_take);
        vec![
            SerializedJson::new(&category),
            SerializedJson::new(&timestamp),
        ]
    }
}

pub struct PairFromRow {
    set: <DynSetCategoryLink as InsertOneLink>::FromRow,
    timestamp: <DynTimestampLink as InsertOneLink>::FromRow,
}

pub struct PairFromRowData {
    category_fk: i64,
    timestamp: TimestampOutput,
}

impl FromRowData for PairFromRow {
    type RData = PairFromRowData;
}

impl<'r> FromRowAlias<'r, <Sqlite as sqlx::Database>::Row> for PairFromRow {
    fn no_alias(
        &self,
        row: &'r <Sqlite as sqlx::Database>::Row,
    ) -> Result<Self::RData, crate::from_row::FromRowError> {
        Ok(PairFromRowData {
            category_fk: self.set.no_alias(row)?,
            timestamp: self.timestamp.no_alias(row)?,
        })
    }

    fn str_alias(
        &self,
        row: crate::from_row::RowStrAliased<'r, <Sqlite as sqlx::Database>::Row>,
    ) -> Result<Self::RData, crate::from_row::FromRowError>
    where
        <Sqlite as sqlx::Database>::Row: sqlx::Row,
    {
        Ok(PairFromRowData {
            category_fk: self.set.str_alias(row.clone())?,
            timestamp: self.timestamp.str_alias(row)?,
        })
    }

    fn num_alias(
        &self,
        row: crate::from_row::RowNumAliased<'r, <Sqlite as sqlx::Database>::Row>,
    ) -> Result<Self::RData, crate::from_row::FromRowError>
    where
        <Sqlite as sqlx::Database>::Row: sqlx::Row,
    {
        Ok(PairFromRowData {
            category_fk: self.set.num_alias(row.clone())?,
            timestamp: self.timestamp.num_alias(row)?,
        })
    }
}
