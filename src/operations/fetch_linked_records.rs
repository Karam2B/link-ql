use std::collections::HashMap;

use sqlx::{Decode, Encode, Row, Type};

use crate::{
    collections::{Collection, CollectionId, SingleColumnId},
    database_extention::DatabaseExt,
    execute::Executable,
    expressions::{filters::ColumnIn, table},
    extentions::{
        Members,
        common_expressions::{Identifier, TableNameExpression},
    },
    fix_executor::ExecutorTrait,
    from_row::FromRowAlias,
    links::{
        relation_many_to_many::ManyToMany,
        relation_one_to_many::join_expression::JoinExpression,
        relation_one_to_many_inverse::OneToManyInverse,
    },
    operations::{CollectionOutput, Operation, OperationOutput},
    sqlx_query_builder::{
        Expression, OpExpression, StatementBuilder, basic_expressions::Bind,
    },
    statements::select_statement::SelectStatement,
};

pub type LinkedRecordsMap<ParentId, ChildId, ChildOutput> =
    HashMap<ParentId, Vec<CollectionOutput<ChildId, ChildOutput>>>;

pub type ManyToManyLinkedMap<FromId, ToId, ToOutput> = LinkedRecordsMap<FromId, ToId, ToOutput>;

pub type OneToManyInverseLinkedMap<FromId, ToId, ToOutput> =
    LinkedRecordsMap<FromId, ToId, ToOutput>;

fn rows_to_linked_map<S, FromId, ToId, ToOutput, To>(
    rows: Vec<S::Row>,
    to: To,
    to_id: To::Id,
) -> LinkedRecordsMap<FromId, ToId, ToOutput>
where
    S: DatabaseExt,
    FromId: Copy + Clone + std::hash::Hash + Eq + for<'r> Decode<'r, S> + Type<S>,
    To: Collection + Members,
    To::OutputData: Send,
    ToId: Send,
    To: for<'r> FromRowAlias<'r, S::Row, RData = ToOutput>,
    To::Id: for<'r> FromRowAlias<'r, S::Row, RData = ToId>,
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    let mut map = HashMap::new();

    for row in rows {
        let from_id = row.try_get::<FromId, _>("from_id").unwrap();
        let id = to_id.no_alias(&row).unwrap();
        let attributes = to.no_alias(&row).unwrap();
        map.entry(from_id)
            .or_insert_with(Vec::new)
            .push(CollectionOutput { id, attributes });
    }

    map
}

struct InverseLinkedSelect {
    fk_col: String,
    to_table: String,
    to_cols: Vec<String>,
}

impl OpExpression for InverseLinkedSelect {
}

impl<'q, S> Expression<'q, S> for InverseLinkedSelect
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.sanitize(&self.to_table);
        ctx.syntax(".");
        ctx.sanitize(&self.fk_col);
        ctx.syntax(r#" AS "from_id", "#);
        ctx.sanitize(&self.to_table);
        ctx.syntax(".");
        ctx.sanitize("id");
        for col in &self.to_cols {
            ctx.syntax(", ");
            ctx.sanitize(&self.to_table);
            ctx.syntax(".");
            ctx.sanitize(col);
        }
    }
}

pub struct FetchOneToManyInverseLinked<Key, From, To>
where
    From: Collection,
    To: Collection + Members + TableNameExpression,
{
    pub link: OneToManyInverse<Key, From, To>,
    pub from_ids: Vec<<From::Id as CollectionId>::IdData>,
    fk_col: String,
    to_table: String,
    to_cols: Vec<String>,
}

impl<Key, From, To> Clone for FetchOneToManyInverseLinked<Key, From, To>
where
    Key: Clone,
    From: Collection + Clone,
    To: Collection + Members + TableNameExpression + Clone,
    <From::Id as CollectionId>::IdData: Clone,
{
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
            from_ids: self.from_ids.clone(),
            fk_col: self.fk_col.clone(),
            to_table: self.to_table.clone(),
            to_cols: self.to_cols.clone(),
        }
    }
}

impl<Key, From, To> OperationOutput for FetchOneToManyInverseLinked<Key, From, To>
where
    From: Collection,
    To: Collection + Members + TableNameExpression,
{
    type Output = OneToManyInverseLinkedMap<
        <From::Id as CollectionId>::IdData,
        <To::Id as CollectionId>::IdData,
        To::OutputData,
    >;
}

impl<Key, From, To> FetchOneToManyInverseLinked<Key, From, To>
where
    Key: Clone + AsRef<str>,
    From: Collection + TableNameExpression + Clone,
    To: Collection + TableNameExpression + Members + Clone,
{
    pub fn new(
        link: OneToManyInverse<Key, From, To>,
        from_ids: Vec<<From::Id as CollectionId>::IdData>,
    ) -> Self {
        let to = link.to.clone();
        Self {
            fk_col: format!(
                "fk_{}{}",
                link.from.table_name_lower_case(),
                link.fk_unique_id.as_ref(),
            ),
            to_table: to.table_name().to_string(),
            to_cols: to.members_names(),
            link,
            from_ids,
        }
    }
}

impl<S, Key, From, To> Operation<S> for FetchOneToManyInverseLinked<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From::Id as CollectionId>::IdData: Copy
        + Clone
        + std::hash::Hash
        + Eq
        + Send
        + for<'q> Encode<'q, S>
        + Type<S>
        + for<'r> Decode<'r, S>,
    To: Collection<Id: SingleColumnId + Identifier> + TableNameExpression + Members + Clone + Send,
    To::OutputData: Send,
    <To::Id as CollectionId>::IdData: Send,
    To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
    To::Id: for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
    <To::Id as Identifier>::Identifier: for<'q> Expression<'q, S>,
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        if self.from_ids.is_empty() {
            return HashMap::new();
        }

        let to_table = self.to_table.clone();
        let fk_col = self.fk_col.clone();

        let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
            select_items: (InverseLinkedSelect {
                fk_col: fk_col.clone(),
                to_table: to_table.clone(),
                to_cols: self.to_cols,
            },),
            from: table(to_table.clone()),
            joins: (),
            wheres: (ColumnIn {
                col: table(to_table).col(fk_col),
                values: self.from_ids.into_iter().map(Bind).collect::<Vec<_>>(),
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

        let to = self.link.to.clone();
        let to_id = to.id();
        rows_to_linked_map::<S, _, _, _, _>(rows, to, to_id)
    }
}

struct ManyToManyLinkedSelect {
    junction_table: String,
    from_col: String,
    to_table: String,
    to_cols: Vec<String>,
}

impl OpExpression for ManyToManyLinkedSelect {
}

impl<'q, S> Expression<'q, S> for ManyToManyLinkedSelect
where
    S: DatabaseExt,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        ctx.sanitize(&self.junction_table);
        ctx.syntax(".");
        ctx.sanitize(&self.from_col);
        ctx.syntax(r#" AS "from_id", "#);
        ctx.sanitize(&self.to_table);
        ctx.syntax(".");
        ctx.sanitize("id");
        for col in &self.to_cols {
            ctx.syntax(", ");
            ctx.sanitize(&self.to_table);
            ctx.syntax(".");
            ctx.sanitize(col);
        }
    }
}

pub struct FetchManyToManyLinked<Key, From, To>
where
    From: Collection,
    To: Collection + Members + TableNameExpression,
{
    pub link: ManyToMany<false, Key, From, To>,
    pub from_ids: Vec<<From::Id as CollectionId>::IdData>,
    junction_table: String,
    from_col: String,
    to_table: String,
    to_cols: Vec<String>,
}

impl<Key, From, To> Clone for FetchManyToManyLinked<Key, From, To>
where
    Key: Clone,
    From: Collection + Clone,
    To: Collection + Members + TableNameExpression + Clone,
    <From::Id as CollectionId>::IdData: Clone,
{
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
            from_ids: self.from_ids.clone(),
            junction_table: self.junction_table.clone(),
            from_col: self.from_col.clone(),
            to_table: self.to_table.clone(),
            to_cols: self.to_cols.clone(),
        }
    }
}

impl<Key, From, To> OperationOutput for FetchManyToManyLinked<Key, From, To>
where
    From: Collection,
    To: Collection + Members + TableNameExpression,
{
    type Output = ManyToManyLinkedMap<
        <From::Id as CollectionId>::IdData,
        <To::Id as CollectionId>::IdData,
        To::OutputData,
    >;
}

impl<Key, From, To> FetchManyToManyLinked<Key, From, To>
where
    Key: Clone + AsRef<str>,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone,
    To: Collection<Id: SingleColumnId> + TableNameExpression + Members + Clone,
{
    pub fn new(
        link: ManyToMany<false, Key, From, To>,
        from_ids: Vec<<From::Id as CollectionId>::IdData>,
    ) -> Self {
        let to = link.to.clone();
        Self {
            junction_table: format!(
                "ct_{}{}{}",
                link.from.table_name_lower_case(),
                link.to.table_name_lower_case(),
                link.relation_key.as_ref()
            ),
            from_col: format!("{}_id", link.from.table_name_lower_case()),
            to_table: to.table_name().to_string(),
            to_cols: to.members_names(),
            link,
            from_ids,
        }
    }
}

impl<S, Key, From, To> Operation<S> for FetchManyToManyLinked<Key, From, To>
where
    S: DatabaseExt + ExecutorTrait,
    Key: Clone + AsRef<str> + Send,
    From: Collection<Id: SingleColumnId> + TableNameExpression + Clone + Send,
    <From::Id as CollectionId>::IdData: Copy
        + Clone
        + std::hash::Hash
        + Eq
        + Send
        + for<'q> Encode<'q, S>
        + Type<S>
        + for<'r> Decode<'r, S>,
    To: Collection<Id: SingleColumnId + Identifier> + TableNameExpression + Members + Clone + Send,
    To::OutputData: Send,
    <To::Id as CollectionId>::IdData: Send,
    To: for<'r> FromRowAlias<'r, S::Row, RData = To::OutputData>,
    To::Id: for<'r> FromRowAlias<'r, S::Row, RData = <To::Id as CollectionId>::IdData>,
    <To::Id as Identifier>::Identifier: for<'q> Expression<'q, S>,
    <To as TableNameExpression>::TableNameExpression: for<'q> Expression<'q, S> + Clone,
    <To as TableNameExpression>::LowerCaseTableNameExpression: AsRef<str>,
    for<'a> &'a str: sqlx::ColumnIndex<S::Row>,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        if self.from_ids.is_empty() {
            return HashMap::new();
        }

        let junction = self.junction_table.clone();
        let from_col = self.from_col.clone();

        let (stmt, args) = StatementBuilder::<'_, S>::new(SelectStatement {
            select_items: (ManyToManyLinkedSelect {
                junction_table: junction.clone(),
                from_col: from_col.clone(),
                to_table: self.to_table,
                to_cols: self.to_cols,
            },),
            from: table(junction.clone()),
            joins: (JoinExpression {
                join_type: "INNER JOIN",
                foreign_table: self.link.to.table_name_expression(),
                foreign_column: self.link.to.id().identifier(),
                local_table: table(junction.clone()),
                local_column: self.link.to_junction_column(),
            },),
            wheres: (ColumnIn {
                col: table(junction).col(from_col),
                values: self.from_ids.into_iter().map(Bind).collect::<Vec<_>>(),
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

        let to = self.link.to.clone();
        let to_id = self.link.to.id();
        rows_to_linked_map::<S, _, _, _, _>(rows, to, to_id)
    }
}
