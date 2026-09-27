type TodoFix = std::convert::Infallible;
impl crate::operations::OperationOutput for TodoFix {
    type Output = TodoFix;
}
impl crate::from_row::FromRowData for TodoFix {
    type RData = TodoFix;
}
use std::any::Any;

use sqlx::Database;

use crate::{
    database_extention::DatabaseExt,
    fix_executor::ExecutorTrait,
    from_row::FromRowData,
    gen_serde::{Serialize, json_serialize_side::JsonAsString},
    json_client::compat::{
        insert_sets::{
            AggregatedInsertReturning, AggregatedInsertSets, DynIdentifierOnly,
            DynSelfPrescribedInsert,
        },
        raw_from_row::RawFromRow,
    },
    operations::{
        Operation, OperationOutput,
        boxed_operation::BoxedOperation,
        insert::{ConstraintViolation, InsertLinkConsumeData, InsertLinkData, InsertOneLink},
        operations_expressions_crossover::{IdentifierOnly, SelfPrescribedInsert},
    },
    sqlx_query_builder::{
        Expression, Join,
        combinators::OptionalExpression,
        trait_objects::{box_expression, BoxedExpression},
    },
};

/// [`FromRowAlias`] that decodes one sub-row per link.
pub struct JsonInsertLinksFromRow<S>(pub Vec<Box<dyn RawFromRow<S> + Send>>);

impl<S: Database> FromRowData for JsonInsertLinksFromRow<S> {
    type RData = Vec<Box<dyn Any + Send>>;
}

impl<'r, S: Database> crate::from_row::FromRowAlias<'r, S::Row> for JsonInsertLinksFromRow<S> {
    fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, crate::from_row::FromRowError> {
        self.0
            .iter()
            .map(|from_row: &Box<dyn RawFromRow<S> + Send>| {
                RawFromRow::dyn_no_alias(&**from_row, row)
            })
            .collect()
    }

    fn str_alias(
        &self,
        row: crate::from_row::RowStrAliased<'r, S::Row>,
    ) -> Result<Self::RData, crate::from_row::FromRowError>
    where
        S::Row: sqlx::Row,
    {
        self.0
            .iter()
            .map(|from_row: &Box<dyn RawFromRow<S> + Send>| {
                RawFromRow::dyn_str_alias(&**from_row, row.clone())
            })
            .collect()
    }

    fn num_alias(
        &self,
        row: crate::from_row::RowNumAliased<'r, S::Row>,
    ) -> Result<Self::RData, crate::from_row::FromRowError>
    where
        S::Row: sqlx::Row,
    {
        self.0
            .iter()
            .map(|from_row: &Box<dyn RawFromRow<S> + Send>| {
                RawFromRow::dyn_num_alias(&**from_row, row.clone())
            })
            .collect()
    }
}

pub struct JsonInsertOneToConsume<S> {
    link: Box<dyn JsonInsertOneLink<S> + Send>,
    data: InsertLinkData<Box<dyn Any + Send>, Box<dyn Any + Send>, Box<dyn Any + Send>>,
}

impl<S> JsonInsertOneToConsume<S> {
    pub fn new<T>(pre_data: T) -> Self
    where
        S: sqlx::Database,
        T: InsertLinkConsumeData,
        T::Link: Send + 'static,
        <T::Link as InsertOneLink>::InsertValuesData: 'static + Send,
        <T::Link as InsertOneLink>::PreOpData: 'static + Send,
        <T::Link as InsertOneLink>::PostOpData: 'static + Send,
        T::Link: JsonInsertOneLink<S>,
    {
        let (link, data) = pre_data.consume_data();
        Self {
            link: Box::new(link),
            data: InsertLinkData {
                insert_value_data: Box::new(data.insert_value_data),
                pre_op_data: Box::new(data.pre_op_data),
                post_op_data: Box::new(data.post_op_data),
            },
        }
    }
}

pub trait JsonInsertOneLink<S: Database>: Send + Sync + 'static {
    fn dyn_pre_op_init(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send>;
    fn dyn_pre_op_split(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Result<
        (
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
        ),
        ConstraintViolation,
    >;
    fn dyn_insert_returning(&self) -> Box<dyn DynIdentifierOnly<S> + Send>;
    fn dyn_insert_value(
        &self,
        from_data: Box<dyn Any + Send>,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Box<dyn DynSelfPrescribedInsert<S> + Send>;
    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send>;
    fn dyn_from_row_result(
        &self,
        from_data: Box<dyn Any + Send>,
        from_row: Box<dyn Any + Send>,
        pre_op_to_post_op: Box<dyn Any + Send>,
    ) -> (Box<dyn BoxedOperation<S> + Send>, Box<dyn Any + Send>);
    fn dyn_post_op_output(
        &self,
        poo: Box<dyn Any + Send>,
    ) -> Result<Box<dyn Any + Send>, ConstraintViolation>;
    fn dyn_take(
        self: Box<Self>,
        post_op_output: Box<dyn Any + Send>,
        insert_items: Box<dyn Any + Send>,
        pre_op_to_post_op: Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send>;
}

impl<T, S> JsonInsertOneLink<S> for T
where
    T: Send + Sync + 'static,
    S: sqlx::Database + DatabaseExt,
    T: InsertOneLink,
    T::PreOpData: 'static,
    T::PreOp: Operation<S> + 'static,
    T::PreOpToInsertValue: Send + 'static,
    T::PreOpToTake: Send + 'static,
    T::PreOpToPostOp: Send + 'static,
    T::InsertReturning: Send + 'static + IdentifierOnly,
    <T::InsertReturning as IdentifierOnly>::Identifier: Send + OptionalExpression,
    for<'e> Join<<T::InsertReturning as IdentifierOnly>::Identifier>: Expression<'e, S>,
    T::InsertSets: Send + 'static + SelfPrescribedInsert,
    <T::InsertSets as SelfPrescribedInsert>::InsertId: Send + OptionalExpression,
    <T::InsertSets as SelfPrescribedInsert>::InsertValue: Send + OptionalExpression,
    for<'e> Join<<T::InsertSets as SelfPrescribedInsert>::InsertId>: Expression<'e, S>,
    for<'e> Join<<T::InsertSets as SelfPrescribedInsert>::InsertValue>: Expression<'e, S>,
    T::FromRow: Send + 'static + RawFromRow<S>,
    T::PostOpData: Send + 'static,
    T::PostOp: Operation<S> + Send + 'static,
    T::TakeInput: Send + 'static,
    T::PostOpOutput: Send + 'static,
    T::Output: Send + 'static + Serialize<JsonAsString>,
{
    fn dyn_pre_op_init(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send> {
        // if std::any::TypeId::of::<T::PreOpData>()
        //     == std::any::TypeId::of::<Box<dyn Any + Send>>()
        // {
        //     let wrapper = (self as &dyn Any)
        //         .downcast_ref::<Box<dyn JsonInsertOneLink<S> + Send>>()
        //         .expect("PreOpData is Box<dyn Any>; self is Box<dyn JsonInsertOneLink>");
        //     return wrapper.as_ref().dyn_pre_op_init(input);
        // }
        let downcased = input.downcast::<T::PreOpData>().unwrap();
        Box::new(self.pre_operation_init(*downcased))
    }
    fn dyn_pre_op_split(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Result<
        (
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
        ),
        ConstraintViolation,
    > {
        let downcasted_pre_op_output = pre_op_output
            .downcast::<<T::PreOp as OperationOutput>::Output>()
            .unwrap();

        let (insert_value, take, post_op) = self.pre_op_split(*downcasted_pre_op_output)?;
        Ok((Box::new(insert_value), Box::new(take), Box::new(post_op)))
    }
    fn dyn_insert_returning(&self) -> Box<dyn DynIdentifierOnly<S> + Send> {
        Box::new(self.insert_returning())
    }
    fn dyn_insert_value(
        &self,
        from_data: Box<dyn Any + Send>,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Box<dyn DynSelfPrescribedInsert<S> + Send> {
        let downcasted_from_data = from_data.downcast::<T::InsertValuesData>().unwrap();
        let downcasted_pre_op_output = pre_op_output.downcast::<T::PreOpToInsertValue>().unwrap();
        Box::new(self.insert_value(*downcasted_from_data, *downcasted_pre_op_output))
    }

    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send> {
        Box::new(self.from_row())
    }
    fn dyn_from_row_result(
        &self,
        from_data: Box<dyn Any + Send>,
        from_row: Box<dyn Any + Send>,
        pre_op_to_post_op: Box<dyn Any + Send>,
    ) -> (Box<dyn BoxedOperation<S> + Send>, Box<dyn Any + Send>) {
        let downcasted_from_data = from_data.downcast::<T::PostOpData>().unwrap();
        let downcasted_from_row = from_row
            .downcast::<<T::FromRow as FromRowData>::RData>()
            .unwrap();
        let downcasted_pre_op_to_post_op =
            pre_op_to_post_op.downcast::<T::PreOpToPostOp>().unwrap();
        let (post_op, take_input) = self.from_row_result(
            *downcasted_from_data,
            *downcasted_from_row,
            *downcasted_pre_op_to_post_op,
        );

        (Box::new(post_op), Box::new(take_input))
    }
    fn dyn_post_op_output(
        &self,
        poo: Box<dyn Any + Send>,
    ) -> Result<Box<dyn Any + Send>, ConstraintViolation> {
        let downcasted_poo = poo
            .downcast::<<T::PostOp as OperationOutput>::Output>()
            .unwrap();
        Ok(Box::new(self.post_op_output(*downcasted_poo)?))
    }
    fn dyn_take(
        self: Box<Self>,
        post_op_output: Box<dyn Any + Send>,
        insert_items: Box<dyn Any + Send>,
        pre_op_to_post_op: Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send> {
        let downcasted_post_op_output = post_op_output.downcast::<T::PostOpOutput>().unwrap();
        let downcasted_insert_items = insert_items.downcast::<T::TakeInput>().unwrap();
        let downcasted_pre_op_to_take = pre_op_to_post_op.downcast::<T::PreOpToTake>().unwrap();
        Box::new(self.take(
            *downcasted_post_op_output,
            *downcasted_insert_items,
            *downcasted_pre_op_to_take,
        ))
    }
}

impl<'b, S> InsertOneLink for Box<dyn JsonInsertOneLink<S> + Send + 'b>
where
    S: Database,
{
    type PreOp = Box<dyn BoxedOperation<S> + Send>;

    type PreOpData = Box<dyn Any + Send>;

    fn pre_operation_init(&self, input: Self::PreOpData) -> Self::PreOp {
        self.dyn_pre_op_init(input)
    }

    fn pre_op_split(
        &self,
        pre_op_output: <Self::PreOp as crate::operations::OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        crate::operations::insert::ConstraintViolation,
    > {
        self.dyn_pre_op_split(pre_op_output)
    }

    type PreOpToInsertValue = Box<dyn Any + Send>;

    type PreOpToTake = Box<dyn Any + Send>;

    type PreOpToPostOp = Box<dyn Any + Send>;

    type InsertReturning = Box<dyn DynIdentifierOnly<S> + Send>;

    fn insert_returning(&self) -> Self::InsertReturning {
        self.dyn_insert_returning()
    }

    type InsertValuesData = Box<dyn Any + Send>;

    type InsertSets = Box<dyn DynSelfPrescribedInsert<S> + Send>;

    fn insert_value(
        &self,
        from_data: Self::InsertValuesData,
        pre_op_output: Self::PreOpToInsertValue,
    ) -> Self::InsertSets {
        self.dyn_insert_value(from_data, pre_op_output)
    }

    type FromRow = Box<dyn RawFromRow<S> + Send>;

    fn from_row(&self) -> Self::FromRow {
        self.dyn_from_row()
    }

    type TakeInput = Box<dyn Any + Send>;

    type PostOp = Box<dyn BoxedOperation<S> + Send>;

    type PostOpData = Box<dyn Any + Send>;

    fn from_row_result(
        &self,
        from_data: Self::PostOpData,
        from_row: <Self::FromRow as FromRowData>::RData,
        pre_op_to_post_op: Self::PreOpToPostOp,
    ) -> (Self::PostOp, Self::TakeInput) {
        self.dyn_from_row_result(from_data, from_row, pre_op_to_post_op)
    }

    type PostOpOutput = Box<dyn Any + Send>;

    fn post_op_output(
        &self,
        poo: Box<dyn Any + Send>,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        self.dyn_post_op_output(poo)
    }

    type Output = Box<dyn Serialize<JsonAsString> + Send>;

    fn take(
        self,
        post_op_output: Self::PostOpOutput,
        insert_items: Self::TakeInput,
        pre_op_to_post_op: Self::PreOpToTake,
    ) -> Self::Output {
        Box::new(self.dyn_take(post_op_output, insert_items, pre_op_to_post_op))
    }
}

impl<S> InsertLinkConsumeData for Vec<JsonInsertOneToConsume<S>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type Link = Vec<Box<dyn JsonInsertOneLink<S> + Send>>;

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
        let mut links = Vec::with_capacity(self.len());
        let mut pre_op_data = Vec::with_capacity(self.len());
        let mut insert_value_data = Vec::with_capacity(self.len());
        let mut post_op_data = Vec::with_capacity(self.len());

        for item in self {
            links.push(item.link);
            pre_op_data.push(item.data.pre_op_data);
            insert_value_data.push(item.data.insert_value_data);
            post_op_data.push(item.data.post_op_data);
        }

        (
            links,
            InsertLinkData {
                pre_op_data,
                insert_value_data,
                post_op_data,
            },
        )
    }
}

impl<'b, S> InsertOneLink for Vec<Box<dyn JsonInsertOneLink<S> + Send + 'b>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type PreOp = Vec<Box<dyn BoxedOperation<S> + Send>>;

    type PreOpData = Vec<Box<dyn Any + Send>>;

    fn pre_operation_init(&self, input: Self::PreOpData) -> Self::PreOp {
        self.iter()
            .zip(input)
            .map(|(link, data)| link.as_ref().dyn_pre_op_init(data))
            .collect()
    }

    fn pre_op_split(
        &self,
        pre_op_output: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpToInsertValue,
            Self::PreOpToTake,
            Self::PreOpToPostOp,
        ),
        ConstraintViolation,
    > {
        let mut to_insert_value = Vec::with_capacity(self.len());
        let mut to_take = Vec::with_capacity(self.len());
        let mut to_post_op = Vec::with_capacity(self.len());

        for (link, output) in self.iter().zip(pre_op_output.into_iter()) {
            let (insert_value, take, post_op) = link.as_ref().dyn_pre_op_split(output)?;
            to_insert_value.push(insert_value);
            to_take.push(take);
            to_post_op.push(post_op);
        }

        Ok((to_insert_value, to_take, to_post_op))
    }

    type PreOpToInsertValue = Vec<Box<dyn Any + Send>>;

    type PreOpToTake = Vec<Box<dyn Any + Send>>;

    type PreOpToPostOp = Vec<Box<dyn Any + Send>>;

    type InsertReturning = AggregatedInsertReturning<S>;

    fn insert_returning(&self) -> Self::InsertReturning {
        AggregatedInsertReturning(
            self.iter()
                .map(|link| link.as_ref().dyn_insert_returning())
                .collect(),
        )
    }

    type InsertValuesData = Vec<Box<dyn Any + Send>>;

    type InsertSets = AggregatedInsertSets<S>;

    fn insert_value(
        &self,
        from_data: Self::InsertValuesData,
        pre_op_output: Self::PreOpToInsertValue,
    ) -> Self::InsertSets {
        AggregatedInsertSets(
            self.iter()
                .zip(from_data.into_iter().zip(pre_op_output))
                .map(|(link, (from_data, pre_op))| {
                    link.as_ref().dyn_insert_value(from_data, pre_op)
                })
                .collect(),
        )
    }

    type FromRow = JsonInsertLinksFromRow<S>;

    fn from_row(&self) -> Self::FromRow {
        JsonInsertLinksFromRow(
            self.iter()
                .map(|link| link.as_ref().dyn_from_row())
                .collect(),
        )
    }

    type TakeInput = Vec<Box<dyn Any + Send>>;

    type PostOp = Vec<Box<dyn BoxedOperation<S> + Send>>;

    type PostOpData = Vec<Box<dyn Any + Send>>;

    fn from_row_result(
        &self,
        from_data: Self::PostOpData,
        from_row: <Self::FromRow as FromRowData>::RData,
        pre_op_to_post_op: Self::PreOpToPostOp,
    ) -> (Self::PostOp, Self::TakeInput) {
        let mut take_inputs = Vec::with_capacity(self.len());

        let post_ops = self
            .iter()
            .zip(from_data)
            .zip(from_row.into_iter().zip(pre_op_to_post_op))
            .map(|((link, from_data), (from_row, pre_op_to_post_op))| {
                let (post_op, take_input) =
                    link.as_ref()
                        .dyn_from_row_result(from_data, from_row, pre_op_to_post_op);
                take_inputs.push(take_input);
                post_op
            })
            .collect();

        (post_ops, take_inputs)
    }

    type PostOpOutput = Vec<Box<dyn Any + Send>>;

    fn post_op_output(
        &self,
        poo: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        self.iter()
            .zip(poo.into_iter())
            .map(|(link, output)| link.as_ref().dyn_post_op_output(output))
            .collect()
    }

    type Output = Vec<Box<dyn Serialize<JsonAsString> + Send>>;

    fn take(
        self,
        post_op_output: Self::PostOpOutput,
        insert_items: Self::TakeInput,
        pre_op_to_take: Self::PreOpToTake,
    ) -> Self::Output {
        self.into_iter()
            .zip(
                post_op_output
                    .into_iter()
                    .zip(insert_items.into_iter().zip(pre_op_to_take)),
            )
            .map(|(link, (post_op_output, (insert_items, pre_op_to_take)))| {
                link.dyn_take(post_op_output, insert_items, pre_op_to_take)
            })
            .collect()
    }
}
