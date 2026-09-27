use sqlx::{ColumnIndex, Decode, Row, Type};

pub mod swich_to_base_id {
    use sqlx::Row;

    use crate::from_row::{RowNumAliased, RowStrAliased};

    pub fn str_alias_to_base_id<'r, R: Row>(
        str_alias: RowStrAliased<'r, R>,
    ) -> RowStrAliased<'r, R> {
        RowStrAliased::new(str_alias.row, "i")
    }
    pub fn num_alias_to_base_id<'r, R: Row>(
        num_alias: RowNumAliased<'r, R>,
    ) -> RowNumAliased<'r, R> {
        RowNumAliased::new(num_alias.row, "i")
    }
}

pub struct RowStrAliased<'r, R: Row> {
    pub(crate) row: &'r R,
    pub(crate) alias: &'static str,
}

impl<'r, R: Row> Clone for RowStrAliased<'r, R> {
    fn clone(&self) -> Self {
        Self {
            row: self.row,
            alias: self.alias,
        }
    }
}

impl<'r, R: Row> RowStrAliased<'r, R> {
    pub fn new(row: &'r R, alias: &'static str) -> Self {
        Self { row, alias }
    }
    pub fn try_get<T>(&self, name: &str) -> Result<T, sqlx::Error>
    where
        T: Type<R::Database> + Decode<'r, R::Database>,
        for<'q> &'q str: ColumnIndex<R>,
    {
        Row::try_get(self.row, format!("{}{}", self.alias, name).as_str())
    }
    #[track_caller]
    pub fn get<T>(&self, name: &str) -> T
    where
        T: Type<R::Database> + Decode<'r, R::Database>,
        for<'q> &'q str: ColumnIndex<R>,
    {
        self.try_get(name).unwrap()
    }
}

pub struct RowNumAliased<'r, R: Row> {
    pub(crate) row: &'r R,
    pub(crate) str_alias: &'static str,
    // only Vec<T> and tuples, can initiate this with Some(usize)
    pub(crate) num_alias: Option<usize>,
}

impl<'r, R: Row> Clone for RowNumAliased<'r, R> {
    fn clone(&self) -> Self {
        Self {
            row: self.row,
            str_alias: self.str_alias,
            num_alias: self.num_alias,
        }
    }
}

impl<'r, R: Row> RowNumAliased<'r, R> {
    pub fn new(row: &'r R, name: &'static str) -> Self {
        Self {
            row,
            str_alias: name,
            num_alias: None,
        }
    }
    pub fn try_get<T>(&self, name: &str) -> Result<T, sqlx::Error>
    where
        T: Type<R::Database> + Decode<'r, R::Database>,
        for<'q> &'q str: ColumnIndex<R>,
    {
        Row::try_get(
            self.row,
            format!(
                "{}{}{}",
                self.str_alias,
                self.num_alias.map(|e| e.to_string()).unwrap_or_default(),
                name,
            )
            .as_str(),
        )
    }
    #[track_caller]
    pub fn get<T>(&self, name: &str) -> T
    where
        T: Type<R::Database> + Decode<'r, R::Database>,
        for<'q> &'q str: ColumnIndex<R>,
    {
        self.try_get(name).unwrap()
    }
}

#[derive(Debug)]
pub enum FromRowError {
    MismatchType,
    ColumnNotFound(String),
}

impl From<sqlx::Error> for FromRowError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::ColumnNotFound(name) => FromRowError::ColumnNotFound(name),
            _ => panic!(
                "{value:?}, either: 1. incorrect impl of FromRowAlias, 2. uncatchable error like database disconnection"
            ),
        }
    }
}

pub mod from_row_v2 {
    use core::fmt;

    use crate::from_row::{FromRowData, FromRowError, RowNumAliased, RowStrAliased};
    use sqlx::{ColumnIndex, Database, Decode, Row, Type};

    pub trait RowAliased: Clone + Sized {
        type SqlxRow: Row;
        type Database: Database;

        fn get_sqlx_row(&self) -> &Self::SqlxRow;

        #[inline]
        #[track_caller]
        fn get<I, T>(self, index: I) -> T
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            T: Type<Self::Database> + for<'r> Decode<'r, Self::Database>,
        {
            self.try_get::<I, T>(index).unwrap()
        }

        fn try_get_optional<I, T>(self, index: I) -> Result<Option<T>, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            Option<T>: Type<Self::Database> + for<'r> Decode<'r, Self::Database>;

        fn try_get<I, T>(self, index: I) -> Result<T, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            T: Type<Self::Database> + for<'r> Decode<'r, Self::Database>;
    }

    impl<'r, R: Row> RowAliased for &'r R
    where
        for<'q> &'q str: ColumnIndex<R>,
    {
        type SqlxRow = R;
        type Database = R::Database;

        fn get_sqlx_row(&self) -> &Self::SqlxRow {
            *self
        }

        fn try_get<I, T>(self, index: I) -> Result<T, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            T: Type<Self::Database> + for<'s> Decode<'s, Self::Database>,
        {
            sqlx::Row::try_get(self.get_sqlx_row(), index)
        }

        fn try_get_optional<I, T>(self, index: I) -> Result<Option<T>, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            Option<T>: Type<Self::Database> + for<'s> Decode<'s, Self::Database>,
        {
            sqlx::Row::try_get(self.get_sqlx_row(), index)
        }
    }
    impl<'r, R: Row> RowAliased for RowStrAliased<'r, R>
    where
        for<'q> &'q str: ColumnIndex<R>,
    {
        type SqlxRow = R;

        type Database = R::Database;

        fn get_sqlx_row(&self) -> &Self::SqlxRow {
            self.row
        }

        fn try_get<I, T>(self, index: I) -> Result<T, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            T: Type<Self::Database> + for<'r2> Decode<'r2, Self::Database>,
        {
            sqlx::Row::try_get(
                self.row,
                format!("{}{}", self.alias, index.to_string()).as_str(),
            )
        }
        fn try_get_optional<I, T>(self, index: I) -> Result<Option<T>, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            Option<T>: Type<Self::Database> + for<'r2> Decode<'r2, Self::Database>,
        {
            sqlx::Row::try_get(
                self.row,
                format!("{}{}", self.alias, index.to_string()).as_str(),
            )
        }
    }

    impl<'r, R: Row> RowAliased for RowNumAliased<'r, R>
    where
        for<'q> &'q str: ColumnIndex<R>,
    {
        type SqlxRow = R;

        type Database = R::Database;

        fn get_sqlx_row(&self) -> &Self::SqlxRow {
            self.row
        }

        fn try_get<I, T>(self, index: I) -> Result<T, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            T: Type<Self::Database> + for<'r2> Decode<'r2, Self::Database>,
        {
            sqlx::Row::try_get(
                self.row,
                format!(
                    "{}{}{}",
                    self.str_alias,
                    self.num_alias.unwrap_or_default(),
                    index.to_string()
                )
                .as_str(),
            )
        }
        fn try_get_optional<I, T>(self, index: I) -> Result<Option<T>, sqlx::Error>
        where
            I: ColumnIndex<Self::SqlxRow> + fmt::Display,
            Option<T>: Type<Self::Database> + for<'r2> Decode<'r2, Self::Database>,
        {
            sqlx::Row::try_get(
                self.row,
                format!(
                    "{}{}{}",
                    self.str_alias,
                    self.num_alias.unwrap_or_default(),
                    index.to_string()
                )
                .as_str(),
            )
        }
    }

    pub trait FromRowAlias2<'r, RAlias>: FromRowData {
        fn from_row_alias(&self, row_aliased: RAlias) -> Result<Self::RData, FromRowError>;
    }
}

pub trait FromRowData {
    type RData;
}

pub trait FromRowAlias<'r, R>: FromRowData {
    // used in operation with a returning clause
    fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError>;
    // used in operation that have local field belong to different links and collections
    fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError>
    where
        R: Row;
    // used in links, where `Vec<T>` and tuples use an Option<usize>
    fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError>
    where
        R: Row;
}

pub mod named_col_from_row {
    use crate::from_row::FromRowAlias;
    use crate::from_row::FromRowData;
    use crate::from_row::FromRowError;
    use sqlx::ColumnIndex;
    use sqlx::Decode;
    use sqlx::Row;
    use sqlx::Type as SType;
    use std::marker::PhantomData;

    pub struct NamedColFromRow<Name, Type> {
        pub name: Name,
        pub ty: PhantomData<Type>,
    }

    impl<Name, Type> FromRowData for NamedColFromRow<Name, Type> {
        type RData = Type;
    }

    impl<'r, R, Name, Type> FromRowAlias<'r, R> for NamedColFromRow<Name, Type>
    where
        R: Row,
        // there is no way for now to implement this without converting to a string first
        // niche use case I have is Sanitize, where it usually have the form of
        // Sanitize<(&'static str, (DefaultKey,), &'static str)>
        // the only way to avoid heap allocation is to finish gen_serde::Serialize<StringBuffer>
        Name: ToString,
        for<'s> &'s str: ColumnIndex<R>,
        Type: for<'q> Decode<'q, R::Database> + SType<R::Database>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
            Ok(row.get(self.name.to_string().as_str()))
        }

        fn str_alias(&self, row: super::RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError>
        where
            R: Row,
        {
            Ok(row.get(self.name.to_string().as_str()))
        }

        fn num_alias(&self, row: super::RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError>
        where
            R: Row,
        {
            Ok(row.get(self.name.to_string().as_str()))
        }
    }
}

#[linked_sql_macros::skip]
mod functional_impls {
    use crate::from_row::FromRowAlias;
    use crate::from_row::FromRowData;
    use crate::from_row::FromRowError;
    use crate::from_row::RowNumAliased;
    use crate::from_row::RowStrAliased;
    use sqlx::Row;

    impl<T> FromRowData for Vec<T>
    where
        T: FromRowData,
    {
        type RData = Vec<T::RData>;
    }
    impl<'r, T, R> FromRowAlias<'r, R> for Vec<T>
    where
        T: FromRowAlias<'r, R>,
    {
        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
            let mut r = vec![];
            for each in self {
                r.push(each.no_alias(row)?);
            }
            Ok(r)
        }

        fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError>
        where
            R: Row,
        {
            let mut r = vec![];
            for each in self {
                r.push(each.str_alias(row)?);
            }
            Ok(r)
        }

        fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError>
        where
            R: Row,
        {
            let mut r = vec![];
            for each in self {
                r.push(each.num_alias(row)?);
            }
            Ok(r)
        }
    }
}

pub trait TryFromRowAlias<'r, R>: FromRowData {
    fn try_no_alias(&self, row: &'r R) -> Result<Option<Self::RData>, FromRowError>;
    fn try_str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Option<Self::RData>, FromRowError>
    where
        R: Row;
    fn try_num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Option<Self::RData>, FromRowError>
    where
        R: Row;
}

impl FromRowData for () {
    type RData = ();
}

impl<'r, R> FromRowAlias<'r, R> for ()
where
    R: Row,
{
    fn no_alias(&self, _: &'r R) -> Result<Self::RData, FromRowError> {
        Ok(())
    }
    fn str_alias(&self, _: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
        Ok(())
    }
    fn num_alias(&self, _: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
        Ok(())
    }
}

pub mod row_helpers {
    use crate::from_row::FromRowAlias;
    use crate::from_row::FromRowError;
    use crate::from_row::RowStrAliased;
    use sqlx::Row;

    pub trait AliasRowHelper<'r, Handler>: Row + Sized + 'r {
        type Output;
        fn row_no_alias(&'r self, handler: &Handler) -> Result<Self::Output, FromRowError>;
        fn row_str_alias(
            &'r self,
            handler: &Handler,
            str_alias_str: &'static str,
        ) -> Result<Self::Output, FromRowError>;
        fn row_num_alias(
            &'r self,
            handler: &Handler,
            str_alias_str: &'static str,
            num_alias_num: Option<usize>,
        ) -> Result<Self::Output, FromRowError>;
    }

    impl<'r, Handler, Row_> AliasRowHelper<'r, Handler> for Row_
    where
        Handler: FromRowAlias<'r, Row_>,
        Row_: Row + Sized + 'r,
    {
        type Output = Handler::RData;
        fn row_no_alias(&'r self, handler: &Handler) -> Result<Handler::RData, FromRowError> {
            handler.no_alias(self)
        }
        fn row_num_alias(
            &'r self,
            handler: &Handler,
            str_alias_str: &'static str,
            num_alias_num: Option<usize>,
        ) -> Result<Self::Output, FromRowError> {
            handler.num_alias(super::RowNumAliased {
                row: self,
                str_alias: str_alias_str,
                num_alias: num_alias_num,
            })
        }
        fn row_str_alias(
            &'r self,
            handler: &Handler,
            str_alias_str: &'static str,
        ) -> Result<Handler::RData, FromRowError> {
            handler.str_alias(RowStrAliased::new(self, str_alias_str))
        }
    }

    pub trait OneRowHelper<'r>: Row + Sized + 'r {
        fn from_row<Data: sqlx::FromRow<'r, Self>>(&'r self) -> Result<Data, FromRowError>;
    }

    impl<'r, Row_> OneRowHelper<'r> for Row_
    where
        Row_: Row + Sized + 'r,
    {
        fn from_row<Data: sqlx::FromRow<'r, Self>>(&'r self) -> Result<Data, FromRowError> {
            Ok(Data::from_row(self)?)
        }
    }

    pub trait ManyRowHelper<'r> {
        type Single: Row + Sized + 'r;
        fn from_rows<Data2: sqlx::FromRow<'r, Self::Single>>(
            &'r self,
        ) -> Result<Vec<Data2>, FromRowError>;
    }

    impl<'r, Row_> ManyRowHelper<'r> for Vec<Row_>
    where
        Row_: Row + Sized + 'r,
    {
        type Single = Row_;
        fn from_rows<Data: sqlx::FromRow<'r, Self::Single>>(
            &'r self,
        ) -> Result<Vec<Data>, FromRowError> {
            let mut r = vec![];
            for each in self {
                r.push(Data::from_row(each)?);
            }
            Ok(r)
        }
    }
}

#[cfg(test)]
mod test {
    use sqlx::Sqlite;

    use crate::{
        connect_in_memory::ConnectInMemory,
        from_row::{FromRowAlias, RowStrAliased},
        test_module::{Category, CategoryHandler},
    };

    #[tokio::test]
    async fn main() {
        let pool = Sqlite::in_memory_pool().await;

        let row = sqlx::query(
            "
        CREATE TABLE Category ( title TEXT );
        INSERT INTO Category (title) VALUES ('cat_1');
        SELECT title as cat_title, title  FROM Category;
    ",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let s = CategoryHandler
            .str_alias(RowStrAliased::new(&row, "cat_"))
            .unwrap();

        assert_eq!(
            s,
            Category {
                title: "cat_1".to_string(),
            },
        );

        let s = CategoryHandler.no_alias(&row).unwrap();

        assert_eq!(
            s,
            Category {
                title: "cat_1".to_string(),
            },
        );
    }
}
