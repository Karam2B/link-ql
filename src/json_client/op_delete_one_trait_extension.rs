use std::any::Any;

use sqlx::Database;

type JsonDeleteOneInitSplitForPreOp = Vec<Box<dyn Any + Send>>;
type JsonDeleteOneInitSplitForWheres = Vec<Box<dyn Any + Send>>;

use crate::{
    database_extention::DatabaseExt,
    json_client::{
        compat::{
            delete_pre_op::ErasedDeletePreOp,
            raw_from_row::RawFromRow,
        },
        dynamic_collection::DynamicCollection,
    },
    fix_executor::ExecutorTrait,
    from_row::FromRowData,
    gen_serde::{Serialize, json_serialize_side::JsonAsString},
    operations::{
        Operation, OperationOutput,
        boxed_operation::BoxedOperation,
        delete::{DeleteLink, DeleteLinkData, DeleteLinkPreOp, DeleteLinkSplit},
    },
    sqlx_query_builder::{
        Expression, Join,
        combinators::OptionalExpression,
        trait_objects::{box_expression, BoxedExpression},
    },
};

pub struct JsonDeleteLinksFromRow<S>(pub Vec<Box<dyn RawFromRow<S> + Send>>);

unsafe impl<S: Database> Sync for JsonDeleteLinksFromRow<S> {}

impl<S: Database> FromRowData for JsonDeleteLinksFromRow<S> {
    type RData = Vec<Box<dyn Any + Send>>;
}

impl<'r, S: Database> crate::from_row::FromRowAlias<'r, S::Row> for JsonDeleteLinksFromRow<S> {
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

pub struct JsonDeleteOneToConsume<S> {
    pub(crate) link: Box<dyn JsonDeleteOneLink<S> + Send>,
    pub(crate) init_split_for_pre_op: Box<dyn Any + Send>,
    pub(crate) data: DeleteLinkData<Box<dyn Any + Send>>,
}

impl<S> JsonDeleteOneToConsume<S> {
    pub fn from_split<T>(split: T) -> Self
    where
        S: sqlx::Database,
        T: DeleteLinkSplit,
        T::Link: JsonDeleteOneLink<S> + Send + 'static,
        T::InitSplitForPreOp: Send + 'static,
        <T::Link as DeleteLink>::InitSplitForWheres: Send + 'static,
    {
        let (link, init_split_for_pre_op, data) = split.init_split();
        Self {
            link: Box::new(link),
            init_split_for_pre_op: Box::new(init_split_for_pre_op),
            data: DeleteLinkData {
                wheres: Box::new(data.wheres),
            },
        }
    }
}

pub trait JsonDeleteOneLink<S: Database>: Send + Sync + 'static {
    fn dyn_pre_op(
        &self,
        init: Box<dyn Any + Send>,
        wheres: &dyn Any,
    ) -> Box<dyn BoxedOperation<S> + Send>;
    fn dyn_split_pre_op(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> (Box<dyn Any + Send>, Box<dyn Any + Send>);
    fn dyn_wheres(
        &self,
        init_split_for_wheres: Box<dyn Any + Send>,
        pre_op_split_wheres: Box<dyn Any + Send>,
    ) -> Box<dyn BoxedExpression<S> + Send>;
    fn dyn_delete_return_expression(&self) -> Box<dyn BoxedExpression<S> + Send>;
    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send>;
    fn dyn_take_once(
        &self,
        links: Box<dyn Any + Send>,
        pre_op_split_take: Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send>;
    fn dyn_take_mut(
        &self,
        links: Box<dyn Any + Send>,
        pre_op_split_take: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send>;
}

impl<T, S> JsonDeleteOneLink<S> for T
where
    T: Send + Sync + 'static,
    S: Database + DatabaseExt + Sync,
    T: DeleteLink,
    T: ErasedDeletePreOp<S>,
    T::PreOpOutput: Send + 'static,
    T::PreOpSplitWheres: Send + 'static,
    T::PreOpSplitTake: Send + 'static,
    T::InitSplitForWheres: Send + 'static,
    T::Wheres: Send + OptionalExpression,
    for<'e> Join<T::Wheres>: Expression<'e, S>,
    T::DeleteReturnExpression: Send + OptionalExpression,
    for<'e> Join<T::DeleteReturnExpression>: Expression<'e, S>,
    T::DeleteReturnFromRow: Send + Sync + RawFromRow<S>,
    T::Output: Send + Serialize<JsonAsString>,
{
    fn dyn_pre_op(
        &self,
        init: Box<dyn Any + Send>,
        wheres: &dyn Any,
    ) -> Box<dyn BoxedOperation<S> + Send> {
        self.erased_pre_op(init, wheres)
    }

    fn dyn_split_pre_op(
        &self,
        pre_op_output: Box<dyn Any + Send>,
    ) -> (Box<dyn Any + Send>, Box<dyn Any + Send>) {
        let downcasted = pre_op_output.downcast::<T::PreOpOutput>().unwrap();
        let (wheres, take) = self.split_pre_op(*downcasted);
        (Box::new(wheres), Box::new(take))
    }

    fn dyn_wheres(
        &self,
        init_split_for_wheres: Box<dyn Any + Send>,
        pre_op_split_wheres: Box<dyn Any + Send>,
    ) -> Box<dyn BoxedExpression<S> + Send> {
        let init = init_split_for_wheres
            .downcast::<T::InitSplitForWheres>()
            .unwrap();
        let pre = pre_op_split_wheres
            .downcast::<T::PreOpSplitWheres>()
            .unwrap();
        box_expression(self.wheres(*init, *pre), " AND ")
    }

    fn dyn_delete_return_expression(&self) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.delete_return_expression(), ", ")
    }

    fn dyn_from_row(&self) -> Box<dyn RawFromRow<S> + Send> {
        Box::new(self.from_row())
    }

    fn dyn_take_once(
        &self,
        links: Box<dyn Any + Send>,
        pre_op_split_take: Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send> {
        let downcasted_links = links
            .downcast::<<T::DeleteReturnFromRow as FromRowData>::RData>()
            .unwrap();
        let downcasted_take = pre_op_split_take.downcast::<T::PreOpSplitTake>().unwrap();
        Box::new(self.take_once(*downcasted_links, *downcasted_take))
    }

    fn dyn_take_mut(
        &self,
        links: Box<dyn Any + Send>,
        pre_op_split_take: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send> {
        let downcasted_links = links
            .downcast::<<T::DeleteReturnFromRow as FromRowData>::RData>()
            .unwrap();
        let downcasted_take = pre_op_split_take
            .downcast_mut::<T::PreOpSplitTake>()
            .unwrap();
        Box::new(self.take_mut(*downcasted_links, downcasted_take))
    }
}

impl<W, S> DeleteLinkPreOp<W> for Box<dyn JsonDeleteOneLink<S> + Send>
where
    W: Clone + Send + 'static,
    S: Database,
{
    type InitSplitForPreOp = Box<dyn Any + Send>;

    type PreOp = Box<dyn BoxedOperation<S> + Send>;

    fn pre_op(&self, init: Self::InitSplitForPreOp, wheres: &W) -> Self::PreOp {
        self.dyn_pre_op(init, wheres as &dyn Any)
    }
}

impl<S> DeleteLink for Box<dyn JsonDeleteOneLink<S> + Send>
where
    S: Database,
{
    type Output = Box<dyn Serialize<JsonAsString> + Send>;

    type PreOpOutput = Box<dyn Any + Send>;

    type PreOpSplitWheres = Box<dyn Any + Send>;

    type PreOpSplitTake = Box<dyn Any + Send>;

    fn split_pre_op(
        &self,
        pre_op_output: Self::PreOpOutput,
    ) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake) {
        self.dyn_split_pre_op(pre_op_output)
    }

    type InitSplitForWheres = Box<dyn Any + Send>;

    type Wheres = Box<dyn BoxedExpression<S> + Send>;

    fn wheres(
        &self,
        init_split_for_wheres: Self::InitSplitForWheres,
        pre_op_split_wheres: Self::PreOpSplitWheres,
    ) -> Self::Wheres {
        self.dyn_wheres(init_split_for_wheres, pre_op_split_wheres)
    }

    type DeleteReturnExpression = Box<dyn BoxedExpression<S> + Send>;

    fn delete_return_expression(&self) -> Self::DeleteReturnExpression {
        self.dyn_delete_return_expression()
    }

    type DeleteReturnFromRow = Box<dyn RawFromRow<S> + Send>;

    fn from_row(&self) -> Self::DeleteReturnFromRow {
        self.dyn_from_row()
    }

    fn take_mut(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
        self.dyn_take_mut(links, pre_op_split_take)
    }

    fn take_once(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: Self::PreOpSplitTake,
    ) -> Self::Output {
        self.dyn_take_once(links, pre_op_split_take)
    }
}

impl<S> DeleteLinkSplit for Vec<JsonDeleteOneToConsume<S>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type Link = Vec<Box<dyn JsonDeleteOneLink<S> + Send>>;

    type InitSplitForPreOp = JsonDeleteOneInitSplitForPreOp;

    fn init_split(
        self,
    ) -> (
        Self::Link,
        Self::InitSplitForPreOp,
        DeleteLinkData<<Self::Link as DeleteLink>::InitSplitForWheres>,
    ) {
        let mut links = Vec::with_capacity(self.len());
        let mut init_split_for_pre_op = Vec::with_capacity(self.len());
        let mut wheres = Vec::with_capacity(self.len());

        for item in self {
            links.push(item.link);
            init_split_for_pre_op.push(item.init_split_for_pre_op);
            wheres.push(item.data.wheres);
        }

        (links, init_split_for_pre_op, DeleteLinkData { wheres })
    }
}

impl<'b, W, S> DeleteLinkPreOp<W> for Vec<Box<dyn JsonDeleteOneLink<S> + Send + 'b>>
where
    W: Clone + Send + 'static,
    S: Database + DatabaseExt + ExecutorTrait,
    Vec<JsonDeleteOneToConsume<S>>:
        DeleteLinkSplit<InitSplitForPreOp: IntoIterator<Item = Box<dyn Any + Send>> + Send>,
{
    type InitSplitForPreOp = <Vec<JsonDeleteOneToConsume<S>> as DeleteLinkSplit>::InitSplitForPreOp;

    type PreOp = Vec<Box<dyn BoxedOperation<S> + Send>>;

    fn pre_op(&self, init: Self::InitSplitForPreOp, wheres: &W) -> Self::PreOp {
        self.iter()
            .zip(init)
            .map(|(link, data)| link.as_ref().dyn_pre_op(data, wheres as &dyn Any))
            .collect()
    }
}

impl<'b, S> DeleteLink for Vec<Box<dyn JsonDeleteOneLink<S> + Send + 'b>>
where
    S: Database + DatabaseExt + ExecutorTrait,
{
    type Output = Vec<Box<dyn Serialize<JsonAsString> + Send>>;

    type PreOpOutput = Vec<Box<dyn Any + Send>>;

    type PreOpSplitWheres = Vec<Box<dyn Any + Send>>;

    type PreOpSplitTake = Vec<Box<dyn Any + Send>>;

    fn split_pre_op(
        &self,
        pre_op_output: Self::PreOpOutput,
    ) -> (Self::PreOpSplitWheres, Self::PreOpSplitTake) {
        let mut wheres = Vec::with_capacity(self.len());
        let mut take = Vec::with_capacity(self.len());

        for (link, output) in self.iter().zip(pre_op_output.into_iter()) {
            let (w, t) = link.as_ref().dyn_split_pre_op(output);
            wheres.push(w);
            take.push(t);
        }

        (wheres, take)
    }

    type InitSplitForWheres = JsonDeleteOneInitSplitForWheres;

    type Wheres = Vec<Box<dyn BoxedExpression<S> + Send>>;

    fn wheres(
        &self,
        init_split_for_wheres: Self::InitSplitForWheres,
        pre_op_split_wheres: Self::PreOpSplitWheres,
    ) -> Self::Wheres {
        self
            .iter()
            .zip(init_split_for_wheres.into_iter().zip(pre_op_split_wheres))
            .map(|(link, (init, pre))| link.as_ref().dyn_wheres(init, pre))
            .collect()
    }

    type DeleteReturnExpression = Vec<Box<dyn BoxedExpression<S> + Send>>;

    fn delete_return_expression(&self) -> Self::DeleteReturnExpression {
        self
            .iter()
            .map(|link| link.as_ref().dyn_delete_return_expression())
            .collect()
    }

    type DeleteReturnFromRow = JsonDeleteLinksFromRow<S>;

    fn from_row(&self) -> Self::DeleteReturnFromRow {
        JsonDeleteLinksFromRow(
            self.iter()
                .map(|link| link.as_ref().dyn_from_row())
                .collect(),
        )
    }

    fn take_mut(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: &mut Self::PreOpSplitTake,
    ) -> Self::Output {
        self.iter()
            .zip(links.into_iter().zip(pre_op_split_take.iter_mut()))
            .map(|(link, (row, take))| link.as_ref().dyn_take_mut(row, take))
            .collect()
    }

    fn take_once(
        &self,
        links: <Self::DeleteReturnFromRow as FromRowData>::RData,
        pre_op_split_take: Self::PreOpSplitTake,
    ) -> Self::Output {
        self.iter()
            .zip(links.into_iter().zip(pre_op_split_take))
            .map(|(link, (row, take))| link.as_ref().dyn_take_once(row, take))
            .collect()
    }
}
