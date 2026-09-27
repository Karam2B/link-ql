pub trait Collection {
    fn table_name(&self) -> &str;
    fn table_name_lower_case(&self) -> &str;
    /// Wire or expression input for insert-style operations.
    type InputData;
    /// Patch input for update-style operations.
    type UpdateData;
    /// Row shape after SELECT / RETURNING.
    type OutputData;
    type Id: CollectionId;
    fn id(&self) -> Self::Id;
}

/// dyn-compatable parts of Collection
pub trait CollectionBasic {
    fn table_name(&self) -> &str;
    fn table_name_lower_case(&self) -> &str;
}

impl<T> CollectionBasic for T
where
    T: Collection,
{
    fn table_name(&self) -> &str {
        self.table_name()
    }
    fn table_name_lower_case(&self) -> &str {
        self.table_name_lower_case()
    }
}

pub trait Member {
    fn name(&self) -> &str;
    type Data;
    type CollectionHandler;
}

pub trait HasHandler {
    type Handler;
}

pub trait CollectionId {
    type IdData;
}

pub trait SingleColumnId: CollectionId + AsRef<str> {}

#[derive(Clone, Debug)]
pub struct SingleIncremintalInt<ForTable>(pub ForTable);

mod impl_single_incremintal_int {
    use super::SingleIncremintalInt;
    use crate::collections::{CollectionId, SingleColumnId};

    impl<T> AsRef<str> for SingleIncremintalInt<T> {
        fn as_ref(&self) -> &str {
            "id"
        }
    }

    impl<T> CollectionId for SingleIncremintalInt<T> {
        type IdData = i64;
    }

    impl<T> SingleColumnId for SingleIncremintalInt<T> {}
}

mod single_incremintal_int_from_row {
    use sqlx::{ColumnIndex, Decode, Row, Type};

    use super::SingleIncremintalInt;
    use crate::from_row::{
        FromRowAlias, FromRowData, FromRowError, RowStrAliased, RowNumAliased,
        TryFromRowAlias,
    };

    impl<T> FromRowData for SingleIncremintalInt<T> {
        type RData = i64;
    }

    impl<'r, R: Row, T> FromRowAlias<'r, R> for SingleIncremintalInt<T>
    where
        R: Row + 'r,
        i64: Type<R::Database> + Decode<'r, R::Database>,
        for<'a> &'a str: ColumnIndex<R>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
            Ok(row.try_get("id")?)
        }
        fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            Ok(row.try_get("id")?)
        }
        fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
            Ok(row.try_get("id")?)
        }
    }

    impl<'r, R: Row, T> TryFromRowAlias<'r, R> for SingleIncremintalInt<T>
    where
        R: Row + 'r,
        i64: Type<R::Database> + Decode<'r, R::Database>,
        for<'a> &'a str: ColumnIndex<R>,
    {
        fn try_no_alias(&self, row: &'r R) -> Result<Option<Self::RData>, FromRowError> {
            Ok(row.get("id"))
        }

        fn try_str_alias(
            &self,
            row: RowStrAliased<'r, R>,
        ) -> Result<Option<Self::RData>, FromRowError>
        where
            R: Row,
        {
            Ok(row.get("id"))
        }

        fn try_num_alias(
            &self,
            row: RowNumAliased<'r, R>,
        ) -> Result<Option<Self::RData>, FromRowError>
        where
            R: Row,
        {
            Ok(row.get("id"))
        }
    }
}

#[linked_sql_macros::skip]
pub(crate) mod impl_id {
    use sqlx::Sqlite;
    use tracing::warn;

    use crate::{
        collections::{
            AutoGenerate, CollectionId, CreateIdFor, Manual, SingleColumnId, SingleIncremintalInt,
        },
        extentions::common_expressions::{
            Aliased, Identifier, MigrateExpression, Scoped, V0OnUpdate,
        },
        sqlx_query_builder::{
            Expression, OpExpression, StatementBuilder,
            basic_expressions::{AliasedScopedColumn, Bind, ScopedColumn, UpdatingColumn},
            sanitize_combinator::Sanitize,
        },
        update_mod::Update,
    };

    impl<T> AsRef<str> for SingleIncremintalInt<T> {
        fn as_ref(&self) -> &str {
            "id"
        }
    }

    impl<T> CollectionId for SingleIncremintalInt<T> {
        type IdData = i64;
    }

    impl<T> CreateIdFor<SingleIncremintalInt<T>> for AutoGenerate {
        type Result = ();
        fn create_id(self, _: &SingleIncremintalInt<T>) -> Option<()> {
            None
        }
    }

    impl<T> CreateIdFor<SingleIncremintalInt<T>> for Manual<i64> {
        type Result = Bind<i64>;
        fn create_id(self, _: &SingleIncremintalInt<T>) -> Option<Self::Result> {
            warn!("todo: bind can be used in set sytax (id = $0)");

            Some(Bind(self.0))
        }
    }

    impl<T> SingleColumnId for SingleIncremintalInt<T> {}

    impl Aliased for SingleIncremintalInt<&'static str> {
        type Aliased = AliasedScopedColumn<
            &'static str,
            &'static str,
            Sanitize<(&'static str, &'static str)>,
        >;
        fn aliased(&self, alias: &'static str) -> Self::Aliased {
            AliasedScopedColumn {
                table: self.0,
                column: "id",
                alias: Sanitize((alias, "id")),
            }
        }
        type NumAliased = AliasedScopedColumn<
            &'static str,
            &'static str,
            Sanitize<(&'static str, usize, &'static str)>,
        >;
        fn num_aliased(&self, num: usize, alias: &'static str) -> Self::NumAliased {
            AliasedScopedColumn {
                table: self.0,
                column: "id",
                alias: Sanitize((alias, num, "id")),
            }
        }
    }

    impl Aliased for SingleIncremintalInt<String> {
        type Aliased = AliasedScopedColumn<
            String,
            &'static str,
            Sanitize<(&'static str, &'static str)>,
        >;
        fn aliased(&self, alias: &'static str) -> Self::Aliased {
            AliasedScopedColumn {
                table: self.0.clone(),
                column: "id",
                alias: Sanitize((alias, "id")),
            }
        }
        type NumAliased = AliasedScopedColumn<
            String,
            &'static str,
            Sanitize<(&'static str, usize, &'static str)>,
        >;
        fn num_aliased(&self, num: usize, alias: &'static str) -> Self::NumAliased {
            AliasedScopedColumn {
                table: self.0.clone(),
                column: "id",
                alias: Sanitize((alias, num, "id")),
            }
        }
    }

    impl Aliased for SingleIncremintalInt<std::sync::Arc<str>> {
        type Aliased = AliasedScopedColumn<
            std::sync::Arc<str>,
            &'static str,
            Sanitize<(&'static str, &'static str)>,
        >;
        fn aliased(&self, alias: &'static str) -> Self::Aliased {
            AliasedScopedColumn {
                table: std::sync::Arc::clone(&self.0),
                column: "id",
                alias: Sanitize((alias, "id")),
            }
        }
        type NumAliased = AliasedScopedColumn<
            std::sync::Arc<str>,
            &'static str,
            Sanitize<(&'static str, usize, &'static str)>,
        >;
        fn num_aliased(&self, num: usize, alias: &'static str) -> Self::NumAliased {
            AliasedScopedColumn {
                table: std::sync::Arc::clone(&self.0),
                column: "id",
                alias: Sanitize((alias, num, "id")),
            }
        }
    }

    impl Scoped for SingleIncremintalInt<&'static str> {
        type Scoped = ScopedColumn<&'static str, &'static str>;
        fn scoped(&self) -> Self::Scoped {
            ScopedColumn {
                table: self.0,
                col: "id",
            }
        }
    }

    impl Scoped for SingleIncremintalInt<String> {
        type Scoped = ScopedColumn<String, &'static str>;
        fn scoped(&self) -> Self::Scoped {
            ScopedColumn {
                table: self.0.clone(),
                col: "id",
            }
        }
    }

    impl Scoped for SingleIncremintalInt<std::sync::Arc<str>> {
        type Scoped = ScopedColumn<std::sync::Arc<str>, &'static str>;
        fn scoped(&self) -> Self::Scoped {
            ScopedColumn {
                table: std::sync::Arc::clone(&self.0),
                col: "id",
            }
        }
    }

    impl<T> Identifier for SingleIncremintalInt<T> {
        type Identifier = &'static str;
        fn identifier(&self) -> Self::Identifier {
            "id"
        }
    }

    impl<T> crate::extentions::common_expressions::V0OnInsert for SingleIncremintalInt<T> {
        type InsertInput = ();
        type InsertExpression = ();

        fn on_insert(self, _: Self::InsertInput) -> Self::InsertExpression {}
    }

    impl V0OnUpdate for SingleIncremintalInt<&'static str> {
        type UpdateInput = Update<i64>;
        type UpdateExpression = UpdatingColumn<&'static str, Update<i64>>;
        fn on_update(self, input: Self::UpdateInput) -> Self::UpdateExpression {
            UpdatingColumn {
                col: self.identifier(),
                set: input,
            }
        }
    }

    impl V0OnUpdate for SingleIncremintalInt<String> {
        type UpdateInput = Update<i64>;
        type UpdateExpression = UpdatingColumn<String, Update<i64>>;
        fn on_update(self, input: Self::UpdateInput) -> Self::UpdateExpression {
            UpdatingColumn {
                col: self.0,
                set: input,
            }
        }
    }

    pub struct IdMigration;

    impl<T> MigrateExpression for SingleIncremintalInt<T> {
        type MigrateExpression = IdMigration;
        fn migrate_expression(&self) -> Self::MigrateExpression {
            IdMigration
        }
    }

    impl OpExpression for IdMigration {
}
    impl<'q> Expression<'q, Sqlite> for IdMigration {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>) {
            ctx.syntax(&"\"id\" INTEGER PRIMARY KEY AUTOINCREMENT");
        }
    }

    mod from_row_impls {
        use sqlx::{ColumnIndex, Decode, Row, Type};

        use super::SingleIncremintalInt;
        use crate::from_row::{
            FromRowAlias, FromRowData, FromRowError, RowStrAliased, RowNumAliased,
            TryFromRowAlias,
        };

        impl<T> FromRowData for SingleIncremintalInt<T> {
            type RData = i64;
        }

        impl<'r, R: Row, T> FromRowAlias<'r, R> for SingleIncremintalInt<T>
        where
            R: Row + 'r,
            i64: Type<R::Database> + Decode<'r, R::Database>,
            for<'a> &'a str: ColumnIndex<R>,
        {
            fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
                Ok(row.try_get("id")?)
            }
            fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                Ok(row.try_get("id")?)
            }
            fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                Ok(row.try_get("id")?)
            }
        }

        // T is infered to be Option<i64>
        impl<'r, R: Row, T> TryFromRowAlias<'r, R> for SingleIncremintalInt<T>
        where
            R: Row + 'r,
            i64: Type<R::Database> + Decode<'r, R::Database>,
            for<'a> &'a str: ColumnIndex<R>,
        {
            fn try_no_alias(&self, row: &'r R) -> Result<Option<Self::RData>, FromRowError> {
                Ok(row.get("id"))
            }

            fn try_str_alias(
                &self,
                row: RowStrAliased<'r, R>,
            ) -> Result<Option<Self::RData>, FromRowError>
            where
                R: Row,
            {
                Ok(row.get("id"))
            }

            fn try_num_alias(
                &self,
                row: RowNumAliased<'r, R>,
            ) -> Result<Option<Self::RData>, FromRowError>
            where
                R: Row,
            {
                Ok(row.get("id"))
            }
        }
    }
}
