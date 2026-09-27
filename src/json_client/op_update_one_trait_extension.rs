use std::any::Any;

use sqlx::Database;

use crate::{
    database_extention::DatabaseExt,
    json_client::compat::{
        insert_sets::{AggregatedUpdateSets, DynUpdateSets},
        raw_from_row::RawFromRow,
    },
    fix_executor::ExecutorTrait,
    from_row::FromRowData,
    gen_serde::{Serialize, json_serialize_side::JsonAsString},
    operations::{
        Operation, OperationOutput,
        boxed_operation::BoxedOperation,
        insert::ConstraintViolation,
        update::{UpdateLink, UpdateLinkData, UpdateLinkSplit},
        operations_expressions_crossover::SelfPrescribedInsert,
    },
    sqlx_query_builder::{
        Expression, Join,
        combinators::OptionalExpression,
        trait_objects::{box_expression, BoxedExpression},
    },
};

pub struct JsonUpdateLinksFromRow<S>(pub Vec<Box<dyn RawFromRow<S> + Send>>);

impl<S: Database> FromRowData for JsonUpdateLinksFromRow<S> {
    type RData = Vec<Box<dyn Any + Send>>;
}

impl<'r, S: Database> crate::from_row::FromRowAlias<'r, S::Row> for JsonUpdateLinksFromRow<S> {
    fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, crate::from_row::FromRowError> {
        self.0
            .iter()
            .map(|from_row| RawFromRow::dyn_no_alias(&**from_row, row))
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
            .map(|from_row| RawFromRow::dyn_str_alias(&**from_row, row.clone()))
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
            .map(|from_row| RawFromRow::dyn_num_alias(&**from_row, row.clone()))
            .collect()
    }
}

pub struct JsonUpdateOneToConsume<S> {
    pub(crate) link: Box<dyn JsonUpdateOneLink<S> + Send>,
    pub(crate) data: UpdateLinkData<
        Box<dyn Any + Send>,
        Box<dyn Any + Send>,
        Box<dyn Any + Send>,
        Box<dyn Any + Send>,
    >,
}

impl<S> JsonUpdateOneToConsume<S> {
    pub fn new<T>(split: T) -> Self
    where
        S: sqlx::Database,
        T: UpdateLinkSplit,
        T::Link: JsonUpdateOneLink<S> + Send + 'static,
        <T::Link as UpdateLink>::InitSplitForWheres: Send + 'static,
        <T::Link as UpdateLink>::InitSplitForUpdateValues: Send + 'static,
        <T::Link as UpdateLink>::InitSplitForPreOp: Send + 'static,
        <T::Link as UpdateLink>::InitSplitPostOp: Send + 'static,
    {
        let (link, data) = split.init_split();
        Self {
            link: Box::new(link),
            data: UpdateLinkData {
                wheres: Box::new(data.wheres),
                update_values: Box::new(data.update_values),
                pre_op: Box::new(data.pre_op),
                post_op: Box::new(data.post_op),
            },
        }
    }
}

pub trait JsonUpdateOneLink<S: Database>: Send + Sync + 'static {
    fn dyn_pre_op(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send>;
    fn dyn_split_pre_op(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Result<
        (
            Box<dyn BoxedExpression<S> + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
        ),
        ConstraintViolation,
    >;
    fn dyn_wheres(&self, wheres: Box<dyn Any + Send>) -> Box<dyn BoxedExpression<S> + Send>;
    fn dyn_update_names(&self) -> Box<dyn BoxedExpression<S> + Send>;
    fn dyn_update_values(
        &self,
        values: Box<dyn Any + Send>,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Box<dyn DynUpdateSets<S> + Send>;
    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send>;
    fn dyn_post_op(
        &self,
        init_split: Box<dyn Any + Send>,
        pre_op_split: Box<dyn Any + Send>,
    ) -> Box<dyn BoxedOperation<S> + Send>;
    fn dyn_from_row_result(
        &self,
        row_data: &Box<dyn Any + Send>,
        post_op: &mut Box<dyn BoxedOperation<S> + Send>,
    );
    fn dyn_post_op_output(
        &self,
        poo: Box<dyn Any + Send>,
    ) -> Result<Box<dyn Any + Send>, ConstraintViolation>;
    fn dyn_take(
        &self,
        from_row: Box<dyn Any + Send>,
        post_op: &mut Box<dyn Any + Send>,
        pre_op_split_take: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send>;
}

impl<T, S> JsonUpdateOneLink<S> for T
where
    T: Send + Sync + 'static,
    S: sqlx::Database + DatabaseExt,
    T: UpdateLink,
    T::InitSplitForPreOp: 'static + Send,
    T::PreOp: Operation<S> + 'static,
    T::PreOpSplitWheres: Send + 'static + OptionalExpression,
    for<'e> Join<T::PreOpSplitWheres>: Expression<'e, S>,
    T::PreOpSplitValues: Send + 'static,
    T::PreOpSplitPostOp: Send + 'static,
    T::PreOpSplitTake: Send + 'static,
    T::InitSplitForWheres: Send + 'static,
    T::UpdateWhere: Send + 'static + OptionalExpression,
    for<'e> Join<T::UpdateWhere>: Expression<'e, S>,
    T::UpdateReturning: Send + 'static + OptionalExpression,
    for<'e> Join<T::UpdateReturning>: Expression<'e, S>,
    T::InitSplitForUpdateValues: Send + 'static,
    T::UpdateSets: Send + 'static + SelfPrescribedInsert,
    <T::UpdateSets as SelfPrescribedInsert>::UpdateSets: Send + OptionalExpression,
    for<'e> Join<<T::UpdateSets as SelfPrescribedInsert>::UpdateSets>: Expression<'e, S>,
    T::FromRow: Send + 'static + RawFromRow<S>,
    T::InitSplitPostOp: Send + 'static,
    T::PostOp: Operation<S> + Send + 'static,
    T::PostOpOutput: Send + 'static,
    T::Output: Send + 'static + Serialize<JsonAsString>,
{
    fn dyn_pre_op(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send> {
        let downcasted = input.downcast::<T::InitSplitForPreOp>().unwrap();
        Box::new(self.pre_op(*downcasted))
    }

    fn dyn_split_pre_op(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Result<
        (
            Box<dyn BoxedExpression<S> + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
            Box<dyn Any + Send>,
        ),
        ConstraintViolation,
    > {
        let downcasted_pre_op_output = pre_op_output
            .downcast::<<T::PreOp as OperationOutput>::Output>()
            .unwrap();
        let (wheres, values, post_op, take) = self.split_pre_op(*downcasted_pre_op_output)?;
        Ok((
            box_expression(wheres, " AND "),
            Box::new(values),
            Box::new(post_op),
            Box::new(take),
        ))
    }

    fn dyn_wheres(&self, wheres: Box<dyn Any + Send>) -> Box<dyn BoxedExpression<S> + Send> {
        let downcasted = wheres.downcast::<T::InitSplitForWheres>().unwrap();
        box_expression(self.wheres(*downcasted), " AND ")
    }

    fn dyn_update_names(&self) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.update_names(), ", ")
    }

    fn dyn_update_values(
        &self,
        values: Box<dyn Any + Send>,
        pre_op_output: Box<dyn Any + Send>,
    ) -> Box<dyn DynUpdateSets<S> + Send> {
        let downcasted_values = values.downcast::<T::InitSplitForUpdateValues>().unwrap();
        let downcasted_pre_op_output = pre_op_output.downcast::<T::PreOpSplitValues>().unwrap();
        Box::new(self.update_values(*downcasted_values, *downcasted_pre_op_output))
    }

    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send> {
        Box::new(self.from_row())
    }

    fn dyn_post_op(
        &self,
        init_split: Box<dyn Any + Send>,
        pre_op_split: Box<dyn Any + Send>,
    ) -> Box<dyn BoxedOperation<S> + Send> {
        let downcasted_init = init_split.downcast::<T::InitSplitPostOp>().unwrap();
        let downcasted_pre = pre_op_split.downcast::<T::PreOpSplitPostOp>().unwrap();
        Box::new(self.post_op(*downcasted_init, *downcasted_pre))
    }

    fn dyn_from_row_result(
        &self,
        row_data: &Box<dyn Any + Send>,
        post_op: &mut Box<dyn BoxedOperation<S> + Send>,
    ) {
        let downcasted_row = row_data
            .downcast_ref::<<T::FromRow as FromRowData>::RData>()
            .unwrap();
        let downcasted_post_op = (post_op.as_mut() as &mut dyn Any)
            .downcast_mut::<T::PostOp>()
            .unwrap();
        self.from_row_result(downcasted_row, downcasted_post_op);
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
        &self,
        from_row: Box<dyn Any + Send>,
        post_op: &mut Box<dyn Any + Send>,
        pre_op_split_take: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send> {
        let downcasted_from_row = from_row
            .downcast::<<T::FromRow as FromRowData>::RData>()
            .unwrap();
        let downcasted_post_op = post_op.downcast_mut::<T::PostOpOutput>().unwrap();
        let downcasted_take = pre_op_split_take
            .downcast_mut::<T::PreOpSplitTake>()
            .unwrap();
        Box::new(self.take(*downcasted_from_row, downcasted_post_op, downcasted_take))
    }
}

impl<'b, S> UpdateLink for Box<dyn JsonUpdateOneLink<S> + Send + 'b>
where
    S: Database,
{
    type InitSplitForPreOp = Box<dyn Any + Send>;

    type PreOp = Box<dyn BoxedOperation<S> + Send>;

    fn pre_op(&self, init_split_for_pre_op: Self::InitSplitForPreOp) -> Self::PreOp {
        self.dyn_pre_op(init_split_for_pre_op)
    }

    fn split_pre_op(
        &self,
        pre_op_output: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpSplitWheres,
            Self::PreOpSplitValues,
            Self::PreOpSplitPostOp,
            Self::PreOpSplitTake,
        ),
        ConstraintViolation,
    > {
        self.dyn_split_pre_op(pre_op_output)
    }

    type PreOpSplitWheres = Box<dyn BoxedExpression<S> + Send>;

    type PreOpSplitValues = Box<dyn Any + Send>;

    type PreOpSplitPostOp = Box<dyn Any + Send>;

    type PreOpSplitTake = Box<dyn Any + Send>;

    type InitSplitForWheres = Box<dyn Any + Send>;

    type UpdateWhere = Box<dyn BoxedExpression<S> + Send>;

    fn wheres(&self, wheres: Self::InitSplitForWheres) -> Self::UpdateWhere {
        self.dyn_wheres(wheres)
    }

    type UpdateReturning = Box<dyn BoxedExpression<S> + Send>;

    fn update_names(&self) -> Self::UpdateReturning {
        self.dyn_update_names()
    }

    type InitSplitForUpdateValues = Box<dyn Any + Send>;

    type UpdateSets = Box<dyn DynUpdateSets<S> + Send>;

    fn update_values(
        &self,
        values: Self::InitSplitForUpdateValues,
        pre_op_output: Self::PreOpSplitValues,
    ) -> Self::UpdateSets {
        self.dyn_update_values(values, pre_op_output)
    }

    type FromRow = Box<dyn RawFromRow<S> + Send>;

    fn from_row(&self) -> Self::FromRow {
        self.dyn_from_row()
    }

    type InitSplitPostOp = Box<dyn Any + Send>;

    type PostOp = Box<dyn BoxedOperation<S> + Send>;

    fn post_op(
        &self,
        from_init_split: Self::InitSplitPostOp,
        from_pre_op: Self::PreOpSplitPostOp,
    ) -> Self::PostOp {
        self.dyn_post_op(from_init_split, from_pre_op)
    }

    fn from_row_result(
        &self,
        row_data: &<Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOp,
    ) {
        self.dyn_from_row_result(row_data, post_op)
    }

    type PostOpOutput = Box<dyn Any + Send>;

    fn post_op_output(
        &self,
        poo: <Self::PostOp as OperationOutput>::Output,
    ) -> Result<Self::PostOpOutput, ConstraintViolation> {
        self.dyn_post_op_output(poo)
    }

    type Output = Box<dyn Serialize<JsonAsString> + Send>;

    fn take(
        &self,
        from_row: <Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOpOutput,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
        self.dyn_take(from_row, post_op, pre_op_split_take)
    }
}

impl<S> UpdateLinkSplit for Vec<JsonUpdateOneToConsume<S>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type Link = Vec<Box<dyn JsonUpdateOneLink<S> + Send>>;

    fn init_split(
        self,
    ) -> (
        Self::Link,
        UpdateLinkData<
            <Self::Link as UpdateLink>::InitSplitForWheres,
            <Self::Link as UpdateLink>::InitSplitForUpdateValues,
            <Self::Link as UpdateLink>::InitSplitForPreOp,
            <Self::Link as UpdateLink>::InitSplitPostOp,
        >,
    ) {
        let mut links = Vec::with_capacity(self.len());
        let mut wheres = Vec::with_capacity(self.len());
        let mut update_values = Vec::with_capacity(self.len());
        let mut pre_op = Vec::with_capacity(self.len());
        let mut post_op = Vec::with_capacity(self.len());

        for item in self {
            links.push(item.link);
            wheres.push(item.data.wheres);
            update_values.push(item.data.update_values);
            pre_op.push(item.data.pre_op);
            post_op.push(item.data.post_op);
        }

        (
            links,
            UpdateLinkData {
                wheres,
                update_values,
                pre_op,
                post_op,
            },
        )
    }
}

impl<'b, S> UpdateLink for Vec<Box<dyn JsonUpdateOneLink<S> + Send + 'b>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type InitSplitForPreOp = Vec<Box<dyn Any + Send>>;

    type PreOp = Vec<Box<dyn BoxedOperation<S> + Send>>;

    fn pre_op(&self, init_split_for_pre_op: Self::InitSplitForPreOp) -> Self::PreOp {
        self.iter()
            .zip(init_split_for_pre_op)
            .map(|(link, data)| link.as_ref().dyn_pre_op(data))
            .collect()
    }

    fn split_pre_op(
        &self,
        pre_op_output: <Self::PreOp as OperationOutput>::Output,
    ) -> Result<
        (
            Self::PreOpSplitWheres,
            Self::PreOpSplitValues,
            Self::PreOpSplitPostOp,
            Self::PreOpSplitTake,
        ),
        ConstraintViolation,
    > {
        let mut wheres = Vec::with_capacity(self.len());
        let mut values = Vec::with_capacity(self.len());
        let mut post_op = Vec::with_capacity(self.len());
        let mut take = Vec::with_capacity(self.len());

        for (link, output) in self.iter().zip(pre_op_output.into_iter()) {
            let (w, v, p, t) = link.as_ref().dyn_split_pre_op(output)?;
            wheres.push(w);
            values.push(v);
            post_op.push(p);
            take.push(t);
        }

        Ok((wheres, values, post_op, take))
    }

    type PreOpSplitWheres = Vec<Box<dyn BoxedExpression<S> + Send>>;

    type PreOpSplitValues = Vec<Box<dyn Any + Send>>;

    type PreOpSplitPostOp = Vec<Box<dyn Any + Send>>;

    type PreOpSplitTake = Vec<Box<dyn Any + Send>>;

    type InitSplitForWheres = Vec<Box<dyn Any + Send>>;

    type UpdateWhere = Vec<Box<dyn BoxedExpression<S> + Send>>;

    fn wheres(&self, wheres: Self::InitSplitForWheres) -> Self::UpdateWhere {
        self
            .iter()
            .zip(wheres)
            .map(|(link, data)| link.as_ref().dyn_wheres(data))
            .collect()
    }

    type UpdateReturning = Vec<Box<dyn BoxedExpression<S> + Send>>;

    fn update_names(&self) -> Self::UpdateReturning {
        self
            .iter()
            .map(|link| link.as_ref().dyn_update_names())
            .collect()
    }

    type InitSplitForUpdateValues = Vec<Box<dyn Any + Send>>;

    type UpdateSets = AggregatedUpdateSets<S>;

    fn update_values(
        &self,
        values: Self::InitSplitForUpdateValues,
        pre_op_output: Self::PreOpSplitValues,
    ) -> Self::UpdateSets {
        AggregatedUpdateSets(
            self.iter()
                .zip(values.into_iter().zip(pre_op_output))
                .map(|(link, (values, pre_op))| {
                    link.as_ref().dyn_update_values(values, pre_op)
                })
                .collect(),
        )
    }

    type FromRow = JsonUpdateLinksFromRow<S>;

    fn from_row(&self) -> Self::FromRow {
        JsonUpdateLinksFromRow(
            self.iter()
                .map(|link| link.as_ref().dyn_from_row())
                .collect(),
        )
    }

    type InitSplitPostOp = Vec<Box<dyn Any + Send>>;

    type PostOp = Vec<Box<dyn BoxedOperation<S> + Send>>;

    fn post_op(
        &self,
        from_init_split: Self::InitSplitPostOp,
        from_pre_op: Self::PreOpSplitPostOp,
    ) -> Self::PostOp {
        self.iter()
            .zip(from_init_split.into_iter().zip(from_pre_op))
            .map(|(link, (init, pre))| link.as_ref().dyn_post_op(init, pre))
            .collect()
    }

    fn from_row_result(
        &self,
        row_data: &<Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOp,
    ) {
        for (link, (row, post)) in self.iter().zip(row_data.iter().zip(post_op.iter_mut())) {
            link.as_ref().dyn_from_row_result(row, post);
        }
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
        &self,
        from_row: <Self::FromRow as FromRowData>::RData,
        post_op: &mut Self::PostOpOutput,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
        self.iter()
            .zip(from_row)
            .zip(post_op.drain(..))
            .zip(pre_op_split_take.drain(..))
            .map(|(((link, from_row), mut post_op), mut take)| {
                link.as_ref().dyn_take(from_row, &mut post_op, &mut take)
            })
            .collect()
    }
}
