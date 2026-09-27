//! json_client-local shims for APIs removed from the crate root.

pub mod raw_from_row {
    use std::any::Any;

    use sqlx::Database;

    use crate::from_row::{FromRowAlias, FromRowData, FromRowError, RowNumAliased, RowStrAliased};

    pub trait RawFromRow<S: Database> {
        fn dyn_no_alias<'r>(&self, row: &'r S::Row) -> Result<Box<dyn Any + Send>, FromRowError>;
        fn dyn_str_alias<'r>(
            &self,
            row: RowStrAliased<'r, S::Row>,
        ) -> Result<Box<dyn Any + Send>, FromRowError>;
        fn dyn_num_alias<'r>(
            &self,
            row: RowNumAliased<'r, S::Row>,
        ) -> Result<Box<dyn Any + Send>, FromRowError>;
    }

    impl<S, T> RawFromRow<S> for T
    where
        S: Database,
        T: for<'r> FromRowAlias<'r, S::Row, RData: Send + 'static>,
    {
        fn dyn_no_alias<'r>(
            &self,
            row: &'r <S as Database>::Row,
        ) -> Result<Box<dyn Any + Send>, FromRowError> {
            Ok(Box::new(self.no_alias(row)?))
        }

        fn dyn_str_alias<'r>(
            &self,
            row: RowStrAliased<'r, <S as Database>::Row>,
        ) -> Result<Box<dyn Any + Send>, FromRowError> {
            Ok(Box::new(self.str_alias(row)?))
        }

        fn dyn_num_alias<'r>(
            &self,
            row: RowNumAliased<'r, <S as Database>::Row>,
        ) -> Result<Box<dyn Any + Send>, FromRowError> {
            Ok(Box::new(self.num_alias(row)?))
        }
    }

    impl<'b, S> FromRowData for Box<dyn RawFromRow<S> + Send + 'b> {
        type RData = Box<dyn Any + Send>;
    }

    impl<'b, 'r, S: Database> FromRowAlias<'r, <S as Database>::Row>
        for Box<dyn RawFromRow<S> + Send + 'b>
    {
        fn no_alias(&self, row: &'r S::Row) -> Result<Self::RData, FromRowError> {
            (&**self).dyn_no_alias(row)
        }

        fn str_alias(&self, row: RowStrAliased<'r, S::Row>) -> Result<Self::RData, FromRowError>
        where
            S::Row: sqlx::prelude::Row,
        {
            (&**self).dyn_str_alias(row)
        }

        fn num_alias(&self, row: RowNumAliased<'r, S::Row>) -> Result<Self::RData, FromRowError>
        where
            S::Row: sqlx::prelude::Row,
        {
            (&**self).dyn_num_alias(row)
        }
    }
}

pub mod dynamic_input {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        database_extention::DatabaseExt,
        json_client::{ToBind, dynamic_collection::DynamicInsertInput},
        operations::operations_expressions_crossover::{NamedBind, SelfPrescribedInsert},
        sqlx_query_builder::Join,
        tuple_trait::AsTuple,
    };

    impl<S> AsTuple for DynamicInsertInput<S> {
        const IS_STRUCT: bool = false;
        const NAMES: &'static [&'static str] = &[];
        type Tuple = ();
        fn into_tuple(self) -> Self::Tuple {}
        fn from_tuple(_: Self::Tuple) -> Self {
            HashMap::new()
        }
    }

    pub struct NamedBindList<S>(pub Vec<NamedBind<Arc<str>, Arc<str>, Box<dyn ToBind<S> + Send>>>);

    impl<S> SelfPrescribedInsert for NamedBindList<S>
    where
        S: DatabaseExt,
    {
        type InsertId = Vec<
            <NamedBind<Arc<str>, Arc<str>, Box<dyn ToBind<S> + Send>> as SelfPrescribedInsert>::InsertId,
        >;
        type InsertValue = Vec<
            <NamedBind<Arc<str>, Arc<str>, Box<dyn ToBind<S> + Send>> as SelfPrescribedInsert>::InsertValue,
        >;
        type UpdateSets = Vec<
            <NamedBind<Arc<str>, Arc<str>, Box<dyn ToBind<S> + Send>> as SelfPrescribedInsert>::UpdateSets,
        >;

        fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
            let mut ids = Vec::with_capacity(self.0.len());
            let mut vals = Vec::with_capacity(self.0.len());
            for each in self.0 {
                let (id, val) = each.on_insert();
                ids.push(id);
                vals.push(val);
            }
            (ids, vals)
        }

        fn on_update(self) -> Self::UpdateSets {
            self.0.into_iter().map(|each| each.on_update()).collect()
        }
    }
}

pub mod insert_sets {
    use crate::{
        database_extention::DatabaseExt,
        operations::operations_expressions_crossover::{IdentifierOnly, SelfPrescribedInsert},
        sqlx_query_builder::{
            Expression, Join,
            combinators::OptionalExpression,
            trait_objects::{box_expression, BoxedExpression},
        },
    };

    pub trait DynUpdateSets<S: DatabaseExt>: Send {
        fn dyn_on_update(self: Box<Self>) -> Box<dyn BoxedExpression<S> + Send>;
    }

    impl<T, S> DynUpdateSets<S> for T
    where
        S: DatabaseExt,
        T: SelfPrescribedInsert + Send + 'static,
        T::UpdateSets: OptionalExpression + Send + 'static,
        for<'e> Join<T::UpdateSets>: Expression<'e, S>,
    {
        fn dyn_on_update(self: Box<Self>) -> Box<dyn BoxedExpression<S> + Send> {
            box_expression(self.on_update(), ", ")
        }
    }

    pub struct AggregatedUpdateSets<S>(pub Vec<Box<dyn DynUpdateSets<S> + Send>>);

    impl<S> SelfPrescribedInsert for AggregatedUpdateSets<S>
    where
        S: DatabaseExt,
    {
        type InsertId = ();
        type InsertValue = ();
        fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
            ((), ())
        }
        type UpdateSets = Vec<Box<dyn BoxedExpression<S> + Send>>;
        fn on_update(self) -> Self::UpdateSets {
            self.0.into_iter().map(|set| set.dyn_on_update()).collect()
        }
    }

    pub trait DynSelfPrescribedInsert<S: DatabaseExt>: Send {
        fn dyn_on_insert(
            self: Box<Self>,
        ) -> (
            Box<dyn BoxedExpression<S> + Send>,
            Box<dyn BoxedExpression<S> + Send>,
        );
    }

    impl<T, S> DynSelfPrescribedInsert<S> for T
    where
        S: DatabaseExt,
        T: SelfPrescribedInsert + Send + 'static,
        T::InsertId: OptionalExpression + Send + 'static,
        T::InsertValue: OptionalExpression + Send + 'static,
        for<'e> Join<T::InsertId>: Expression<'e, S>,
        for<'e> Join<T::InsertValue>: Expression<'e, S>,
    {
        fn dyn_on_insert(
            self: Box<Self>,
        ) -> (
            Box<dyn BoxedExpression<S> + Send>,
            Box<dyn BoxedExpression<S> + Send>,
        ) {
            let (id, val) = self.on_insert();
            (box_expression(id, ", "), box_expression(val, ", "))
        }
    }

    pub struct AggregatedInsertSets<S>(pub Vec<Box<dyn DynSelfPrescribedInsert<S> + Send>>);

    impl<S> SelfPrescribedInsert for AggregatedInsertSets<S>
    where
        S: DatabaseExt,
    {
        type InsertId = Vec<Box<dyn BoxedExpression<S> + Send>>;
        type InsertValue = Vec<Box<dyn BoxedExpression<S> + Send>>;
        type UpdateSets = ();

        fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
            let mut ids = Vec::with_capacity(self.0.len());
            let mut vals = Vec::with_capacity(self.0.len());
            for set in self.0 {
                let (id, val) = set.dyn_on_insert();
                ids.push(id);
                vals.push(val);
            }
            (ids, vals)
        }

        fn on_update(self) -> Self::UpdateSets {}
    }

    pub trait DynIdentifierOnly<S: DatabaseExt>: Send {
        fn dyn_identifier_only(&self) -> Box<dyn BoxedExpression<S> + Send>;
    }

    impl<T, S> DynIdentifierOnly<S> for T
    where
        S: DatabaseExt,
        T: IdentifierOnly + Send + 'static,
        T::Identifier: OptionalExpression + Send + 'static,
        for<'e> Join<T::Identifier>: Expression<'e, S>,
    {
        fn dyn_identifier_only(&self) -> Box<dyn BoxedExpression<S> + Send> {
            box_expression(self.identifier_only(), ", ")
        }
    }

    pub struct AggregatedInsertReturning<S>(pub Vec<Box<dyn DynIdentifierOnly<S> + Send>>);

    impl<S> IdentifierOnly for AggregatedInsertReturning<S>
    where
        S: DatabaseExt,
    {
        type Identifier = Vec<Box<dyn BoxedExpression<S> + Send>>;
        fn identifier_only(&self) -> Self::Identifier {
            self
                .0
                .iter()
                .map(|item| item.dyn_identifier_only())
                .collect()
        }
    }
}

pub mod filters {
    use std::sync::Arc;

    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{
            Expression, OpExpression, StatementBuilder, trait_objects::BoxedExpression,
        },
    };

    pub struct FilterAnd<S>(pub Vec<Box<dyn BoxedExpression<S> + Send>>);
    pub struct FilterOr<S>(pub Vec<Box<dyn BoxedExpression<S> + Send>>);

    pub struct ScopedIsNull {
        pub table: Arc<str>,
        pub col: Arc<str>,
        pub is_not: bool,
    }

    impl OpExpression for ScopedIsNull {
    }

    impl<'q, S> Expression<'q, S> for ScopedIsNull
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            ctx.sanitize(self.table.as_ref());
            ctx.syntax(".");
            ctx.sanitize(self.col.as_ref());
            if self.is_not {
                ctx.syntax(" IS NOT NULL");
            } else {
                ctx.syntax(" IS NULL");
            }
        }
    }

    impl<S> OpExpression for FilterAnd<S>
    where
        S: DatabaseExt,
    {
    }
    impl<S> OpExpression for FilterOr<S>
    where
        S: DatabaseExt,
    {
    }

    impl<'q, S> Expression<'q, S> for FilterAnd<S>
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            if self.0.is_empty() {
                return;
            }
            ctx.syntax("(");
            for (i, expr) in self.0.into_iter().enumerate() {
                if i > 0 {
                    ctx.syntax(" AND ");
                }
                Expression::expression(expr, ctx);
            }
            ctx.syntax(")");
        }
    }

    impl<'q, S> Expression<'q, S> for FilterOr<S>
    where
        S: DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
            if self.0.is_empty() {
                return;
            }
            ctx.syntax("(");
            for (i, expr) in self.0.into_iter().enumerate() {
                if i > 0 {
                    ctx.syntax(" OR ");
                }
                Expression::expression(expr, ctx);
            }
            ctx.syntax(")");
        }
    }
}

pub mod delete_pre_op {
    use std::{any::Any, sync::Arc};

    use sqlx::Database;

    use crate::{
        database_extention::DatabaseExt,
        json_client::dynamic_collection::DynamicCollection,
        links::{
            DefaultRelationKey, relation_many_to_many::DeleteManyToManyLinked,
            relation_one_to_many::OneToMany,
        },
        operations::{
            Operation,
            boxed_operation::BoxedOperation,
            delete::{DeleteLink, DeleteLinkPreOp},
        },
        sqlx_query_builder::basic_expressions::{Bind, ColumnEqual, ScopedColumn},
    };

    pub type JsonDeleteWheres = ColumnEqual<ScopedColumn<Arc<str>, &'static str>, Bind<i64>>;

    pub trait ErasedDeletePreOp<S: Database>: DeleteLink + Send + Sync {
        fn erased_pre_op(
            &self,
            init: Box<dyn Any + Send>,
            wheres: &dyn Any,
        ) -> Box<dyn BoxedOperation<S> + Send>;
    }

    impl<S> ErasedDeletePreOp<S>
        for OneToMany<DefaultRelationKey, Arc<DynamicCollection<S>>, Arc<DynamicCollection<S>>>
    where
        S: Database + DatabaseExt + Sync,
        Self: DeleteLinkPreOp<JsonDeleteWheres>,
        <Self as DeleteLinkPreOp<JsonDeleteWheres>>::PreOp: Operation<S> + 'static,
    {
        fn erased_pre_op(
            &self,
            init: Box<dyn Any + Send>,
            wheres: &dyn Any,
        ) -> Box<dyn BoxedOperation<S> + Send> {
            let init = init
                .downcast::<<Self as DeleteLinkPreOp<JsonDeleteWheres>>::InitSplitForPreOp>()
                .unwrap();
            let wheres = wheres.downcast_ref::<JsonDeleteWheres>().unwrap();
            Box::new(self.pre_op(*init, wheres))
        }
    }

    impl<S> ErasedDeletePreOp<S>
        for DeleteManyToManyLinked<
            DefaultRelationKey,
            Arc<DynamicCollection<S>>,
            Arc<DynamicCollection<S>>,
        >
    where
        S: Database + DatabaseExt + Sync,
        Self: DeleteLinkPreOp<JsonDeleteWheres>,
        <Self as DeleteLinkPreOp<JsonDeleteWheres>>::PreOp: Operation<S> + 'static,
    {
        fn erased_pre_op(
            &self,
            init: Box<dyn Any + Send>,
            wheres: &dyn Any,
        ) -> Box<dyn BoxedOperation<S> + Send> {
            let init = init
                .downcast::<<Self as DeleteLinkPreOp<JsonDeleteWheres>>::InitSplitForPreOp>()
                .unwrap();
            let wheres = wheres.downcast_ref::<JsonDeleteWheres>().unwrap();
            Box::new(self.pre_op(*init, wheres))
        }
    }
}
