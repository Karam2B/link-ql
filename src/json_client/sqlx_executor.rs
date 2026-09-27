use crate::json_client::client_interface::{
    AddCollectionInput, Client, ClientOperationError, ClientOperationInput, ClientOperationOutput,
    SupportedType,
};
use crate::json_client::dynamic_collection::DynamicCollection;
use crate::{
    database_extention::DatabaseExt,
    operations::operations_expressions_crossover::MigrateExpression,
    sqlx_query_builder::Expression,
};
use sqlx::{IntoArguments, Pool, Sqlite};
use std::collections::HashSet;
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tokio::sync::{RwLock as Trw, mpsc as tokio_mpsc};

pub struct SqlxExecutor<S>
where
    S: sqlx::Database + DatabaseExt,
{
    pub(crate) reciever: tokio::sync::mpsc::UnboundedReceiver<(
        ClientOperationInput,
        oneshot::Sender<Result<ClientOperationOutput, ClientOperationError>>,
    )>,
    pub(crate) data: Arc<SqlxExecutorData<S>>,
}

pub(crate) struct SqlxExecutorData<S>
where
    S: sqlx::Database + DatabaseExt,
{
    pub(crate) collections: Trw<HashMap<Arc<str>, Trw<Arc<DynamicCollection<S>>>>>,
    pub(crate) migration: Trw<Vec<String>>,
    pub(crate) link_info: Trw<LinkInformations>,
    pub(crate) pool: Pool<S>,
    _s: PhantomData<S>,
}
#[derive(Default, Debug)]
pub struct LinkInformations {
    pub one_to_many: HashSet<FromTo>,
    pub many_to_many: HashSet<FromTo>,
    pub timestamped: HashSet<Arc<str>>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FromTo {
    pub from: Arc<str>,
    pub to: Arc<str>,
}

impl Client {
    pub fn new_sqlx_db<S>(pool: Pool<S>) -> (Self, SqlxExecutor<S>)
    where
        S: DatabaseExt,
        bool: for<'d> sqlx::Decode<'d, S> + sqlx::Type<S> + for<'q> sqlx::Encode<'q, S>,
        std::string::String:
            sqlx::Type<S> + for<'q> sqlx::Encode<'q, S> + for<'d> sqlx::Decode<'d, S>,
        DynamicCollection<S>: MigrateExpression<Migrate: Expression<'static, S>>,
        for<'a> S::Arguments<'a>: IntoArguments<'a, S>,
    {
        let (sender, reciever) = tokio_mpsc::unbounded_channel::<(
            ClientOperationInput,
            oneshot::Sender<Result<ClientOperationOutput, ClientOperationError>>,
        )>();

        let data = Arc::new(SqlxExecutorData {
            collections: Trw::new(Default::default()),
            migration: Trw::new(Default::default()),
            link_info: Trw::new(Default::default()),
            pool,
            _s: PhantomData,
        });

        (Client { sender }, SqlxExecutor { reciever, data })
    }
}
