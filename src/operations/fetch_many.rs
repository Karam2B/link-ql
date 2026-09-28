use crate::{
    collections::{Collection, CollectionId, SingleColumnId},
    database_extention::DatabaseExt,
    execute::Executable,
    fix_executor::ExecutorTrait,
    from_row::{FromRowAlias, FromRowData, RowStrAliased},
    operations::{
        LinkedOutput, Operation, OperationOutput,
        operations_expressions_crossover::{
            ExpressionsForOperation, SelfPrescribedInsert, TableExpressions,
        },
    },
    sqlx_query_builder::{
        Expression, Join, StatementBuilder,
        basic_expressions::{Bind, ManyColumnsLargerOrEqual},
        combinators::{Nest, OptionalExpression},
        statements::select_statement::SelectStatement,
    },
};
use sqlx::{Encode, Type};

pub trait LinkFetch {
    type SelectItems;
    fn non_aggregating_select_items(&self) -> Self::SelectItems;

    type Join;
    fn non_duplicating_join_expressions(&self) -> Self::Join;

    type Wheres;
    fn where_expressions(&self) -> Self::Wheres;

    type Op;
    type OpInput;

    fn operation_construct_once(&self, item: &<Self::SelectItems as FromRowData>::RData) -> Self::Op
    where
        Self::SelectItems: FromRowData,
    {
        let mut ret = self.operation_initialize_input();
        self.operation_fix_on_many(item, &mut ret);
        self.operation_construct(ret)
    }

    fn operation_initialize_input(&self) -> Self::OpInput;

    fn operation_fix_on_many(
        &self,
        item: &<Self::SelectItems as FromRowData>::RData,
        poi: &mut Self::OpInput,
    ) where
        Self::SelectItems: FromRowData;

    fn operation_construct(&self, input: Self::OpInput) -> Self::Op
    where
        Self::SelectItems: FromRowData;

    type Output;

    fn take_once(
        self,
        item: <Self::SelectItems as FromRowData>::RData,
        mut op: <Self::Op as OperationOutput>::Output,
    ) -> Self::Output
    where
        Self: Sized,
        Self::SelectItems: FromRowData,
        Self::Op: OperationOutput,
    {
        self.take_many(item, &mut op)
    }

    fn take_many(
        &self,
        item: <Self::SelectItems as FromRowData>::RData,
        op: &mut <Self::Op as OperationOutput>::Output,
    ) -> Self::Output
    where
        Self::SelectItems: FromRowData,
        Self::Op: OperationOutput;
}

pub use std_impls::Empty;

mod std_impls {
    use super::LinkFetch;
    use crate::from_row::{RowNumAliased, RowStrAliased};
    use crate::{
        from_row::{FromRowAlias, FromRowData, FromRowError},
        operations::operations_expressions_crossover::ExpressionsForOperation,
    };
    use sqlx::Row;

    pub struct Empty;

    impl LinkFetch for () {
        type Output = ();
        // maybe should be replaced by ()
        type SelectItems = Empty;
        fn non_aggregating_select_items(&self) -> Self::SelectItems {
            Empty
        }

        type Join = ();
        fn non_duplicating_join_expressions(&self) -> Self::Join {}

        type Wheres = ();

        fn where_expressions(&self) -> Self::Wheres {}

        type Op = ();
        type OpInput = ();
        fn operation_initialize_input(&self) -> Self::OpInput {}
        fn operation_construct(&self, _: Self::OpInput) -> Self::Op
        where
            Self::SelectItems: FromRowData,
        {
        }

        fn operation_fix_on_many(
            &self,
            _: &<Self::SelectItems as FromRowData>::RData,
            _: &mut Self::Op,
        ) where
            Self::SelectItems: FromRowData,
        {
        }

        fn take_many(
            &self,
            item: <Self::SelectItems as FromRowData>::RData,
            _: &mut <Self::Op as crate::operations::OperationOutput>::Output,
        ) -> Self::Output {
            item
        }
    }

    impl ExpressionsForOperation for Empty {
        type Identifier = ();
        fn identifier(&self) -> Self::Identifier {
            ()
        }
        type Scoped = ();
        fn scoped(&self) -> Self::Scoped {
            ()
        }
        type ScopedAliased = ();
        fn scoped_aliased(&self, _: &'static str) -> Self::ScopedAliased {
            ()
        }
        type NumScopedAliased = ();
        fn num_scoped_aliased(&self, _: usize, _: &'static str) -> Self::NumScopedAliased {
            ()
        }
    }

    impl FromRowData for Empty {
        type RData = ();
    }

    impl<'r, R> FromRowAlias<'r, R> for Empty
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
}

pub struct FetchMany<From, Links, Wheres, Order, FirstItem> {
    pub base: From,
    pub wheres: Wheres,
    pub links: Links,
    pub cursor_order_by: Order,
    pub cursor_first_item: FirstItem,
    pub limit: i64,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ManyOutput<T, Next> {
    pub items: Vec<T>,
    pub next_item: Option<Next>,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NextItem<Id, OrderedByFields> {
    pub id: Id,
    pub ordered_by_field: OrderedByFields,
}

impl<B, L, W, O, F> OperationOutput for FetchMany<B, L, W, O, F>
where
    B: Collection,
    L: LinkFetch,
    O: FromRowData,
{
    type Output = ManyOutput<
        LinkedOutput<<B::Id as CollectionId>::IdData, B::OutputData, L::Output>,
        NextItem<<B::Id as CollectionId>::IdData, O::RData>,
    >;
}

pub trait SealedFirstItemTrait {}

pub trait FirstItemTrait<B>: SealedFirstItemTrait {
    type WhereClause;
    fn where_clause(self, base_id: B::Id) -> Self::WhereClause
    where
        B: Collection;
}

impl SealedFirstItemTrait for () {}

impl<B> FirstItemTrait<B> for () {
    type WhereClause = ();

    fn where_clause(self, _: <B>::Id) -> Self::WhereClause
    where
        B: Collection,
    {
    }
}

impl<T0, T1> SealedFirstItemTrait for (T0, T1) {}

impl<B, First> FirstItemTrait<B> for (<B::Id as CollectionId>::IdData, First)
where
    B: Collection<Id: ExpressionsForOperation + SingleColumnId>,
    First: SelfPrescribedInsert,
{
    type WhereClause = ManyColumnsLargerOrEqual<
        (
            Nest<First::InsertId>,
            Nest<<B::Id as ExpressionsForOperation>::Scoped>,
        ),
        (
            Nest<First::InsertValue>,
            Nest<Bind<<B::Id as CollectionId>::IdData>>,
        ),
    >;

    fn where_clause(self, base_id: B::Id) -> Self::WhereClause
    where
        B: Collection,
    {
        let (idents, values) = self.1.on_insert();
        ManyColumnsLargerOrEqual {
            ids: (Nest(idents), Nest(base_id.scoped())),
            values: (Nest(values), Nest(Bind(self.0))),
        }
    }
}

impl<T0, T1> SealedFirstItemTrait for Option<(T0, T1)> {}

impl<B, First> FirstItemTrait<B> for Option<(<B::Id as CollectionId>::IdData, First)>
where
    (<B::Id as CollectionId>::IdData, First): FirstItemTrait<B>,
    B: Collection<Id: SingleColumnId>,
{
    type WhereClause =
        Option<<(<B::Id as CollectionId>::IdData, First) as FirstItemTrait<B>>::WhereClause>;

    fn where_clause(self, base_id: B::Id) -> Self::WhereClause
    where
        B: Collection,
    {
        self.map(|first| first.where_clause(base_id))
    }
}

// was: `for FetchMany<Base, Links, Wheres, OrderBy, (<Base::Id as CollectionId>::IdData, First)>`
impl<S, Base, Links, Wheres, OrderBy, First2> Operation<S>
    for FetchMany<Base, Links, Wheres, OrderBy, First2>
where
    S: DatabaseExt,
    S: ExecutorTrait,
    Base: Send,
    OrderBy: Send,
    Wheres: Send + OptionalExpression,
    for<'q> Join<Wheres>: Expression<'q, S>,
    Links: Send + LinkFetch<Output: Send>,
    Links::Wheres: OptionalExpression,
    for<'q> Join<Links::Wheres>: Expression<'q, S>,
    Links::SelectItems: Send + ExpressionsForOperation<ScopedAliased: OptionalExpression>,
    for<'q> Join<<Links::SelectItems as ExpressionsForOperation>::ScopedAliased>: Expression<'q, S>,
    Links::SelectItems: for<'r> FromRowAlias<'r, S::Row, RData: Send>,
    Links::Join: OptionalExpression,
    for<'q> Join<Links::Join>: Expression<'q, S>,
    Links::Op: Operation<S>,
    Links::OpInput: Send,
    Base: Collection<OutputData: Send, Id: Send>,
    Base: TableExpressions<
            PascalCase: for<'q> Expression<'q, S>,
            ScopedAliased: OptionalExpression,
        >,
    for<'q> Join<Base::ScopedAliased>: Expression<'q, S>,
    Base: FromRowData<RData = Base::OutputData>,
    Base: for<'r> FromRowAlias<'r, S::Row>,
    Base::InheritJoin: OptionalExpression,
    for<'q> Join<Base::InheritJoin>: Expression<'q, S>,
    Base::Id: Send + CollectionId<IdData: Send>,
    Base::Id: FromRowData<RData = <Base::Id as CollectionId>::IdData>,
    Base::Id: for<'r> FromRowAlias<'r, S::Row>,
    First2: Send + FirstItemTrait<Base, WhereClause: Send + OptionalExpression>,
    for<'q> Join<<First2 as FirstItemTrait<Base>>::WhereClause>: Expression<'q, S>,
    Base::Id: ExpressionsForOperation<
            ScopedAliased: OptionalExpression,
            Scoped: OptionalExpression,
        >,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::ScopedAliased>: Expression<'q, S>,
    for<'q> Join<<Base::Id as ExpressionsForOperation>::Scoped>: Expression<'q, S>,
    Links: LinkFetch<Output: Send>,
    i64: for<'q> Encode<'q, S> + Type<S>,
    OrderBy: Send + Clone,
    OrderBy: ExpressionsForOperation<Scoped: OptionalExpression>,
    for<'q> Join<<OrderBy as ExpressionsForOperation>::Scoped>: Expression<'q, S>,
    OrderBy: for<'r> FromRowAlias<'r, S::Row, RData: Send>,
{
    async fn exec_operation(self, pool: &mut S::Connection) -> Self::Output {
        // let db = S::singleton();
        let id = self.base.id();
        let link_items = self.links.non_aggregating_select_items();
        let query_builder = StatementBuilder::<S>::new(SelectStatement {
            select_items: (
                Nest(id.scoped_aliased("i")),
                Nest(self.base.scoped_aliased("b")),
                Nest(link_items.scoped_aliased("l")),
            ),
            from: self.base.table_name_pascal_case(),
            joins: (
                Nest(self.base.inherit_join()),
                Nest(self.links.non_duplicating_join_expressions()),
            ),
            group_by: (),
            order: (Nest(self.cursor_order_by.scoped()),),
            wheres: (
                Nest(self.wheres),
                Nest(self.links.where_expressions()),
                Nest(self.cursor_first_item.where_clause(self.base.id())),
            ),
            limit: Bind(self.limit + 1),
        });

        let (stmt, arg) = query_builder.unwrap();

        tracing::info!(sql_stmt = %stmt, "fetch many");

        let mut all = S::fetch_all(
            &mut *pool,
            Executable {
                string: &stmt,
                arguments: arg,
            },
        )
        .await
        .unwrap();

        let has_more = if all.len() == (self.limit + 1) as usize {
            let last = all
                .pop()
                .expect("bug: len is usize + 1, should have last item to pop");
            let next = self
                .cursor_order_by
                .str_alias(RowStrAliased::new(&last, "b"))
                .unwrap();
            let id = id.str_alias(RowStrAliased::new(&last, "i")).unwrap();
            Some(NextItem {
                id,
                ordered_by_field: next,
            })
        } else {
            None
        };

        let mut input = self.links.operation_initialize_input();

        let all = all
            .into_iter()
            .map(|e| {
                let id = id.str_alias(RowStrAliased::new(&e, "i")).unwrap();
                let link = link_items.str_alias(RowStrAliased::new(&e, "l")).unwrap();
                self.links.operation_fix_on_many(&link, &mut input);
                return LinkedOutput {
                    id,
                    attributes: self.base.str_alias(RowStrAliased::new(&e, "b")).unwrap(),
                    links: link,
                };
            })
            .collect::<Vec<_>>();

        let mut po = self
            .links
            .operation_construct(input)
            .exec_operation(&mut *pool)
            .await;

        let all = all
            .into_iter()
            .map(|e| LinkedOutput {
                id: e.id,
                attributes: e.attributes,
                links: self.links.take_many(e.links, &mut po),
            })
            .collect::<Vec<_>>();

        ManyOutput {
            items: all,
            next_item: has_more,
        }
    }
}

#[cfg(test)]
mod test {
    use sqlx::{Sqlite, query};

    use crate::{
        connect_in_memory::ConnectInMemory,
        operations::{
            LinkedOutput, Operation,
            fetch_many::{FetchMany, ManyOutput, NextItem},
        },
        test_module::{Todo, TodoHandler, todo_members},
        track_sqlx_query::watch_sqlx_calls,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn main() {
        watch_sqlx_calls(async |actions| {
            let mut conn = Sqlite::in_memory_connection().await;

            query(
                "
                CREATE TABLE Todo (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    done BOOLEAN NOT NULL,
                    description TEXT
                );

                INSERT INTO Todo (title, done, description) VALUES
                    ('non_unique', true, 'description_1'),
                    ('second_todo', false, 'description_2'),
                    ('third_todo', true, 'description_3'),
                    ('non_unique', false, 'description_4'),
                    ('fifth_todo', true, 'description_5'),
                    ('sixth_todo', false, 'description_6');
                ",
            )
            .execute(&mut conn)
            .await
            .unwrap();
            actions.clear();

            let output = Operation::<Sqlite>::exec_operation(
                FetchMany {
                    base: TodoHandler,
                    wheres: (),
                    links: (),
                    cursor_order_by: todo_members::title,
                    cursor_first_item: Some((4, todo_members::title::bind(String::from("non_unique")))),
                    limit: 2,
                },
                &mut conn,
            )
            .await;

            pretty_assertions::assert_eq!(
                actions.take(),
                vec![
                    r#"SELECT "Todo"."id" AS "iid", "Todo"."title" AS "btitle", "Todo"."done" AS "bdone", "Todo"."description" AS "bdescription" FROM "Todo" WHERE ("Todo"."title","Todo"."id") >= ($1,$2) ORDER BY "Todo"."title" LIMIT $3;"#
                        .to_string(),
                ]
            );

            pretty_assertions::assert_eq!(
                output,
                ManyOutput {
                    items: vec![
                        LinkedOutput {
                            id: 4,
                            attributes: Todo {
                                title: "non_unique".to_string(),
                                done: false,
                                description: Some("description_4".to_string()),
                            },
                            links: (),
                        },
                        LinkedOutput {
                            id: 2,
                            attributes: Todo {
                                title: "second_todo".to_string(),
                                done: false,
                                description: Some("description_2".to_string()),
                            },
                            links: (),
                        },
                    ],
                    next_item: Some(NextItem {
                        id: 6,
                        ordered_by_field: String::from("sixth_todo"),
                    }),
                }
            );
        })
        .await;
    }
}
