//! this module is to replace the old operations module
//! the difference is this is more composible that the old one.
//! and it distinguish operation that return streams from the one that return only one record.
//!
//! in addition, part of the refactoring is that the new interface
//! depends on the new sqlx_query_builder/valid_syntax module. while the
//! old one depends on the old operation_expressions_crossover module.
use futures::Future;
use futures::Stream;
use sqlx::Database;

pub trait OperationOutput {
    type Output;
}
pub trait Operation<S>: OperationOutput<Output: Send> + Send {
    fn exec_operation(self, conn: &mut S::Connection) -> impl Future<Output = Self::Output> + Send
    where
        Self: Sized,
        S: Database;
}

pub trait StreamedOperation<S>: OperationOutput<Output: Send> + Send {
    fn stream_operation(self, conn: &mut S::Connection) -> impl Stream<Item = Self::Output> + Send
    where
        Self: Sized,
        S: Database;
}

pub mod streamed_ops {
    use std::{collections::HashMap, hash::Hash, pin::pin};

    use futures::StreamExt;

    use crate::operations::v2::{Operation, OperationOutput, StreamedOperation};

    pub struct OperationVec<T>(pub T);

    impl<T> OperationOutput for OperationVec<T>
    where
        T: OperationOutput,
    {
        type Output = Vec<T::Output>;
    }

    impl<S, T> Operation<S> for OperationVec<T>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            Self: Sized,
            S: sqlx::Database,
        {
            async move {
                let mut stream = pin!(self.0.stream_operation(conn));
                let mut output = Vec::new();
                while let Some(item) = stream.next().await {
                    output.push(item);
                }
                output
            }
        }
    }

    pub struct OperationHashed<T>(pub T);

    impl<T, Key, Value> OperationOutput for OperationHashed<T>
    where
        T: OperationOutput<Output = (Key, Value)>,
    {
        type Output = HashMap<Key, Vec<Value>>;
    }

    impl<S, T, Key, Value> Operation<S> for OperationHashed<T>
    where
        T: OperationOutput<Output = (Key, Value)>,
        T: StreamedOperation<S>,
        Value: Send,
        Key: Send + Eq + Hash,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send
        where
            Self: Sized,
            S: sqlx::Database,
        {
            async move {
                let mut stream = pin!(self.0.stream_operation(conn));
                let mut output = HashMap::new();
                while let Some((fi, ti)) = stream.next().await {
                    output.entry(fi).or_insert_with(|| Vec::new()).push(ti)
                }
                output
            }
        }
    }

    pub struct PanicOnFailure;
    pub struct ResultOnFailure;

    pub struct OneRecordOperation<T, Infalibility> {
        pub operation: T,
        pub infalibility: Infalibility,
    }

    impl<T> OperationOutput for OneRecordOperation<T, PanicOnFailure>
    where
        T: OperationOutput,
    {
        type Output = Option<T::Output>;
    }

    impl<S, T> Operation<S> for OneRecordOperation<T, PanicOnFailure>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
        S: sqlx::Database,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let mut stream = pin!(self.operation.stream_operation(conn));

                let output = stream.next().await;

                if stream.next().await.is_some() {
                    panic!("expected only one record, but got multiple");
                }

                output
            }
        }
    }

    impl<T> OperationOutput for OneRecordOperation<T, ResultOnFailure>
    where
        T: OperationOutput,
    {
        type Output = Result<Option<T::Output>, ()>;
    }

    impl<S, T> Operation<S> for OneRecordOperation<T, ResultOnFailure>
    where
        T: OperationOutput,
        T: StreamedOperation<S>,
        S: sqlx::Database,
    {
        fn exec_operation(
            self,
            conn: &mut <S>::Connection,
        ) -> impl Future<Output = Self::Output> + Send {
            async move {
                let mut stream = pin!(self.operation.stream_operation(conn));

                let output = stream.next().await;

                if stream.next().await.is_some() {
                    return Err(());
                }

                Ok(output)
            }
        }
    }
}
