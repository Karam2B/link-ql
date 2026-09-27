mod impl_connect_in_memory {
    use crate::connect_in_memory::ConnectInMemory;
    use sqlx::pool::PoolOptions;
    use sqlx::{ConnectOptions, Pool, Sqlite, sqlite::SqliteConnectOptions};

    impl ConnectInMemory for Sqlite {
        async fn in_memory_connection() -> <Self as sqlx::Database>::Connection {
            SqliteConnectOptions::new()
                .in_memory(true)
                .connect()
                .await
                .unwrap()
        }
        fn in_memory_pool() -> impl Future<Output = Pool<Self>> {
            async {
                PoolOptions::<Sqlite>::new()
                    .max_connections(1)
                    .connect("sqlite::memory:")
                    .await
                    .unwrap()
            }
        }
    }
}

mod impl_database_extention {
    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
    };
    use sqlx::Sqlite;

    impl DatabaseExt for Sqlite {
        fn sanitize(string: &str, into: &mut String) {
            let mut s = string.chars();
            while let Some(next) = s.next() {
                match next {
                    '"' => {
                        into.push(next);
                        into.push('"');
                    }
                    '\'' => {
                        into.push(next);
                        into.push('\'');
                    }
                    '\\' => {
                        into.push(next);
                        into.push('\\');
                    }
                    n => into.push(n),
                }
            }
        }
        fn sanitize_start(into: &mut String) {
            into.push('"');
        }
        fn sanitize_end(into: &mut String) {
            into.push('"');
        }
        type IdExpression = IdExpression;
        fn id_on_create_table_expression() -> Self::IdExpression {
            IdExpression
        }
        type SinglePrimaryKeyConstaint = IdExpression2;
        fn single_primary_key_constaint_expression() -> Self::SinglePrimaryKeyConstaint {
            IdExpression2
        }
        type MultiplePrimaryKeyConstaint = IdExpression2;
        fn multiple_primary_key_constaint_expression() -> Self::MultiplePrimaryKeyConstaint {
            IdExpression2
        }
    }

    pub struct IdExpression;

    impl OpExpression for IdExpression {
    }
    impl<'q> Expression<'q, Sqlite> for IdExpression {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax(&"\"id\" INTEGER PRIMARY KEY AUTOINCREMENT");
        }
    }

    pub struct IdExpression2;

    impl OpExpression for IdExpression2 {
    }
    impl<'q> Expression<'q, Sqlite> for IdExpression2 {
        fn expression(self, ctx: &mut StatementBuilder<'q, Sqlite>)
        where
            Sqlite: DatabaseExt,
        {
            ctx.syntax(&"INTEGER PRIMARY KEY");
        }
    }
}
