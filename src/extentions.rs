/// this is a less stable trait
/// I was trying to avoid any heap allocation for types that don't need it
/// instead of `Vec<String>` I tried `impl Iterator<Item = &str>`
/// but I got brutally blocked by lifetime problems
///
/// will be replaced by `AsTuple` and `RelavantExpressions` traits
pub trait Members {
    fn members_names(&self) -> Vec<String>;
}

impl<T> Members for T
where
    T: crate::collections::Collection,
    T::InputData: crate::tuple_trait::AsTuple,
    // T: crate::struct_as_tuple::AsTuple,
{
    fn members_names(&self) -> Vec<String> {
        <T::InputData as crate::tuple_trait::AsTuple>::NAMES
            .iter()
            .map(|name| name.to_string())
            .collect()
    }
}

pub mod common_expressions {

    /// list identifier scoped to their table, ex: 'Todo.title'
    pub trait Scoped {
        type Scoped;
        fn scoped(&self) -> Self::Scoped;
    }

    impl Scoped for () {
        type Scoped = ();
        fn scoped(&self) -> Self::Scoped {}
    }

    pub trait MembersAndIdAliased {
        type MembersAndIdAliased;
        fn members_and_id_aliased(&self, alias: &'static str) -> Self::MembersAndIdAliased;
    }

    impl MembersAndIdAliased for () {
        type MembersAndIdAliased = ();
        fn members_and_id_aliased(&self, _: &'static str) -> Self::MembersAndIdAliased {}
    }

    pub trait TableNameExpression {
        type TableNameExpression;
        fn table_name_expression(&self) -> Self::TableNameExpression;
        type LowerCaseTableNameExpression;
        fn lower_case_table_name_expression(&self) -> Self::LowerCaseTableNameExpression;
    }

    impl TableNameExpression for () {
        type TableNameExpression = ();
        fn table_name_expression(&self) -> Self::TableNameExpression {}
        type LowerCaseTableNameExpression = ();
        fn lower_case_table_name_expression(&self) -> Self::LowerCaseTableNameExpression {}
    }

    /// list identifier aliased, ex: 'Todo.title AS btitle'
    pub trait Aliased {
        type Aliased;
        fn aliased(&self, alias: &'static str) -> Self::Aliased;
        type NumAliased;
        fn num_aliased(&self, num: usize, alias: &'static str) -> Self::NumAliased;
    }

    /// This is one way to generate trait object from `FromRowAlias`
    /// but I think I will depricate it
    ///
    /// I have
    /// 1. `SelectItemsTraitObject` trait object
    ///     this generate trait object from `FromRowAlias` + `Aliased`
    /// 2. I want to create RawFromRow trait object
    ///     this generate trait object from ONLY `FromRowAlias`
    ///
    /// but this is like a mix of both, and I don't like it, it introduces problematic constraints
    pub mod dyn_from_row {
        use core::fmt;
        use std::any::{Any, type_name_of_val};

        use crate::{
            extentions::common_expressions::Aliased,
            from_row::{
                FromRowAlias, FromRowData, FromRowError, RowStrAliased,
                RowNumAliased,
            },
            sqlx_query_builder::{Expression, trait_objects::BoxedExpression},
        };

        pub trait DynFromRow<R> {
            fn clone_as_box(&self) -> Box<dyn DynFromRow<R> + Send>;
            fn no_alias_2<'r>(&self, row: &'r R) -> Result<Box<dyn Any + Send>, FromRowError>;
            fn str_alias_2<'r>(
                &self,
                row: RowStrAliased<'r, R>,
            ) -> Result<Box<dyn Any + Send>, FromRowError>
            where
                R: sqlx::Row;
            fn num_alias_2<'r>(
                &self,
                row: RowNumAliased<'r, R>,
            ) -> Result<Box<dyn Any + Send>, FromRowError>
            where
                R: sqlx::Row;
            fn str_aliased_2(
                &self,
                alias: &'static str,
            ) -> Box<dyn BoxedExpression<R::Database> + Send>
            where
                R: sqlx::Row;
        }

        impl<R, T> DynFromRow<R> for T
        where
            R: sqlx::Row,
            T: Clone + Send + 'static,
            T: for<'r> FromRowAlias<'r, R>,
            T::RData: 'static + Send,
            T::RData: fmt::Debug,
            T: Aliased<Aliased: Send + for<'q> Expression<'q, R::Database>>,
        {
            fn str_aliased_2(
                &self,
                alias: &'static str,
            ) -> Box<dyn BoxedExpression<<R>::Database> + Send>
            where
                R: sqlx::Row,
            {
                Box::new(self.aliased(alias))
            }
            fn clone_as_box(&self) -> Box<dyn DynFromRow<R> + Send> {
                Box::new(self.clone())
            }
            fn no_alias_2<'r>(&self, row: &'r R) -> Result<Box<dyn Any + Send>, FromRowError> {
                Ok(Box::new(self.no_alias(row)?))
            }
            fn str_alias_2<'r>(
                &self,
                row: RowStrAliased<'r, R>,
            ) -> Result<Box<dyn Any + Send>, FromRowError>
            where
                R: sqlx::Row,
            {
                // let type_name = std::any::type_name::<T::RData>();
                // panic!("type_name: {}", type_name);
                let s: T::RData = self.str_alias(row)?;
                assert_eq!(
                    "TypeId(0x63cc4f4b1487754c707f72f691c5c420)",
                    format!("{:?}", s.type_id())
                );

                println!(
                    "I'm this to Any\n type_id: {:?} \n type_name: {:?}",
                    s.type_id(),
                    type_name_of_val(&s)
                );

                Ok(Box::new(s))
            }
            fn num_alias_2<'r>(
                &self,
                row: RowNumAliased<'r, R>,
            ) -> Result<Box<dyn Any + Send>, FromRowError>
            where
                R: sqlx::Row,
            {
                Ok(Box::new(self.num_alias(row)?))
            }
        }

        impl<'r, R> FromRowData for Box<dyn DynFromRow<R> + Send> {
            type RData = Box<dyn Any + Send>;
        }

        impl<'r, R> FromRowAlias<'r, R> for Box<dyn DynFromRow<R> + Send> {
            fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
                Ok(Box::new(self.no_alias_2(row)?))
            }

            fn str_alias(
                &self,
                row: crate::from_row::RowStrAliased<'r, R>,
            ) -> Result<Self::RData, FromRowError>
            where
                R: sqlx::Row,
            {
                Ok(Box::new(self.str_alias_2(row)?))
            }

            fn num_alias(
                &self,
                row: crate::from_row::RowNumAliased<'r, R>,
            ) -> Result<Self::RData, FromRowError>
            where
                R: sqlx::Row,
            {
                Ok(Box::new(self.num_alias_2(row)?))
            }
        }
    }

    pub mod raw_from_row {
        use std::any::Any;

        use sqlx::Database;

        use crate::from_row::{
            FromRowAlias, FromRowData, FromRowError, RowStrAliased, RowNumAliased,
        };

        pub trait RawFromRow<S: Database> {
            fn dyn_no_alias<'r>(
                &self,
                row: &'r S::Row,
            ) -> Result<Box<dyn Any + Send>, FromRowError>;
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

    impl Aliased for () {
        type Aliased = ();
        fn aliased(&self, _: &'static str) -> Self::Aliased {}
        type NumAliased = ();
        fn num_aliased(&self, _: usize, _: &'static str) -> Self::NumAliased {}
    }

    /// list identifier, ex 'title'
    pub trait Identifier {
        type Identifier;
        fn identifier(&self) -> Self::Identifier;
    }

    impl Identifier for () {
        type Identifier = ();
        fn identifier(&self) -> Self::Identifier {}
    }

    impl<T> Identifier for Vec<T>
    where
        T: Identifier,
    {
        type Identifier = Vec<<T as Identifier>::Identifier>;
        fn identifier(&self) -> Self::Identifier {
            self.iter().map(|e| e.identifier()).collect()
        }
    }

    impl Identifier for String {
        type Identifier = String;
        fn identifier(&self) -> Self::Identifier {
            self.clone()
        }
    }

    impl<'q> Identifier for &'q str {
        type Identifier = &'q str;
        fn identifier(&self) -> Self::Identifier {
            self
        }
    }

    pub trait MigrateExpression {
        type MigrateExpression;
        fn migrate_expression(&self) -> Self::MigrateExpression;
    }

    /// important for operations that require runtime checks
    /// to be valid.
    pub trait V0OnInsert: Sized {
        type InsertInput;
        type InsertExpression;
        fn on_insert(self, input: Self::InsertInput) -> Self::InsertExpression;
    }

    impl V0OnInsert for () {
        type InsertInput = ();
        type InsertExpression = ();
        fn on_insert(self, _: Self::InsertInput) -> Self::InsertExpression {}
    }

    pub trait V0OnUpdate: Sized {
        type UpdateInput;
        type UpdateExpression;
        fn on_update(self, input: Self::UpdateInput) -> Self::UpdateExpression;
    }

    pub trait OnInsert {
        type InsertInput;
        type InsertExpression;
        fn on_insert(&self, input: Self::InsertInput) -> Self::InsertExpression;
    }

    impl OnInsert for () {
        type InsertInput = ();
        type InsertExpression = ();
        fn on_insert(&self, _: Self::InsertInput) -> Self::InsertExpression {}
    }

    pub trait OnUpdate {
        type UpdateInput;
        type UpdateExpression;
        fn on_update(&self, input: Self::UpdateInput) -> Self::UpdateExpression;
    }

    impl OnUpdate for () {
        type UpdateInput = ();
        type UpdateExpression = ();
        fn on_update(&self, _: Self::UpdateInput) -> Self::UpdateExpression {}
    }
}

pub mod named_bind {
    use crate::{
        extentions::common_expressions::{Identifier, Scoped, V0OnInsert},
        sqlx_query_builder::basic_expressions::ScopedColumn,
    };

    pub struct NamedBind<Table, Name, Value> {
        pub table: Table,
        pub name: Name,
        pub value: Value,
    }

    impl<T, Name, V> Identifier for NamedBind<T, Name, V>
    where
        Name: Identifier,
    {
        type Identifier = <Name as Identifier>::Identifier;
        fn identifier(&self) -> Self::Identifier {
            self.name.identifier()
        }
    }

    impl<T: Clone, Name: Clone, V> Scoped for NamedBind<T, Name, V> {
        type Scoped = ScopedColumn<T, Name>;
        fn scoped(&self) -> Self::Scoped {
            ScopedColumn {
                table: self.table.clone(),
                col: self.name.clone(),
            }
        }
    }

    impl<T: Clone, Name: Clone, V> Scoped for Vec<NamedBind<T, Name, V>> {
        type Scoped = Vec<ScopedColumn<T, Name>>;
        fn scoped(&self) -> Self::Scoped {
            self.iter().map(|e| e.scoped()).collect()
        }
    }

    impl<T, N, V> V0OnInsert for NamedBind<T, N, V> {
        type InsertInput = ();

        type InsertExpression = V;

        fn on_insert(self, _: Self::InsertInput) -> Self::InsertExpression {
            self.value
        }
    }

    impl<T, N, V> V0OnInsert for Vec<NamedBind<T, N, V>> {
        type InsertInput = ();
        type InsertExpression = Vec<V>;
        fn on_insert(self, input: Self::InsertInput) -> Self::InsertExpression {
            self.into_iter().map(|e| e.on_insert(input)).collect()
        }
    }
}

pub mod as_member_helper {
    use sqlx::Row;

    use crate::from_row::{FromRowAlias, FromRowData};

    pub struct AsMemberHelper<T>(pub T);
    impl<'r, T> FromRowData for AsMemberHelper<T>
    where
        T: crate::collections::Member,
    {
        type RData = T::Data;
    }
    impl<'r, R, T> FromRowAlias<'r, R> for AsMemberHelper<T>
    where
        T: crate::collections::Member,
        R: Row,
        T::Data: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database> + 'r,
        for<'q> &'q str: sqlx::ColumnIndex<R>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, crate::from_row::FromRowError> {
            Ok(row.get(self.0.name()))
        }

        fn str_alias(
            &self,
            row: crate::from_row::RowStrAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            Ok(row.get(self.0.name()))
        }

        fn num_alias(
            &self,
            row: crate::from_row::RowNumAliased<'r, R>,
        ) -> Result<Self::RData, crate::from_row::FromRowError>
        where
            R: sqlx::Row,
        {
            Ok(row.get(self.0.name()))
        }
    }
}
// pub trait CommonExpressions {
//     // used in Fetch* operations
//     type BaseMembersAliased;
//     fn members_aliased_in_base(&self) -> Self::BaseMembersAliased;

//     type LikedMembersAliased;
//     fn members_aliased_in_liked(&self) -> Self::LikedMembersAliased;

//     // used in insert statement or any statement with returning clause
//     type Members;
//     fn members(&self) -> Self::Members;
//     // used in update statement
//     type UpdateData;
//     type Updating;
//     fn updating(&self, sets: Self::UpdateData) -> Self::Updating;
// }

// #[allow(unused)]
// mod impl_relevant_expressions {
//     use std::marker::PhantomData;

//     use crate::{
//         collections::Collection,
//         database_extention::DatabaseExt,
//         extentions::CommonExpressions,
//         query_builder::{
//             Expression, OpExpression, SqlSyntax, StatementBuilder,
//             syntax::comma_join,
//         },
//         singlton::Singleton,
//         tuple_trait::{AsTuple, Tuple, TupleSpec},
//         update_mod::Update,
//     };

//     impl<T> CommonExpressions for T
//     where
//         T: Collection,
//         T: Singleton,
//         T::Data: AsTuple,
//         T::UpdateData: AsTuple,
//     {
//         type BaseMembersAliased = MembersAliased;
//         type LikedMembersAliased = MembersAliased;

//         fn members_aliased_in_base(&self) -> Self::BaseMembersAliased {
//             MembersAliased {
//                 code: 0,
//                 table: T::singleton().table_name(),
//                 names: <T::Data as AsTuple>::NAMES,
//             }
//         }
//         fn members_aliased_in_liked(&self) -> Self::LikedMembersAliased {
//             MembersAliased {
//                 code: 1,
//                 table: T::singleton().table_name(),
//                 names: <T::Data as AsTuple>::NAMES,
//             }
//         }

//         type Members = MembersAliased;
//         fn members(&self) -> Self::Members {
//             MembersAliased {
//                 code: 2,
//                 table: T::singleton().table_name(),
//                 names: <T::Data as AsTuple>::NAMES,
//             }
//         }

//         type UpdateData = T::UpdateData;
//         type Updating = UpdateData<<T::UpdateData as AsTuple>::Tuple>;
//         fn updating(&self, sets: Self::UpdateData) -> Self::Updating {
//             UpdateData {
//                 names: <T::UpdateData as AsTuple>::NAMES,
//                 table: T::singleton().table_name(),
//                 data: sets.into_tuple(),
//             }
//         }
//     }

//     pub struct MembersAliased {
//         /// 0: base, 1: liked, 2: no_alias
//         /// used to avoid having three different structs
//         /// with constant implementation, this is no difference in performance
//         pub code: u8,
//         pub table: &'static str,
//         pub names: &'static [&'static str],
//     }

//     impl OpExpression for MembersAliased {
//         fn is_op(&self) -> bool {
//             self.names.len() != 0
//         }
//     }
//     impl<'q, S: DatabaseExt> Expression<'q, S> for MembersAliased {
//         fn expression<
//             Start: crate::query_builder::SqlSyntax + ?Sized,
//             Join: crate::query_builder::SqlSyntax + ?Sized,
//         >(
//             self,
//             start: &Start,
//             join: &Join,
//             ctx: &mut StatementBuilder<'q, S>,
//         ) where
//             S: DatabaseExt,
//         {
//             let len = self.names.len();
//             if len != 0 {
//                 ctx.syntax(start);
//             }
//             match self.code {
//                 0 => {
//                     for (i, name) in self.names.iter().enumerate() {
//                         ctx.sanitize(self.table);
//                         ctx.syntax(&".");
//                         ctx.sanitize(name);
//                         ctx.syntax(&" AS ");
//                         ctx.sanitize(format!("b{name}").as_str());
//                         if i < len - 1 {
//                             ctx.syntax(join);
//                         }
//                     }
//                 }
//                 1 => {
//                     for (i, name) in self.names.iter().enumerate() {
//                         todo!("should take number in account")
//                         // ctx.sanitize(self.table);
//                         // ctx.syntax(&".");
//                         // ctx.sanitize(name);
//                         // ctx.syntax(&" AS ");
//                         // ctx.sanitize(format!("l{name}").as_str());
//                         // if i < len - 1 {
//                         //     ctx.syntax(join);
//                         // }
//                     }
//                 }
//                 2 => {
//                     for (i, name) in self.names.iter().enumerate() {
//                         ctx.sanitize(name);
//                         if i < len - 1 {
//                             ctx.syntax(join);
//                         }
//                     }
//                 }
//                 _ => panic!("bug: never construct code: {}", self.code),
//             }
//         }
//     }

//     pub struct UpdateData<D> {
//         pub names: &'static [&'static str],
//         pub table: &'static str,
//         pub data: D,
//     }

//     impl<T> OpExpression for UpdateData<T> {
//         fn is_op(&self) -> bool {
//             self.names.len() != 0
//         }
//     }

//     pub struct ToEncode<'q, S, Q>(Q, &'static [&'static str], PhantomData<(&'q (), S)>);
//     impl<'q, S, TupleElement> TupleSpec<Update<TupleElement>>
//         for ToEncode<'q, S, &'_ mut StatementBuilder<'q, S>>
//     where
//         S: DatabaseExt,
//         TupleElement: sqlx::Type<S> + sqlx::Encode<'q, S> + 'q,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: Update<TupleElement>,
//         ) -> Self::Output {
//             match member {
//                 Update::Set(value) => {
//                     self.0.sanitize(self.1[INDEX]);
//                     self.0.syntax(&" = ");
//                     self.0.bind(value);
//                     if INDEX != LAST_INDEX {
//                         self.0.syntax(&comma_join);
//                     }
//                 }
//                 Update::Keep => {
//                     // do nothing
//                 }
//             }
//         }
//     }

//     impl<'q, S, EntireTuple> Expression<'q, S> for UpdateData<EntireTuple>
//     where
//         EntireTuple: 'q + for<'any> Tuple<ToEncode<'q, S, &'any mut StatementBuilder<'q, S>>>,
//     {
//         #[inline]
//         fn expression<
//             Start: crate::query_builder::SqlSyntax + ?Sized,
//             Join: crate::query_builder::SqlSyntax + ?Sized,
//         >(
//             self,
//             start: &Start,
//             join: &Join,
//             ctx: &mut StatementBuilder<'q, S>,
//         ) where
//             S: DatabaseExt,
//         {
//             Tuple::own_tuple_mut_spec(self.data, ToEncode(ctx, self.names, PhantomData));
//         }
//     }
// }
