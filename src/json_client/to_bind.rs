use crate::is_null::IsNull as IsNullTrait;
use sqlx::Database;
use sqlx::Encode;
use sqlx::Type;
use sqlx::encode::IsNull as IsNullEnum;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;

pub trait ToBind<S: Database>: Send + Sync {
    fn clone_to_box<'q>(&self) -> Box<dyn ToBind<S> + Send + 'q>;
    fn bind_ref<'q>(&self, buf: &mut S::ArgumentBuffer<'q>) -> Result<IsNull, BoxDynError>;
    fn bind_boxed<'q>(
        self: Box<Self>,
        buf: &mut <S as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError>;
    fn is_null(&self) -> bool;
}

impl<S: Database> ToBind<S> for () {
    fn clone_to_box<'q>(&self) -> Box<dyn ToBind<S> + Send + 'q> {
        Box::new(())
    }
    fn bind_ref<'q>(
        &self,
        _: &mut <S as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNullEnum, BoxDynError> {
        Ok(IsNullEnum::Yes)
    }

    fn bind_boxed<'q>(
        self: Box<Self>,
        _: &mut <S as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNullEnum, BoxDynError> {
        Ok(IsNullEnum::Yes)
    }

    fn is_null(&self) -> bool {
        true
    }
}

impl<S, T> ToBind<S> for T
where
    T: Clone,
    T: IsNullTrait,
    T: Send + Sync,
    S: Database,
    T: for<'q> Encode<'q, S> + Type<S> + 'static,
{
    fn clone_to_box<'q>(&self) -> Box<dyn ToBind<S> + Send + 'q> {
        Box::new(self.clone())
    }
    fn is_null(&self) -> bool {
        T::is_null()
    }
    fn bind_ref<'q>(
        &self,
        buf: &mut <S as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        Encode::encode_by_ref(self, buf)
    }
    fn bind_boxed<'q>(
        self: Box<Self>,
        buf: &mut <S as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        Encode::encode(*self, buf)
    }
}

impl<S: Database> Clone for Box<dyn ToBind<S> + Send> {
    fn clone(&self) -> Self {
        self.clone_to_box()
    }
}

impl<S> Type<S> for Box<dyn ToBind<S> + Send>
where
    S: sqlx::Database,
{
    fn type_info() -> <S as Database>::TypeInfo {
        panic!(
            "
                sqlx is not built around my style of coding, 
                if I don't have access to self, there is no way to get the type info.
                Also, I don't think this is relavent if I'm not using sqlx::query macro
            "
        )
    }
}

impl<'q, S> Encode<'q, S> for Box<dyn ToBind<S> + Send>
where
    S: Database,
{
    fn encode_by_ref(
        &self,
        buf: &mut <S as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        self.bind_ref(buf)
    }
    fn encode(
        self,
        buf: &mut <S as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
    where
        Self: Sized,
    {
        self.bind_boxed(buf)
    }
}

mod expression_impls {
    use super::*;
    use crate::database_extention::DatabaseExt;
    use crate::sqlx_query_builder::Expression;
    use crate::sqlx_query_builder::OpExpression;
    use crate::sqlx_query_builder::StatementBuilder;
    use sqlx::Database;

    impl<S> OpExpression for Box<dyn ToBind<S> + Send> {
}
    impl<'q, S> Expression<'q, S> for Box<dyn ToBind<S> + Send>
    where
        S: Database + DatabaseExt,
    {
        fn expression(self, ctx: &mut StatementBuilder<'q, S>)
        where
            S: DatabaseExt,
        {
            if self.is_null() {
                ctx.syntax(&"NULL");
            } else {
                ctx.bind(self);
            }
        }
    }
}
