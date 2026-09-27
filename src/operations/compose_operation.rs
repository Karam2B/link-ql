use std::marker::PhantomData;

use sqlx::Database;

use crate::operations::{Operation, OperationOutput};

pub struct ComposeOperation<TupleOp> {
    holding: PhantomData<TupleOp>,
}

impl<Op0, Op1> OperationOutput for ComposeOperation<(Op0, Op1)> {
    type Output = ();
}

impl<S, Op0, Op1> Operation<S> for ComposeOperation<(Op0, Op1)>
where
    S: Database,
    Op0: Send,
    Op1: Send,
{
    async fn exec_operation(self, _pool: &mut S::Connection) -> Self::Output {
        // let (to_first, to_second) = ..();
        // let (to_output, to_second) = self.0.0.exec_operation(pool).await;
        // let second_op = self.0.1.exec_operation(pool).await;

        // (first_op, second_op)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::ComposeOperation;
    use crate::{
        connect_in_memory::ConnectInMemory, operations::Operation,
        track_sqlx_query::watch_sqlx_calls,
    };
    use sqlx::Sqlite;

    #[tokio::test]
    async fn test_compose_operation() {
        watch_sqlx_calls(async |actions| {
            let mut conn = Sqlite::in_memory_connection().await;

            let _op = Operation::<Sqlite>::exec_operation(
                ComposeOperation {
                    holding: PhantomData::<((), ())>,
                },
                &mut conn,
            )
            .await;

            pretty_assertions::assert_eq!(actions.take(), vec!["PRAGMA foreign_keys = ON;"]);
        })
        .await;
    }
}
