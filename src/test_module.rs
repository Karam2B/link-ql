#[cfg(test)]
use crate::links::{DefaultRelationKey, Link, relation_one_to_many::OneToMany};

#[macro_export]
macro_rules! define_collection {
    (struct $pascal_case:ident $size:literal {$(
        $member:ident: $type:ty,
    )*}) => {
        const _: ()  ={
            if $size == 0 {
                panic!("size must be greater than 0");
            }
        };

        $crate::paste_crate::paste! {
        #[derive(Debug, PartialEq, Eq, Clone)]
        pub struct $pascal_case {
            $(
                pub $member: $type,
            )*
        }

        #[derive(Default, Debug, PartialEq, Eq, Clone)]
        pub struct [<$pascal_case Partial>] {
            $(
                pub $member: $crate::update_mod::Update<$type>,
            )*
        }

        #[derive(Clone, Copy, Default)]
        pub struct [<$pascal_case Handler>];

        impl AsRef<str> for [<$pascal_case Handler>] {
            fn as_ref(&self) -> &str {
                stringify!($pascal_case)
            }
        }

        // impl Singleton for $name
        const _: () = {
            use $crate::singleton::Singleton;
            impl Singleton for [<$pascal_case Handler>] {
                fn singleton() -> &'static Self {
                    &[<$pascal_case Handler>]
                }
            }
        };

        // impl AsTuple for $name
        const _: () = {
            use $crate::tuple_trait::AsTuple;
            impl AsTuple for $pascal_case {
                type Tuple = ($($type,)*);
                const NAMES: &'static [&'static str] = &[$(stringify!($member),)*];
                fn into_tuple(self) -> Self::Tuple {
                    ($(self.$member,)*)
                }
                fn from_tuple(($($member,)*): Self::Tuple) -> Self {
                    Self {
                        $( $member, )*
                    }
                }
            }

            impl AsTuple for [<$pascal_case Partial>] {
                type Tuple = ($($crate::update_mod::Update<$type>,)*);
                const NAMES: &'static [&'static str] = &[$(stringify!($member),)*];
                fn into_tuple(self) -> Self::Tuple {
                    ($(self.$member,)*)
                }
                fn from_tuple(($($member,)*): Self::Tuple) -> Self {
                    Self {
                        $( $member, )*
                    }
                }
            }
        };

        // impl Collection for $pascal_case
        const _: () = {
            use $crate::collections::Collection;
            use $crate::collections::SingleIncremintalInt;
            impl Collection for [<$pascal_case Handler>] {
                fn table_name(&self) -> &str {
                    stringify!($pascal_case)
                }
                fn table_name_lower_case(&self) -> &str {
                    stringify!([<$pascal_case:snake>])
                }
                type InputData = $pascal_case;
                type UpdateData = [<$pascal_case Partial>];
                type OutputData = $pascal_case;
                type Id = SingleIncremintalInt<&'static str>;
                fn id(&self) -> Self::Id {
                    SingleIncremintalInt(stringify!($pascal_case))
                }
            }
        };

        // impl HasHandler for $pascal_case
        const _: () = {
            use $crate::collections::HasHandler;
            impl HasHandler for $pascal_case {
                type Handler = [<$pascal_case Handler>];
            }
            impl HasHandler for [<$pascal_case Partial>] {
                type Handler = [<$pascal_case Handler>];
            }
        };

        // impl FromRowAlias
        const _: () = {
            use sqlx::{ColumnIndex, Decode, Row, Type};
            use $crate::from_row::{FromRowAlias, FromRowData, FromRowError, RowStrAliased, RowNumAliased};

            impl FromRowData for [<$pascal_case Handler>] {
                type RData = $pascal_case;
            }

            impl<'r, R> FromRowAlias<'r, R> for [<$pascal_case Handler>]
            where
                R: Row + 'r,
                $(
                    $type: Type<R::Database> + Decode<'r, R::Database>,
                )*
                for<'q> &'q str: ColumnIndex<R>,

            {
                fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
                    Ok(
                        $pascal_case {
                            $(
                                $member: row.try_get(stringify!($member))?,
                            )*
                        }
                    )
                }
                fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                    Ok(
                        $pascal_case {
                            $(
                                $member: row.try_get(stringify!($member))?,
                            )*
                        }
                    )
                }
                fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                    Ok(
                        $pascal_case {
                            $(
                                $member: row.try_get(stringify!($member))?,
                            )*
                        }
                    )
                }
            }

        };


        // impl gen_serde::Deserialize for $pascal_case
        const _: () = {
            use $crate::gen_serde::{Deserialize, DeserializeMap, DeserializeSpec, Deserializer, KnownKey};

            impl DeserializeSpec for $pascal_case {
                type Handler = ();
            }
            impl<'de, S> Deserialize<'de, S> for $pascal_case
            where
                S: Deserializer<'de>,
                S: DeserializeMap<'de>,
                $(
                    $type: Deserialize<'de, S>,
                )*
                S: KnownKey<&'static str>,
            {
                fn deserialize(_: Self::Handler, serialized: &mut S) -> Result<Self, S::Err> {
                    let mut map = serialized.start_map()?;
                    let res = Self {
                        $(
                            $member: serialized.deserialize_with_known_key(&mut map, stringify!($member), ())?,
                        )*
                    };
                    serialized.finish(map)?;

                    Ok(res)
                }
            }
        };

        // impl gen_serde::Serialize for $pascal_case
        const _: () = {
            use $crate::gen_serde::{Serialize, ObjectEncoding};

            impl<F> Serialize<F> for $pascal_case
            where
                F: ObjectEncoding,
                $(
                    $type: Serialize<F>,
                )*
                str: Serialize<F>,
            {
                fn serialize(&self, fmt: &mut F) {
                    let mut object  = F::serialize_start(fmt);
                    $(
                        F::serialize_pair(fmt, &mut object, stringify!($member), &self.$member);
                    )*
                    F::serialize_end(fmt, object);
                }
            }
        };

        // impl ExpressionsForOperation for $pascal_case
        const _: () = {
            use $crate::operations::operations_expressions_crossover::ExpressionsForOperation;
            use $crate::sqlx_query_builder::{
                basic_expressions::{AliasedScopedColumn, ScopedColumn},
                sanitize_combinator::Sanitize,
            };

            impl ExpressionsForOperation for [<$pascal_case Handler>] {
                type Identifier = [&'static str; $size];

                fn identifier(&self) -> Self::Identifier {
                    [$(stringify!($member),)*]
                }

                type Scoped = [ScopedColumn<&'static str, &'static str>; $size];

                fn scoped(&self) -> Self::Scoped {
                    [
                        $(
                            ScopedColumn {
                                table: stringify!($pascal_case),
                                col: stringify!($member),
                            },
                        )*
                    ]
                }

                type ScopedAliased = [
                    AliasedScopedColumn<
                        &'static str,
                        &'static str,
                        Sanitize<(&'static str, &'static str)>,
                    >;
                    $size
                ];

                fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                    [
                        $(
                            AliasedScopedColumn {
                                table: stringify!($pascal_case),
                                column: stringify!($member),
                                alias: Sanitize((alias, stringify!($member))),
                            },
                        )*
                    ]
                }

                type NumScopedAliased = [
                    AliasedScopedColumn<
                        &'static str,
                        &'static str,
                        Sanitize<(&'static str, usize, &'static str)>,
                    >;
                    $size
                ];

                fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
                    [
                        $(
                            AliasedScopedColumn {
                                table: stringify!($pascal_case),
                                column: stringify!($member),
                                alias: Sanitize((alias, num, stringify!($member))),
                            },
                        )*
                    ]
                }
            }

            use $crate::operations::operations_expressions_crossover::TableExpressions;

            impl TableExpressions for [<$pascal_case Handler>] {
                type SnakeCase = &'static str;
                type PascalCase = &'static str;
                fn table_name_snake_case(&self) -> Self::SnakeCase {
                    stringify!([<$pascal_case:snake>])
                }
                fn table_name_pascal_case(&self) -> Self::PascalCase {
                    stringify!($pascal_case)
                }
                type InheritJoin = ();
                fn inherit_join(&self) -> Self::InheritJoin {}
            }


        };

        // impl OnInsert for $pascal_case
        const _: () = {
            use $crate::sqlx_query_builder::statements::insert_statement::IteratorSpec;
            use $crate::operations::operations_expressions_crossover::OnInsert;
            use $crate::operations::operations_expressions_crossover::ExpressionsForOperation;
            use $crate::sqlx_query_builder::Expression;
            use $crate::sqlx_query_builder::OpExpression;
            use $crate::sqlx_query_builder::{StatementBuilder};
            use $crate::database_extention::DatabaseExt;
            use sqlx::{Encode, Type};

            impl OnInsert<$pascal_case> for [<$pascal_case Handler>] {
                type InsertExpression = $pascal_case;
                fn on_insert(&self, input: $pascal_case) -> Self::InsertExpression {
                    input
                }
                type InsertId = [&'static str; $size];
                fn on_insert_with_id(&self, input: $pascal_case) -> (Self::InsertId, Self::InsertExpression) {
                    (self.identifier(), input)
                }
            }

            impl OpExpression for $pascal_case {
            }

            impl<T> OnInsert<IteratorSpec<T>> for [<$pascal_case Handler>]
            where T: IntoIterator<Item = $pascal_case>,
            {
                type InsertExpression = IteratorSpec<T>;
                fn on_insert(&self, input: IteratorSpec<T>) -> Self::InsertExpression {
                    input
                }
                type InsertId = [&'static str; $size];
                fn on_insert_with_id(&self, input: IteratorSpec<T>) -> (Self::InsertId, Self::InsertExpression) {
                    (self.identifier(), input)
                }
            }

            impl<'q, S> Expression<'q, S> for $pascal_case
            where
                S: DatabaseExt,
                $(
                    $type: Encode<'q, S> + 'q + Type<S>,
                )*
            {
                #[allow(unused_assignments)]
                fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
                    let mut first = true;
                    $(
                        if !first {
                            ctx.syntax(", ");
                        }
                        first = false;
                        ctx.bind(self.$member);
                    )*
                }
            }
        };

        // impl OnUpdate for $pascal_case
        const _: () = {
            use $crate::operations::operations_expressions_crossover::OnUpdate;
            use $crate::update_mod::Update;
            use $crate::sqlx_query_builder::Expression;
            use $crate::sqlx_query_builder::OpExpression;
            use $crate::sqlx_query_builder::{StatementBuilder};
            use $crate::database_extention::DatabaseExt;
            use sqlx::{Encode, Type};

            impl OnUpdate<[<$pascal_case Partial>]> for [<$pascal_case Handler>] {
                type UpdateExpression = [<$pascal_case Partial>];
                fn on_update(&self, input: [<$pascal_case Partial>]) -> Self::UpdateExpression {
                    input
                }
            }

            impl OpExpression for [<$pascal_case Partial>] {
                fn is_expression_present(&self) -> bool {
                    false $(|| matches!(self.$member, Update::Set(_)))*
                }
            }

            impl<'q, S> Expression<'q, S> for [<$pascal_case Partial>]
            where
                S: DatabaseExt,
                $(
                    $type: Encode<'q, S> + 'q + Type<S>,
                )*
            {
                #[allow(unused_assignments)]
                fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
                    let mut first = true;
                    $(
                        if let Update::Set(value) = self.$member {
                            if !first {
                                ctx.syntax(", ");
                            }
                            first = false;
                            ctx.syntax(stringify!($member));
                            ctx.syntax(" = ");
                            ctx.bind(value);
                        }
                    )*
                }
            }
        };

        // members
        pub mod [<$pascal_case:snake _members>] {
            use $crate::operations::operations_expressions_crossover::ExpressionsForOperation;
            use $crate::sqlx_query_builder::{
                basic_expressions::{AliasedScopedColumn, ScopedColumn},
                sanitize_combinator::Sanitize,
            };
            use $crate::collections::SingleColumnId;
            use $crate::collections::CollectionId;

            #[allow(non_camel_case_types, dead_code)]
            #[derive(Debug, Clone)]
            pub struct id;

            impl CollectionId for id {
                type IdData = i64;
            }
            impl SingleColumnId for id {}

            impl AsRef<str> for id {
                fn as_ref(&self) -> &str {
                    "id"
                }
            }

            impl ExpressionsForOperation for id {
                type Identifier = &'static str;
                fn identifier(&self) -> Self::Identifier {
                    "id"
                }
                type Scoped = ScopedColumn<&'static str, &'static str>;
                fn scoped(&self) -> Self::Scoped {
                    ScopedColumn {
                        table: stringify!($pascal_case),
                        col: "id",
                    }
                }
                type ScopedAliased = AliasedScopedColumn<
                    &'static str,
                    &'static str,
                    Sanitize<(&'static str, &'static str)>,
                >;
                fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                    AliasedScopedColumn {
                        table: stringify!($pascal_case),
                        column: "id",
                        alias: Sanitize((alias, "id")),
                    }
                }
                type NumScopedAliased = AliasedScopedColumn<
                    &'static str,
                    &'static str,
                    Sanitize<(&'static str, usize, &'static str)>,
                >;
                fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
                    AliasedScopedColumn {
                        table: stringify!($pascal_case),
                        column: "id",
                        alias: Sanitize((alias, num, "id")),
                    }
                }
            }

            const _: () = {
                use $crate::from_row::{
                    FromRowAlias, FromRowData, FromRowError, RowStrAliased,
                    RowNumAliased,
                };
                use sqlx::{ColumnIndex, Decode, Row, Type};

                impl FromRowData for id {
                    type RData = i64;
                }

                impl<'r, R> FromRowAlias<'r, R> for id
                where
                    R: Row + 'r,
                    i64: Type<R::Database> + Decode<'r, R::Database>,
                    for<'q> &'q str: ColumnIndex<R>,
                {
                    fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
                        Ok(row.try_get("id")?)
                    }

                    fn str_alias(
                        &self,
                        row: RowStrAliased<'r, R>,
                    ) -> Result<Self::RData, FromRowError> {
                        Ok(row.try_get("id")?)
                    }

                    fn num_alias(
                        &self,
                        row: RowNumAliased<'r, R>,
                    ) -> Result<Self::RData, FromRowError> {
                        Ok(row.try_get("id")?)
                    }
                }
            };

            $(
                #[allow(non_camel_case_types, dead_code)]
                #[derive(Debug, Clone)]
                pub struct $member;

                impl AsRef<str> for $member {
                    fn as_ref(&self) -> &str {
                        stringify!($member)
                    }
                }

                impl $member {
                    #[allow(dead_code)]
                    pub fn bind(value: $type) ->
                    $crate::operations::operations_expressions_crossover::NamedBind<
                        super::[<$pascal_case Handler>],
                        $member,
                        $type,
                    >
                    {
                        $crate::operations::operations_expressions_crossover::NamedBind {
                            table: super::[<$pascal_case Handler>],
                            name: $member,
                            value: value,
                        }
                    }
                }

                impl ExpressionsForOperation for $member {
                    type Identifier = &'static str;
                    fn identifier(&self) -> Self::Identifier {
                        stringify!($member)
                    }
                    type Scoped = ScopedColumn<&'static str, &'static str>;
                    fn scoped(&self) -> Self::Scoped {
                        ScopedColumn {
                            table: stringify!($pascal_case),
                            col: stringify!($member),
                        }
                    }
                    type ScopedAliased = AliasedScopedColumn<
                        &'static str,
                        &'static str,
                        Sanitize<(&'static str, &'static str)>,
                    >;
                    fn scoped_aliased(&self, alias: &'static str) -> Self::ScopedAliased {
                        AliasedScopedColumn {
                            table: stringify!($pascal_case),
                            column: stringify!($member),
                            alias: Sanitize((alias, stringify!($member))),
                        }
                    }
                    type NumScopedAliased = AliasedScopedColumn<
                        &'static str,
                        &'static str,
                        Sanitize<(&'static str, usize, &'static str)>,
                    >;
                    fn num_scoped_aliased(&self, num: usize, alias: &'static str) -> Self::NumScopedAliased {
                        AliasedScopedColumn {
                            table: stringify!($pascal_case),
                            column: stringify!($member),
                            alias: Sanitize((alias, num, stringify!($member))),
                        }
                    }
                }
            )*

            $(
                const _: () = {
                    use $crate::from_row::{FromRowData, FromRowAlias, FromRowError, RowStrAliased, RowNumAliased};
                    use sqlx::{Row, Type, Decode, ColumnIndex};

                    impl FromRowData for $member {
                        type RData = $type;
                    }

                    impl<'r, R> FromRowAlias<'r, R> for $member
                    where
                        R: Row + 'r,
                        $type: Type<R::Database> + Decode<'r, R::Database>,
                        for<'q> &'q str: ColumnIndex<R>,
                    {
                        fn no_alias(&self, row: &'r R) -> Result<Self::RData, FromRowError> {
                            Ok(row.try_get(stringify!($member))?)
                        }
                        fn str_alias(&self, row: RowStrAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                            Ok(row.try_get(stringify!($member))?)
                        }
                        fn num_alias(&self, row: RowNumAliased<'r, R>) -> Result<Self::RData, FromRowError> {
                            Ok(row.try_get(stringify!($member))?)
                        }
                    }
                };
            )*
        }


    }};
}

#[cfg(test)]
define_collection!(
    struct Todo 3 {
        title: String,
        done: bool,
        description: Option<String>,
    }
);

#[cfg(test)]
define_collection!(
    struct Category 1 {
        title: String,
    }
);

#[cfg(test)]
define_collection!(
    struct Tag 1 {
        title: String,
    }
);

#[cfg(test)]
impl Link<TodoHandler> for CategoryHandler {
    type Spec = OneToMany<DefaultRelationKey, TodoHandler, CategoryHandler>;

    fn spec(self) -> Self::Spec {
        OneToMany {
            fk_unique_id: DefaultRelationKey,
            from: TodoHandler,
            to: CategoryHandler,
        }
    }
}

#[cfg(test)]
#[linked_sql_macros::skip]
impl Link<TodoHandler> for TagHandler {
    type Spec = ManyToMany<false, DefaultRelationKey, TodoHandler, TagHandler>;

    fn spec(self) -> Self::Spec {
        ManyToMany {
            relation_key: DefaultRelationKey,
            from: TodoHandler,
            to: TagHandler,
        }
    }
}
