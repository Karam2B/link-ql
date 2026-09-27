use core::fmt;
use std::any::Any;
use std::ops::{Deref, DerefMut};

use sqlx::Database;
use tracing::warn;

use crate::{
    database_extention::DatabaseExt,
    from_row::{FromRowAlias, FromRowData},
    gen_serde::{Serialize, SerializedJson, json_serialize_side::JsonAsString},
    json_client::select_items_trait_object::{SelectItemsTraitObject, ToImplSelectItems},
    operations::{
        OperationOutput, boxed_operation::BoxedOperation, fetch_many::LinkFetch,
        operations_expressions_crossover::ExpressionsForOperation,
    },
    sqlx_query_builder::{
        Expression, Join,
        combinators::OptionalExpression,
        trait_objects::{box_expression, BoxedExpression},
    },
};

pub trait JsonLinkFetchMany<S> {
    fn select_items_expr(&self) -> Box<dyn SelectItemsTraitObject<S, ()>>;
    fn post_select_each_2(&self, item: &Box<dyn Any + Send>, poi: &mut Box<dyn Any + Send>);
    fn post_operation_input_init_2(&self) -> Box<dyn Any + Send>;
    fn post_select_2(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send>;
    fn take_2(
        &self,
        item: Box<dyn Any + Send>,
        op: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send>;
    fn join_expr(&self) -> Box<dyn BoxedExpression<S> + Send>;
    fn wheres_expr(&self) -> Box<dyn BoxedExpression<S> + Send>;
}

impl<S, T> JsonLinkFetchMany<S> for T
where
    T: Clone + Send + 'static,
    T::SelectItems: Send,
    T::SelectItems: FromRowData,
    S: DatabaseExt,
    T: LinkFetch,
    T::SelectItems: Send
        + ExpressionsForOperation<
            NumScopedAliased: 'static + Send + OptionalExpression,
            ScopedAliased: 'static + Send + OptionalExpression,
        >,
    for<'e> Join<<T::SelectItems as ExpressionsForOperation>::NumScopedAliased>: Expression<'e, S>,
    for<'e> Join<<T::SelectItems as ExpressionsForOperation>::ScopedAliased>: Expression<'e, S>,
    T::OpInput: 'static + Send,
    T::Op: Send + 'static + BoxedOperation<S>,
    T::Op: OperationOutput,
    T::Output: Serialize<JsonAsString>,
    T::SelectItems: FromRowData<RData: Send + 'static>,
    T::SelectItems: for<'r> FromRowAlias<'r, S::Row>,
    T::Join: Send + 'static + OptionalExpression,
    for<'e> Join<T::Join>: Expression<'e, S>,
    T::Wheres: Send + 'static + OptionalExpression,
    for<'e> Join<T::Wheres>: Expression<'e, S>,
    T::Output: Send + fmt::Debug,
{
    fn join_expr(&self) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.non_duplicating_join_expressions(), " ")
    }

    fn wheres_expr(&self) -> Box<dyn BoxedExpression<S> + Send> {
        box_expression(self.where_expressions(), " AND ")
    }

    fn take_2(
        &self,
        item: Box<dyn Any + Send>,
        op: &mut Box<dyn Any + Send>,
    ) -> Box<dyn Serialize<JsonAsString> + Send> {
        let output = self.take_many(
            *item
                .downcast::<<T::SelectItems as FromRowData>::RData>()
                .unwrap(),
            op.downcast_mut::<<T::Op as OperationOutput>::Output>()
                .unwrap(),
        );

        Box::new(output)
    }

    fn select_items_expr(&self) -> Box<dyn SelectItemsTraitObject<S, ()>> {
        Box::new(ToImplSelectItems {
            select_items: self.non_aggregating_select_items(),
            cast_from_row_result: (),
        })
    }

    fn post_select_each_2(&self, item: &Box<dyn Any + Send>, poi: &mut Box<dyn Any + Send>) {
        let item = item
            .deref()
            .downcast_ref::<<T::SelectItems as FromRowData>::RData>()
            .unwrap();
        let poi = poi.deref_mut().downcast_mut::<T::OpInput>().unwrap();

        self.operation_fix_on_many(item, poi)
    }

    fn post_operation_input_init_2(&self) -> Box<dyn Any + Send> {
        Box::new(self.operation_initialize_input())
    }

    fn post_select_2(&self, input: Box<dyn Any + Send>) -> Box<dyn BoxedOperation<S> + Send> {
        Box::new(self.operation_construct(*input.downcast::<T::OpInput>().unwrap()))
    }
}

impl<'r, S> LinkFetch for Box<dyn JsonLinkFetchMany<S> + Send + 'r>
where
    Box<dyn SelectItemsTraitObject<S, ()>>: FromRowData<RData = Box<dyn Any + Send>>,
    Box<dyn BoxedOperation<S> + Send>: OperationOutput<Output = Box<dyn Any + Send>>,
{
    type SelectItems = Box<dyn SelectItemsTraitObject<S, ()>>;

    fn non_aggregating_select_items(&self) -> Self::SelectItems {
        self.select_items_expr()
    }

    fn operation_fix_on_many(&self, item: &Box<dyn Any + Send>, poi: &mut Self::OpInput)
    where
        Self::SelectItems: FromRowData,
    {
        self.post_select_each_2(item, poi)
    }

    fn take_many(
        &self,
        item: <Self::SelectItems as FromRowData>::RData,
        op: &mut <Self::Op as OperationOutput>::Output,
    ) -> Self::Output
    where
        Self::SelectItems: FromRowData,
        Self::Op: OperationOutput,
    {
        self.take_2(item, op)
    }

    type Join = Box<dyn BoxedExpression<S> + Send>;

    fn non_duplicating_join_expressions(&self) -> Self::Join {
        self.join_expr()
    }

    type Wheres = Box<dyn BoxedExpression<S> + Send>;

    fn where_expressions(&self) -> Self::Wheres {
        self.wheres_expr()
    }

    type Output = Box<dyn Serialize<JsonAsString> + Send>;

    type OpInput = Box<dyn Any + Send>;

    fn operation_initialize_input(&self) -> Self::OpInput {
        self.post_operation_input_init_2()
    }

    type Op = Box<dyn BoxedOperation<S> + Send>;

    fn operation_construct(&self, input: Self::OpInput) -> Self::Op
    where
        Self::SelectItems: FromRowData,
    {
        self.post_select_2(input)
    }
}

impl<'r, S> LinkFetch for Vec<Box<dyn JsonLinkFetchMany<S> + Send + 'r>>
where
    S: Database,
    S: DatabaseExt,
    Vec<Box<dyn SelectItemsTraitObject<S, ()>>>: FromRowData<RData = Vec<Box<dyn Any + Send>>>,
    Vec<Box<dyn BoxedOperation<S> + Send>>: OperationOutput<Output = Vec<Box<dyn Any + Send>>>,
{
    type SelectItems = Vec<Box<dyn SelectItemsTraitObject<S, ()>>>;

    fn non_aggregating_select_items(&self) -> Self::SelectItems {
        self.iter().map(|each| each.select_items_expr()).collect()
    }

    fn operation_fix_on_many(
        &self,
        item: &Vec<Box<dyn Any + Send>>,
        poi: &mut Vec<Box<dyn Any + Send>>,
    ) where
        Self::SelectItems: FromRowData,
    {
        for (i, each) in self.iter().enumerate() {
            each.post_select_each_2(&item[i], &mut poi[i]);
        }
    }

    fn take_many(
        &self,
        item: Vec<Box<dyn Any + Send>>,
        op: &mut Vec<Box<dyn Any + Send>>,
    ) -> Self::Output
    where
        Self::SelectItems: FromRowData,
        Self::Op: OperationOutput,
    {
        self.iter()
            .zip(item)
            .zip(op.iter_mut())
            .map(|((each, item), op)| each.take_2(item, op))
            .collect()
    }

    type Join = Box<dyn BoxedExpression<S> + Send>;

    fn non_duplicating_join_expressions(&self) -> Self::Join {
        if let Some(first) = self.first() {
            warn!("multiple links");
            first.join_expr()
        } else {
            Box::new(())
        }
    }

    type Wheres = ();

    fn where_expressions(&self) -> Self::Wheres {}

    type Output = Vec<Box<dyn Serialize<JsonAsString> + Send>>;

    type OpInput = Vec<Box<dyn Any + Send>>;

    fn operation_initialize_input(&self) -> Self::OpInput {
        self.iter()
            .map(|each| each.post_operation_input_init_2())
            .collect()
    }

    type Op = Vec<Box<dyn BoxedOperation<S> + Send>>;

    fn operation_construct(&self, input: Self::OpInput) -> Self::Op
    where
        Self::SelectItems: FromRowData,
    {
        self.iter()
            .zip(input)
            .map(|(each, input)| each.post_select_2(input))
            .collect()
    }
}
