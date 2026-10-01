use sqlx::Database;

pub mod compose_operation;
// pub mod delete;
pub mod fetch_many;
#[cfg(not(feature = "in_dev_op2"))]
pub mod fetch_one;
#[cfg(feature = "in_dev_op2")]
#[path = "fetch_one_v2.rs"]
pub mod fetch_one;
// pub mod insert;
// pub mod update;

pub trait OperationOutput {
    type Output;
}
pub trait Operation<S>: OperationOutput<Output: Send> + Send {
    fn exec_operation(self, pool: &mut S::Connection) -> impl Future<Output = Self::Output> + Send
    where
        S: Database,
        Self: Sized;
}

#[cfg(feature = "in_dev_op2")]
pub trait StreamedOperation<S>: OperationOutput<Output: Send> + Send {
    fn stream_operation(
        self,
        conn: &mut S::Connection,
    ) -> impl futures::Stream<Item = Self::Output> + Send
    where
        Self: Sized,
        S: Database;
}

#[cfg(feature = "in_dev_op2")]
pub mod streamed_ops {
    use std::{collections::HashMap, hash::Hash, pin::pin};

    use futures::StreamExt;

    use crate::operations::{Operation, OperationOutput, StreamedOperation};

    pub struct OperationVec<T>(pub T);

    impl<T> OperationOutput for OperationVec<T>
    where
        T: OperationOutput,
    {
        type Output = Vec<T::Output>;
    }

    impl<S, T> Operation<S> for OperationVec<T>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            Self: Sized,
            S: sqlx::Database,
        {
            async move {
                let mut stream = pin!(self.0.stream_operation(conn));
                let mut output = Vec::new();
                while let Some(item) = stream.next().await {
                    output.push(item);
                }
                output
            }
        }
    }

    pub struct OperationHashed<T>(pub T);

    impl<T, Key, Value> OperationOutput for OperationHashed<T>
    where
        T: OperationOutput<Output = (Key, Value)>,
    {
        type Output = HashMap<Key, Vec<Value>>;
    }

    impl<S, T, Key, Value> Operation<S> for OperationHashed<T>
    where
        T: OperationOutput<Output = (Key, Value)>,
        T: StreamedOperation<S>,
        Value: Send,
        Key: Send + Eq + Hash,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            Self: Sized,
            S: sqlx::Database,
        {
            async move {
                let mut stream = pin!(self.0.stream_operation(conn));
                let mut output = HashMap::new();
                while let Some((fi, ti)) = stream.next().await {
                    output.entry(fi).or_insert_with(|| Vec::new()).push(ti)
                }
                output
            }
        }
    }

    pub struct PanicOnFailure;
    pub struct ResultOnFailure;

    pub struct OneRecordOperation<T, Infalibility> {
        pub operation: T,
        pub infalibility: Infalibility,
    }

    impl<T> OperationOutput for OneRecordOperation<T, PanicOnFailure>
    where
        T: OperationOutput,
    {
        type Output = Option<T::Output>;
    }

    impl<S, T> Operation<S> for OneRecordOperation<T, PanicOnFailure>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
        S: sqlx::Database,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let mut stream = pin!(self.operation.stream_operation(conn));

                let output = stream.next().await;

                if stream.next().await.is_some() {
                    panic!("expected only one record, but got multiple");
                }

                output
            }
        }
    }

    impl<T> OperationOutput for OneRecordOperation<T, ResultOnFailure>
    where
        T: OperationOutput,
    {
        type Output = Result<Option<T::Output>, ()>;
    }

    impl<S, T> Operation<S> for OneRecordOperation<T, ResultOnFailure>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
        S: sqlx::Database,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let mut stream = pin!(self.operation.stream_operation(conn));

                let output = stream.next().await;

                if stream.next().await.is_some() {
                    return Err(());
                }

                Ok(output)
            }
        }
    }
}

pub mod insert_id_mode {
    pub struct AutoGenerate;
    pub struct Manual<T>(pub T);
}

// valid_syntax would replace this module
#[cfg(not(feature = "in_dev_op2"))]
pub mod operations_expressions_crossover {
    pub trait ExpressionsForOperation {
        type Identifier;
        fn identifier(&self) -> Self::Identifier;

        type Scoped;
        fn scoped(&self) -> Self::Scoped;

        type ScopedAliased;
        fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased;

        type NumScopedAliased;
        fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased;
    }

    pub trait IdentifierOnly {
        type Identifier;
        fn identifier_only(&self) -> Self::Identifier;
    }

    pub trait IdentifierColNames {
        fn col_names(self) -> Vec<String>;
    }

    impl<const N: usize> IdentifierColNames for [&'static str; N] {
        fn col_names(self) -> Vec<String> {
            self.iter().map(|col| (*col).to_string()).collect()
        }
    }

    impl<T> IdentifierOnly for T
    where
        T: ExpressionsForOperation,
    {
        type Identifier = T::Identifier;
        fn identifier_only(&self) -> Self::Identifier {
            self.identifier()
        }
    }

    pub trait OnInsert<Input>: ExpressionsForOperation {
        type InsertExpression;
        fn on_insert(&self, input: Input) -> Self::InsertExpression;
        type InsertId;
        fn on_insert_with_id(&self, input: Input) -> (Self::InsertId, Self::InsertExpression);
    }

    pub trait SelfPrescribedInsert {
        type InsertValue;
        type InsertId;
        fn on_insert(self) -> (Self::InsertId, Self::InsertValue);
        type UpdateSets;
        fn on_update(self) -> Self::UpdateSets;
    }

    pub trait OnUpdate<Input>: ExpressionsForOperation {
        type UpdateExpression;
        fn on_update(&self, input: Input) -> Self::UpdateExpression;
    }

    pub trait TableExpressions: ExpressionsForOperation {
        // usually to impl Expression + AsRef<str>
        type SnakeCase;
        type PascalCase;
        fn table_name_snake_case(&self) -> Self::SnakeCase;
        fn table_name_pascal_case(&self) -> Self::PascalCase;
        type InheritJoin;
        fn inherit_join(&self) -> Self::InheritJoin;
    }

    pub trait MigrateExpression {
        type Migrate;
        fn migrate(&self) -> Self::Migrate;
    }

    pub struct NamedBind<T, N, V> {
        pub table: T,
        pub name: N,
        pub value: V,
    }

    mod named_bind_impls {
        use super::{ExpressionsForOperation, NamedBind, SelfPrescribedInsert, TableExpressions};
        use crate::sqlx_query_builder::basic_expressions::{Bind, ScopedColumn, UpdatingColumn};

        impl<T, N, V> SelfPrescribedInsert for NamedBind<T, N, V>
        where
            T: TableExpressions,
            N: ExpressionsForOperation,
        {
            type InsertValue = Bind<V>;

            type InsertId = ScopedColumn<T::PascalCase, N::Identifier>;

            fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
                (
                    ScopedColumn {
                        table: self.table.table_name_pascal_case(),
                        col: self.name.identifier(),
                    },
                    Bind(self.value),
                )
            }

            type UpdateSets = UpdatingColumn<N::Identifier, Bind<V>>;

            fn on_update(self) -> Self::UpdateSets {
                UpdatingColumn {
                    col: self.name.identifier(),
                    set: Bind(self.value),
                }
            }
        }

        impl<V> SelfPrescribedInsert for NamedBind<std::sync::Arc<str>, std::sync::Arc<str>, V> {
            type InsertValue = Bind<V>;

            type InsertId = ScopedColumn<std::sync::Arc<str>, std::sync::Arc<str>>;

            fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
                (
                    ScopedColumn {
                        table: self.table,
                        col: self.name,
                    },
                    Bind(self.value),
                )
            }

            type UpdateSets = UpdatingColumn<std::sync::Arc<str>, Bind<V>>;

            fn on_update(self) -> Self::UpdateSets {
                UpdatingColumn {
                    col: self.name,
                    set: Bind(self.value),
                }
            }
        }
    }

    mod impl_for_identifier_only {
        use super::IdentifierOnly;
        use crate::sqlx_query_builder::sanitize_combinator::Sanitize;

        impl IdentifierOnly for String {
            type Identifier = String;
            fn identifier_only(&self) -> Self::Identifier {
                self.clone()
            }
        }

        impl IdentifierOnly for &'static str {
            type Identifier = &'static str;
            fn identifier_only(&self) -> Self::Identifier {
                *self
            }
        }

        impl IdentifierOnly for std::sync::Arc<str> {
            type Identifier = std::sync::Arc<str>;
            fn identifier_only(&self) -> Self::Identifier {
                self.clone()
            }
        }

        impl<T: Clone> IdentifierOnly for Sanitize<T> {
            type Identifier = Sanitize<T>;
            fn identifier_only(&self) -> Self::Identifier {
                Sanitize(self.0.clone())
            }
        }
    }

    mod std_impls {
        use super::{ExpressionsForOperation, SelfPrescribedInsert};

        impl ExpressionsForOperation for () {
            type Identifier = ();

            fn identifier(&self) -> Self::Identifier {}

            type Scoped = ();

            fn scoped(&self) -> Self::Scoped {}

            type ScopedAliased = ();

            fn scoped_aliased(&self, _: &'static str) -> Self::ScopedAliased {}

            type NumScopedAliased = ();

            fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {}
        }

        impl<T> ExpressionsForOperation for Vec<T>
        where
            T: ExpressionsForOperation,
        {
            type Identifier = Vec<T::Identifier>;
            fn identifier(&self) -> Self::Identifier {
                self.into_iter().map(|t| t.identifier()).collect()
            }

            type Scoped = Vec<T::Scoped>;

            fn scoped(&self) -> Self::Scoped {
                self.into_iter().map(|t| t.scoped()).collect()
            }

            type ScopedAliased = Vec<T::NumScopedAliased>;

            fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                self.into_iter()
                    .enumerate()
                    .map(|(num, t)| t.num_scoped_aliased(num, alias))
                    .collect()
            }

            type NumScopedAliased = Vec<T::NumScopedAliased>;

            fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {
                panic!("should not nest multiple links")
            }
        }

        impl SelfPrescribedInsert for () {
            type InsertValue = ();
            type InsertId = ();
            fn on_insert(self) -> (Self::InsertId, Self::InsertValue) {
                ((), ())
            }
            type UpdateSets = ();
            fn on_update(self) -> Self::UpdateSets {}
        }
    }

    mod impl_for_single_incremintal_int {
        use super::super::insert_id_mode::AutoGenerate;
        use super::{ExpressionsForOperation, OnInsert};
        use crate::{
            collections::SingleIncremintalInt,
            sqlx_query_builder::{
                Sanitize,
                basic_expressions::{AliasedScopedColumn, ScopedColumn},
            },
        };

        macro_rules! impl_expressions_for_operation {
            ($type:ty) => {
                impl ExpressionsForOperation for SingleIncremintalInt<$type> {
                    type Identifier = &'static str;
                    fn identifier(&self) -> Self::Identifier {
                        "id"
                    }

                    type Scoped = ScopedColumn<$type, &'static str>;

                    fn scoped(&self) -> Self::Scoped {
                        ScopedColumn {
                            table: self.0.clone(),
                            col: "id",
                        }
                    }

                    type ScopedAliased = AliasedScopedColumn<
                        $type,
                        &'static str,
                        Sanitize<(&'static str, &'static str)>,
                    >;

                    fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                        AliasedScopedColumn {
                            table: self.0.clone(),
                            column: "id",
                            alias: Sanitize((alias, "id")),
                        }
                    }

                    type NumScopedAliased = AliasedScopedColumn<
                        $type,
                        &'static str,
                        Sanitize<(&'static str, usize, &'static str)>,
                    >;

                    fn num_scoped_aliased(
                        &self,
                        num: usize,
                        alias: &'static str,
                    ) -> Self::NumScopedAliased {
                        AliasedScopedColumn {
                            table: self.0.clone(),
                            column: "id",
                            alias: Sanitize((alias, num, "id")),
                        }
                    }
                }
            };
        }

        impl_expressions_for_operation!(&'static str);
        impl_expressions_for_operation!(String);
        impl_expressions_for_operation!(std::sync::Arc<str>);

        impl<T: Clone> OnInsert<AutoGenerate> for SingleIncremintalInt<T>
        where
            SingleIncremintalInt<T>: ExpressionsForOperation,
        {
            type InsertExpression = ();

            fn on_insert(&self, _: AutoGenerate) -> Self::InsertExpression {}

            type InsertId = ();
            fn on_insert_with_id(
                &self,
                _: AutoGenerate,
            ) -> (Self::InsertId, Self::InsertExpression) {
                ((), ())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinkedOutput<Id, C, L> {
    pub id: Id,
    pub attributes: C,
    pub links: L,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollectionOutput<Id, C> {
    pub id: Id,
    pub attributes: C,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ManyLinkOutput<T> {
    pub many_output: Vec<T>,
}

impl<T> From<Vec<T>> for ManyLinkOutput<T> {
    fn from(many_output: Vec<T>) -> Self {
        Self { many_output }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdOutput<Id> {
    pub id: Id,
}

impl<I, C, L> From<LinkedOutput<I, C, L>> for CollectionOutput<I, C> {
    fn from(value: LinkedOutput<I, C, L>) -> Self {
        CollectionOutput {
            id: value.id,
            attributes: value.attributes,
        }
    }
}

impl<I, C, L> From<LinkedOutput<I, C, L>> for IdOutput<I> {
    fn from(value: LinkedOutput<I, C, L>) -> Self {
        IdOutput { id: value.id }
    }
}

impl<I, C> From<CollectionOutput<I, C>> for IdOutput<I> {
    fn from(value: CollectionOutput<I, C>) -> Self {
        IdOutput { id: value.id }
    }
}

// OneRecordOperation would deprecate this module
#[cfg(not(feature = "in_dev_op2"))]
pub mod by_id {
    use super::{
        Operation, OperationOutput, delete::Delete,
        operations_expressions_crossover::ExpressionsForOperation, update::Update,
    };
    use crate::{
        collections::{Collection, CollectionId},
        sqlx_query_builder::{
            Join,
            basic_expressions::{Bind, ColumnEqual},
            combinators::Nest,
        },
    };

    #[allow(type_alias_bounds)]
    #[doc(hidden)]
    pub type ExtendExpressionByIdEqualTo<W, C>
    where
        C: Collection,
        C::Id: ExpressionsForOperation,
    = Join<(
        Nest<
            ColumnEqual<
                <C::Id as ExpressionsForOperation>::Scoped,
                Bind<<C::Id as CollectionId>::IdData>,
            >,
        >,
        Nest<W>,
    )>;

    #[doc(hidden)]
    pub fn extend_expression_by_id_equal_to<W, C>(
        wheres: W,
        base: &C,
        id: <C::Id as CollectionId>::IdData,
    ) -> ExtendExpressionByIdEqualTo<W, C>
    where
        C: Collection,
        C::Id: ExpressionsForOperation,
    {
        Join {
            start: "",
            separator: " AND ",
            items: (
                Nest(ColumnEqual {
                    col: base.id().scoped(),
                    eq: Bind(id),
                }),
                Nest(wheres),
            ),
        }
    }

    pub struct OperationById<OgOperation, Id> {
        pub operation: OgOperation,
        pub id: Id,
    }

    pub trait ExtendById<Id> {
        type TransformedOperation;
        fn transform_operation(self, id: Id) -> Self::TransformedOperation;
    }

    impl<Base, Partial, Wheres, Links, Infalibility> ExtendById<<Base::Id as CollectionId>::IdData>
        for Update<Base, Partial, Wheres, Links, Infalibility>
    where
        Base: Collection,
        Base::Id: ExpressionsForOperation,
    {
        type TransformedOperation =
            Update<Base, Partial, ExtendExpressionByIdEqualTo<Wheres, Base>, Links, Infalibility>;

        fn transform_operation(
            self,
            id: <Base::Id as CollectionId>::IdData,
        ) -> Self::TransformedOperation {
            Update {
                wheres: extend_expression_by_id_equal_to(self.wheres, &self.base, id),
                base: self.base,
                partial: self.partial,
                links: self.links,
                infalibility: self.infalibility,
            }
        }
    }

    impl<Base, Wheres, Links> ExtendById<<Base::Id as CollectionId>::IdData>
        for Delete<Base, Wheres, Links>
    where
        Base: Collection,
        Base::Id: ExpressionsForOperation,
    {
        type TransformedOperation = Delete<Base, ExtendExpressionByIdEqualTo<Wheres, Base>, Links>;
        fn transform_operation(
            self,
            id: <Base::Id as CollectionId>::IdData,
        ) -> Self::TransformedOperation {
            Delete {
                wheres: extend_expression_by_id_equal_to(self.wheres, &self.base, id),
                base: self.base,
                links: self.links,
            }
        }
    }

    impl<V, OgOperation, Id> OperationOutput for OperationById<OgOperation, Id>
    where
        OgOperation: OperationOutput<Output = Vec<V>>,
        OgOperation::TransformedOperation: OperationOutput<Output = Vec<V>>,
        OgOperation: ExtendById<Id>,
    {
        type Output = Option<V>;
    }

    impl<S, V, OgOperation, Id> Operation<S> for OperationById<OgOperation, Id>
    where
        OgOperation: OperationOutput<Output = Vec<V>>,
        OgOperation::TransformedOperation: Operation<S, Output = Vec<V>>,
        OgOperation: ExtendById<Id>,
        V: Send,
        Id: Send,
        OgOperation: Send,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let mut vec_output = self
                    .operation
                    .transform_operation(self.id)
                    .exec_operation(pool)
                    .await;

                let last = vec_output.pop();

                if vec_output.len() != 0 {
                    panic!("made an operation on multiple records!")
                }

                last
            }
        }
    }

    pub struct DeleteById<Base: Collection, Wheres, Links> {
        pub base: Base,
        pub id: <Base::Id as CollectionId>::IdData,
        pub wheres: Wheres,
        pub links: Links,
    }

    impl<T, Base: Collection, Wheres, Links> OperationOutput for DeleteById<Base, Wheres, Links>
    where
        OperationById<Delete<Base, Wheres, Links>, <Base::Id as CollectionId>::IdData>:
            OperationOutput<Output = Option<T>>,
    {
        type Output = Option<T>;
    }

    impl<S, T, Base, Wheres, Links> Operation<S> for DeleteById<Base, Wheres, Links>
    where
        Base: Send + Collection,
        Wheres: Send,
        Links: Send,
        T: Send,
        OperationById<Delete<Base, Wheres, Links>, <Base::Id as CollectionId>::IdData>:
            Operation<S, Output = Option<T>>,
        <Base::Id as CollectionId>::IdData: Send,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                OperationById {
                    operation: Delete {
                        base: self.base,
                        wheres: self.wheres,
                        links: self.links,
                    },
                    id: self.id,
                }
                .exec_operation(pool)
                .await
            }
        }
    }

    pub struct UpdateById<Base: Collection, Partial, Wheres, Links, Infalibility> {
        pub base: Base,
        pub id: <Base::Id as CollectionId>::IdData,
        pub partial: Partial,
        pub wheres: Wheres,
        pub links: Links,
        pub infalibility: Infalibility,
    }

    impl<T, Base: Collection, Partial, Wheres, Links, Infalibility> OperationOutput
        for UpdateById<Base, Partial, Wheres, Links, Infalibility>
    where
        Update<Base, Partial, ExtendExpressionByIdEqualTo<Wheres, Base>, Links, Infalibility>:
            OperationOutput<Output = Vec<T>>,
        Base::Id: ExpressionsForOperation,
    {
        type Output = Option<T>;
    }

    impl<S, T, Base, Partial, Wheres, Links, Infalibility> Operation<S>
        for UpdateById<Base, Partial, Wheres, Links, Infalibility>
    where
        Infalibility: Send,
        Base: Send + Collection,
        Partial: Send,
        Wheres: Send,
        Links: Send,
        T: Send,
        Update<Base, Partial, ExtendExpressionByIdEqualTo<Wheres, Base>, Links, Infalibility>:
            Operation<S, Output = Vec<T>>,
        <Base::Id as CollectionId>::IdData: Send,
        Base::Id: ExpressionsForOperation,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let mut result = Update {
                    wheres: extend_expression_by_id_equal_to(self.wheres, &self.base, self.id),
                    base: self.base,
                    partial: self.partial,
                    links: self.links,
                    infalibility: self.infalibility,
                }
                .exec_operation(pool)
                .await;

                let last = result.pop();

                if result.len() != 0 {
                    panic!("made an operation on multiple records!")
                }

                last
            }
        }
    }
    #[cfg(test)]
    mod test_update {
        use super::super::insert::AbortOperation;
        use super::{Operation, UpdateById};
        use crate::connect_in_memory::ConnectInMemory;
        use crate::test_module::TodoHandler;
        use crate::test_module::TodoPartial;
        use crate::update_mod::Update;
        use sqlx::Row;
        use sqlx::Sqlite;

        #[tokio::test]
        async fn update_by_id() {
            let mut pool = Sqlite::in_memory_connection().await;

            sqlx::query(
                "
            CREATE TABLE IF NOT EXISTS Todo (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                done BOOLEAN NOT NULL,
                description TEXT
            );

            INSERT INTO Todo (id, title, done, description) VALUES 
                (1, 'first_todo', true, 'description_1'),
                (2, 'second_todo', false, NULL),
                (3, 'third_todo', true, 'description_3');

            ",
            )
            .execute(&mut pool)
            .await
            .unwrap();

            Operation::<Sqlite>::exec_operation(
                UpdateById {
                    base: TodoHandler,
                    id: 2,
                    partial: TodoPartial {
                        title: Update::Set("new_title".to_string()),
                        done: Update::Keep,
                        description: Update::Keep,
                    },
                    wheres: (),
                    links: (),
                    infalibility: AbortOperation,
                },
                &mut pool,
            )
            .await
            .unwrap();

            let rows = sqlx::query("SELECT id, title FROM Todo; ")
                .fetch_all(&mut pool)
                .await
                .unwrap();

            if rows.len() != 3 {
                panic!("did not update one row");
            };

            let first: String = rows[0].get("title");
            let second: String = rows[1].get("title");
            let third: String = rows[2].get("title");

            pretty_assertions::assert_eq!(first, "first_todo".to_string());
            pretty_assertions::assert_eq!(second, "new_title".to_string());
            pretty_assertions::assert_eq!(third, "third_todo".to_string());
        }
    }

    #[cfg(test)]
    mod test {
        use super::{DeleteById, Operation};
        use crate::{connect_in_memory::ConnectInMemory, test_module::TodoHandler};
        use sqlx::{Row, Sqlite};

        #[tokio::test]
        async fn delete_by_id() {
            let mut pool = Sqlite::in_memory_connection().await;

            sqlx::query(
                "
            CREATE TABLE IF NOT EXISTS Todo (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                done BOOLEAN NOT NULL,
                description TEXT
            );

            INSERT INTO Todo (id, title, done, description) VALUES 
                (1, 'first_todo', true, 'description_1'),
                (2, 'second_todo', false, NULL),
                (3, 'third_todo', true, 'description_3');

            ",
            )
            .execute(&mut pool)
            .await
            .unwrap();

            Operation::<Sqlite>::exec_operation(
                DeleteById {
                    base: TodoHandler,
                    id: 2,
                    links: (),
                    wheres: (),
                },
                &mut pool,
            )
            .await
            .unwrap();

            let rows = sqlx::query("SELECT id, title FROM Todo; ")
                .fetch_all(&mut pool)
                .await
                .unwrap();

            if rows.len() != 2 {
                panic!("did not delete one row");
            };

            let first: String = rows[0].get("title");
            let third: String = rows[1].get("title");

            pretty_assertions::assert_eq!(first, "first_todo".to_string());
            pretty_assertions::assert_eq!(third, "third_todo".to_string());
        }
    }
}

mod gen_serde_impls {
    use super::{LinkedOutput, ManyLinkOutput};
    use crate::gen_serde::{ObjectEncoding, Serialize};

    impl<F, I, C, L> Serialize<F> for LinkedOutput<I, C, L>
    where
        I: Serialize<F>,
        C: Serialize<F>,
        L: Serialize<F>,
        str: Serialize<F>,
        F: ObjectEncoding,
    {
        fn serialize(&self, ctx: &mut F) {
            let mut object = ctx.serialize_start();
            ctx.serialize_pair(&mut object, "id", &self.id);
            ctx.serialize_pair(&mut object, "attributes", &self.attributes);
            ctx.serialize_pair(&mut object, "links", &self.links);
            ctx.serialize_end(object);
        }
    }

    impl<F, T> Serialize<F> for ManyLinkOutput<T>
    where
        F: ObjectEncoding,
        str: Serialize<F>,
        Vec<T>: Serialize<F>,
    {
        fn serialize(&self, ctx: &mut F) {
            let mut object = ctx.serialize_start();
            ctx.serialize_pair(&mut object, "many_output", &self.many_output);
            ctx.serialize_end(object);
        }
    }
}

// operations::streamed_ops::OneRecordOperation would replace this module
#[cfg(not(feature = "in_dev_op2"))]
pub mod on_one_record {
    use super::{Operation, OperationOutput};

    pub struct OnOneRecord<Operation> {
        pub operation: Operation,
    }

    impl<V, Op: OperationOutput<Output = Vec<V>>> OperationOutput for OnOneRecord<Op> {
        type Output = Option<V>;
    }

    impl<V, S, Op> Operation<S> for OnOneRecord<Op>
    where
        V: Send,
        Op: Operation<S, Output = Vec<V>>,
    {
        fn exec_operation(
            self,
            pool: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: sqlx::Database,
            Self: Sized,
        {
            async move {
                let mut res = self.operation.exec_operation(pool).await;

                let last = res.pop();

                if res.len() != 0 {
                    panic!("made an operation on multiple records!")
                }

                return last;
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::super::delete::Delete;
        use super::{OnOneRecord, Operation};
        use crate::{
            connect_in_memory::ConnectInMemory,
            sqlx_query_builder::basic_expressions::{Bind, ColumnEqual, ScopedColumn},
            test_module::TodoHandler,
        };
        use sqlx::{Connection, Sqlite};

        #[tokio::test]
        async fn on_one_record() {
            let mut conn = Sqlite::in_memory_connection().await;

            sqlx::query(
                "
                CREATE TABLE IF NOT EXISTS Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT
                );

                INSERT INTO Todo (id, title, done, description) VALUES 
                    (1, 'first_todo', true, 'description_1'),
                    (2, 'second_todo', false, NULL),
                    (3, 'third_todo', true, 'description_3');
            ",
            )
            .execute(&mut conn)
            .await
            .unwrap();

            let mut tx = conn.begin().await.unwrap();

            Operation::<Sqlite>::exec_operation(
                OnOneRecord {
                    operation: Delete {
                        base: TodoHandler,
                        links: (),
                        wheres: ColumnEqual {
                            col: ScopedColumn {
                                table: "Todo",
                                col: "id",
                            },
                            eq: Bind(2),
                        },
                    },
                },
                tx.as_mut(),
            )
            .await
            .unwrap();

            tx.commit().await.unwrap();
        }
    }
}

mod std_impls {
    use super::{Operation, OperationOutput};
    use sqlx::Database;

    impl<T> OperationOutput for Vec<T>
    where
        T: OperationOutput,
    {
        type Output = Vec<T::Output>;
    }

    impl<S, T> Operation<S> for Vec<T>
    where
        T: Operation<S, Output: Send> + Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output
        where
            S: sqlx::Database,
        {
            let mut v = vec![];
            for each in self {
                v.push(each.exec_operation(pool).await);
            }
            v
        }
    }

    impl OperationOutput for () {
        type Output = ();
    }

    impl<S: Database> Operation<S> for () {
        async fn exec_operation(self, _: &mut S::Connection) -> Self::Output {
            ()
        }
    }

    impl<T: OperationOutput> OperationOutput for Option<T> {
        type Output = Option<T::Output>;
    }

    impl<S, T> Operation<S> for Option<T>
    where
        T: Operation<S, Output: Send> + Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output
        where
            S: sqlx::Database,
        {
            match self {
                Some(operation) => Some(operation.exec_operation(pool).await),
                None => None,
            }
        }
    }
}

pub mod boxed_operation {
    use super::{Operation, OperationOutput};
    use futures::{Future, FutureExt};
    use sqlx::Database;
    use std::{any::Any, pin::Pin};

    pub trait BoxedOperation<S: Database>: Send + Any {
        fn exec_boxed<'c>(
            self: Box<Self>,
            pool: &'c mut S::Connection,
        ) -> Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'c>>;
    }

    impl<T, S> BoxedOperation<S> for T
    where
        T: Send + Operation<S> + 'static,
        S: Database,
    {
        fn exec_boxed<'c>(
            self: Box<Self>,
            pool: &'c mut S::Connection,
        ) -> Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'c>> {
            Box::pin(async {
                Operation::exec_operation(*self, pool)
                    .map(|f| Box::new(f) as Box<dyn Any + Send>)
                    .await
            })
        }
    }

    impl<S> OperationOutput for Box<dyn BoxedOperation<S> + Send> {
        type Output = Box<dyn Any + Send>;
    }

    impl<S> Operation<S> for Box<dyn BoxedOperation<S> + Send> {
        fn exec_operation(
            self,
            pool: &mut S::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            S: Database,
        {
            BoxedOperation::exec_boxed(self, pool)
        }
    }
}

pub mod execute_expression {
    use super::{Operation, OperationOutput};
    use crate::database_extention::DatabaseExt;
    use crate::execute::Executable;
    use crate::fix_executor::ExecutorTrait;
    use crate::sqlx_query_builder::{Expression, StatementBuilder};
    use sqlx::Database;

    pub struct ExpressionAsOperation<E>(pub E);

    impl<E: Send> OperationOutput for ExpressionAsOperation<E> {
        type Output = ();
    }

    impl<S, E> Operation<S> for ExpressionAsOperation<E>
    where
        E: for<'q> Expression<'q, S>,
        E: Send,
        S: DatabaseExt,
        S: ExecutorTrait,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output
        where
            S: Database,
        {
            let mut qb = StatementBuilder::default();
            self.0.expression(&mut qb);

            let (stmt, arg) = qb.unwrap();
            S::execute(
                pool,
                Executable {
                    string: stmt.as_str(),
                    arguments: arg,
                },
            )
            .await
            .unwrap();
        }
    }
}

pub mod map_operation {
    use sqlx::Database;

    use super::{Operation, OperationOutput};

    pub struct MapOperation<Op, Result>
    where
        Op: OperationOutput,
    {
        pub operation: Op,
        pub map_fn: fn(<Op as OperationOutput>::Output) -> Result,
    }

    impl<Operation, Result> OperationOutput for MapOperation<Operation, Result>
    where
        Operation: OperationOutput,
    {
        type Output = Result;
    }

    impl<Op, Result, S> Operation<S> for MapOperation<Op, Result>
    where
        S: Database,
        Op: Operation<S, Output: Send> + Send,
        Result: Send,
    {
        async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
            let output = self.operation.exec_operation(pool).await;
            (self.map_fn)(output)
        }
    }
}
