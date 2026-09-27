use sqlx::{Decode, Encode, Row, Type};

use crate::{
    collections::{Collection, CollectionId, SingleColumnId},
    database_extention::DatabaseExt,
    execute::Executable,
    expressions::table,
    extentions::{
        Members,
        common_expressions::{Aliased, TableNameExpression},
    },
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    links::relation_many_to_many::ManyToMany,
    operations::{
        LinkedOutput, Operation, OperationOutput, fetch_one::FetchOne,
        operations_expressions_crossover::{ExpressionsForOperation, TableExpressions},
    },
    sqlx_query_builder::{
        basic_expressions::{Bind, ColumnEqual},
        Expression, Join, StatementBuilder,
    },
    statements::{
        delete_statement::DeleteStatement,
        insert_statement::{InsertStatement, One},
        select_statement::SelectStatement,
    },
};

pub trait ManyToManyJunctionNames {
    fn junction_table_as_str(&self) -> String;
    fn from_junction_col_as_str(&self) -> String;
    fn to_junction_col_as_str(&self) -> String;
}

impl<Key, From, To> ManyToManyJunctionNames for ManyToMany<false, Key, From, To>
where
    Key: Clone + AsRef<str>,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone,
    To: Collection<Id: SingleColumnId> + TableNameExpression + Clone,
{
    fn junction_table_as_str(&self) -> String {
        format!(
            "ct_{}{}{}",
            self.from.table_name_lower_case(),
            self.to.table_name_lower_case(),
            self.relation_key.as_ref()
        )
    }

    fn from_junction_col_as_str(&self) -> String {
        format!("{}_id", self.from.table_name_lower_case())
    }

    fn to_junction_col_as_str(&self) -> String {
        format!("{}_id", self.to.table_name_lower_case())
    }
}

#[derive(Clone)]
pub struct InsertJunctionRow<Key, From, To> {
    link: ManyToMany<false, Key, From, To>,
    from_id: i64,
    to_id: i64,
}

impl<Key, From, To> InsertJunctionRow<Key, From, To> {
    pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
        Self {
            link,
            from_id,
            to_id,
        }
    }
}

impl<Key, From, To> OperationOutput for InsertJunctionRow<Key, From, To>
where
    From: Collection,
    To: Collection,
{
    type Output = ();
}

impl<S, Key, From, To> Operation<S> for InsertJunctionRow<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <From::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
    To: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <To as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <To::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
    i64: for<'q> Encode<'q, S> + Type<S> + Send,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        let (stmt, args) = StatementBuilder::<'_, S>::new(InsertStatement {
            table_name: self.link.junction_table_name(),
            identifiers: Join {
                start: "",
                separator: ", ",
                items: (
                    self.link.from_junction_column(),
                    self.link.to_junction_column(),
                ),
            },
            values: One(Join {
                start: "",
                separator: ", ",
                items: (Bind(self.from_id), Bind(self.to_id)),
            }),
            returning: (),
        })
        .unwrap();

        S::execute(
            &mut *pool,
            Executable {
                string: &stmt,
                arguments: args,
            },
        )
        .await
        .unwrap();
    }
}

#[derive(Clone)]
pub struct DeleteJunctionRow<Key, From, To> {
    link: ManyToMany<false, Key, From, To>,
    from_id: i64,
    to_id: i64,
}

impl<Key, From, To> DeleteJunctionRow<Key, From, To> {
    pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
        Self {
            link,
            from_id,
            to_id,
        }
    }
}

impl<Key, From, To> OperationOutput for DeleteJunctionRow<Key, From, To>
where
    From: Collection,
    To: Collection,
{
    type Output = ();
}

impl<S, Key, From, To> Operation<S> for DeleteJunctionRow<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <From::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
    To: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <To as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <To::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
    i64: for<'q> Encode<'q, S> + Type<S> + Send,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        let junction = self.link.junction_table_as_str();
        let from_col = self.link.from_junction_col_as_str();
        let to_col = self.link.to_junction_col_as_str();

        let (stmt, args) = StatementBuilder::<'_, S>::new(DeleteStatement {
            table_name: self.link.junction_table_name(),
            wheres: Join {
                start: "",
                separator: ", ",
                items: (
                    ColumnEqual {
                        col: table(junction.clone()).col(from_col),
                        eq: Bind(self.from_id),
                    },
                    ColumnEqual {
                        col: table(junction).col(to_col),
                        eq: Bind(self.to_id),
                    },
                ),
            },
            returning: (),
        })
        .unwrap();

        S::execute(
            &mut *pool,
            Executable {
                string: &stmt,
                arguments: args,
            },
        )
        .await
        .unwrap();
    }
}

#[derive(Clone)]
pub struct InsertJunctionAndFetch<Key, From, To> {
    link: ManyToMany<false, Key, From, To>,
    from_id: i64,
    to_id: i64,
}

impl<Key, From, To> InsertJunctionAndFetch<Key, From, To> {
    pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64, to_id: i64) -> Self {
        Self {
            link,
            from_id,
            to_id,
        }
    }
}

impl<Key, From, To> OperationOutput for InsertJunctionAndFetch<Key, From, To>
where
    From: Collection,
    To: Collection,
{
    type Output = LinkedOutput<<To::Id as CollectionId>::IdData, To::OutputData, ()>;
}

impl<S, Key, From, To> Operation<S> for InsertJunctionAndFetch<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <From::Id as CollectionId>::IdData: Send + for<'q> Encode<'q, S> + Type<S> + Copy,
    To: Collection<Id: SingleColumnId> + TableNameExpression + TableExpressions<
        ScopedAliased: for<'q> Expression<'q, S>,
        PascalCase: for<'q> Expression<'q, S>,
    > + Members + Clone + Send,
    <To as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <To::Id as CollectionId>::IdData:
        ::std::convert::From<i64> + Send + 'static + for<'q> Encode<'q, S> + Type<S> + Copy,
    To::Id: Send
        + ExpressionsForOperation<
            Scoped: for<'q> Expression<'q, S>,
            ScopedAliased: for<'q> Expression<'q, S>,
        >
        + for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
    To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
    To::OutputData: Send,
    ColumnEqual<<To::Id as ExpressionsForOperation>::Scoped, Bind<<To::Id as CollectionId>::IdData>>: Send,
    i64: for<'q> Encode<'q, S> + Type<S> + Send,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        InsertJunctionRow::new(self.link.clone(), self.from_id, self.to_id)
            .exec_operation(&mut *pool)
            .await;

        let to_id = <To::Id as CollectionId>::IdData::from(self.to_id);
        FetchOne {
            base: self.link.to.clone(),
            links: (),
            wheres: ColumnEqual {
                col: self.link.to.id().scoped(),
                eq: Bind(to_id),
            },
        }
        .exec_operation(&mut *pool)
        .await
        .expect("linked row should exist")
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct SelectJunctionToIds<Key, From, To> {
    link: ManyToMany<false, Key, From, To>,
    from_id: i64,
}

impl<Key, From, To> SelectJunctionToIds<Key, From, To> {
    pub fn new(link: ManyToMany<false, Key, From, To>, from_id: i64) -> Self {
        Self { link, from_id }
    }
}

impl<Key, From, To> OperationOutput for SelectJunctionToIds<Key, From, To>
where
    From: Collection,
    To: Collection,
{
    type Output = Vec<i64>;
}

impl<S, Key, From, To> Operation<S> for SelectJunctionToIds<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <From::Id as CollectionId>::IdData: for<'q> Encode<'q, S> + Type<S> + Send + Copy,
    To: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <To as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    <To::Id as CollectionId>::IdData:
        for<'r> Decode<'r, S> + for<'q> Encode<'q, S> + Type<S> + Send + Copy,
    i64: for<'r> Decode<'r, S> + for<'q> Encode<'q, S> + Type<S> + Send,
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        let junction = self.link.junction_table_as_str();
        let from_col = self.link.from_junction_col_as_str();
        let to_col = self.link.to_junction_col_as_str();

        let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
            select_items: (table(junction.clone()).col(to_col.clone()),),
            from: table(junction.clone()),
            joins: (),
            wheres: (ColumnEqual {
                col: table(junction).col(from_col),
                eq: Bind(self.from_id),
            },),
            group_by: (),
            order: (),
            limit: (),
        })
        .unwrap();

        let rows = S::fetch_all(
            &mut *pool,
            Executable {
                string: &stmt,
                arguments: args,
            },
        )
        .await
        .unwrap();

        rows.into_iter()
            .map(|row| row.try_get::<i64, _>(to_col.as_str()).unwrap())
            .collect()
    }
}
