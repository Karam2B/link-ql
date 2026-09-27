// pub trait OnMigrate {
//     type Statements;
//     fn statments(&self) -> Self::Statements;
// }

// #[cfg(feature = "skip_without_comments")]
#[linked_sql_macros::skip]
pub mod dynamic_migrate {
    use std::pin::Pin;

    use sqlx::{Database, Executor, Pool};

    use crate::{
        database_extention::DatabaseExt,
        on_migrate::OnMigrate,
        sqlx_query_builder::{Expression, StatementBuilder},
    };

    pub trait DynamicOnMigrate<S: Database> {
        fn migrate(&self, pool: Pool<S>) -> Pin<Box<dyn Future<Output = ()>>>;
    }

    impl<T, S> DynamicOnMigrate<S> for T
    where
        S: DatabaseExt,
        T: OnMigrate,
        T::Statements: for<'q> Expression<'q, S>,
        for<'c> &'c mut <S as sqlx::Database>::Connection: Executor<'c, Database = S>,
    {
        fn migrate(&self, pool: Pool<S>) -> Pin<Box<dyn Future<Output = ()>>> {
            let mut qb = StatementBuilder::default();
            self.statments().expression(&mut qb);

            Box::pin(async move {
                use_executor!(fetch_optional(&pool, qb)).unwrap();
            })
        }
    }

    pub async fn migrate<S: Database>(all: Vec<Box<dyn DynamicOnMigrate<S>>>, pool: &Pool<S>) {
        // let tables = sqlx::query_as::<_, (String,)>("SELECT name FROM sqlite_master")
        //     .fetch_all(&*pool)
        //     .await
        //     .unwrap();

        // if tables.is_empty().not() {
        //     panic!("migrate_on_empty_database function should only run on empty database");
        // }

        // let mut v = vec![];
        // v.extend(schema.collections.custom_migrate_statements());
        // v.extend(schema.links.custom_migrate_statements());

        // for each in v {
        //     sqlx::query(&each).execute(pool).await.unwrap();
        // }

        // sqlx::query("CREATE TABLE migration_history (version INTEGER)")
        //     .execute(&*pool)
        //     .await
        //     .unwrap();
        // sqlx::query("INSERT INTO migration_history (version) VALUES (0)")
        //     .execute(&*pool)
        //     .await
        //     .unwrap();

        for each in all {
            each.migrate(pool.clone()).await;
        }
    }
}
