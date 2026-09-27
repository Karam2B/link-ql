/// Singleton Types -- they only have one instance,
/// that can be either a zero-sized type, or type that was only
/// initialized in a static context.
///
/// examples:
///
/// ```no_run
///     use linked_sql::singleton::Singleton;
///
///     struct SingletonType;
///     impl Singleton for SingletonType {
///         fn singleton() -> &'static Self {
///             &SingletonType
///         }
///     }
/// ```
///
/// ```no_run
///     use linked_sql::singleton::Singleton;
///     use std::sync::LazyLock;
///
///     struct SingletonType(String);
///
///     static SINGLETON_INSTANCE: LazyLock<SingletonType> =
///         LazyLock::new(|| SingletonType(String::from("the one and only instance")));
///
///     impl Singleton for SingletonType {
///         fn singleton() -> &'static Self {
///             &SINGLETON_INSTANCE
///         }
///     }
/// ```
///
/// example of how this is used in in this crate
//
/// ```ignore
///     fn collection_basic<T: linked_sql::collections::Collection>(_: T) {}
///

//
///     fn main() {
///         use linked_sql::test_module::{todo, Todo};
///         use std::marker::PhantomData;
//
///         // these are the same calls because todo is a Singleton
///         collection_basic(todo);
///         collection_basic(PhantomData::<Todo>);
///     }
/// ```
///
/// can be usefull to get static strings from generic contexts,
/// in this example `Collection::table_name` is `fn(&self) -> &str`,
/// output string it can have any lifetime, but because of `Singleton` trait,
/// we can get a `&'static str` in any `T: Collection + Singleton` context.
///
/// ```no_run
///     use linked_sql::collections::Collection;
///     use linked_sql::singleton::Singleton;
///
///     fn access_static_str_in_generic_contexts<T: Collection + Singleton>() {
///         let _name: &'static str = T::singleton().table_name();
///     }
/// ```
pub trait Singleton: 'static {
    fn singleton() -> &'static Self;
}

mod mut_singleton {
    //! singleton that can be mutated, I don't know if there
    //! is any use case for this, it can be only implemented for
    //! zero-sized types, which don't have any mutating state.
    //!
    //! Additionally, mutating static data can be done via a mutex
    //! behind a shared reference

    use crate::singleton::Singleton;

    pub trait MutSingleton: 'static {
        fn mut_singleton() -> &'static mut Self;
    }

    impl<T> Singleton for T
    where
        T: MutSingleton,
    {
        fn singleton() -> &'static Self {
            T::mut_singleton()
        }
    }
}

impl<T: 'static> Singleton for std::marker::PhantomData<T> {
    fn singleton() -> &'static Self {
        &std::marker::PhantomData
    }
}

pub mod impl_default {
    use crate::singleton::Singleton;

    /// Clonable Singletons are Default
    ///
    /// sometime `&'static T`` is more powerfull than `T`
    /// because it is not bound to a let statement,
    /// which means more flexible with respect to lifetimes.
    pub struct ClonableSingleton<T>(pub T);

    impl<T> Default for ClonableSingleton<T>
    where
        T: Clone + Singleton,
    {
        #[inline(always)]
        fn default() -> Self {
            Self(T::singleton().clone())
        }
    }
}

#[linked_sql_macros::skip]
// unstable interface for ValidateCollection
pub mod impl_collectinos {
    use super::Singleton;
    use crate::collections::{Collection, HasHandler, ValidateCollection};
    use std::marker::PhantomData;

    impl<T> ValidateCollection for PhantomData<T>
    where
        T: HasHandler,
        T::Handler: Collection + Singleton + ValidateCollection,
    {
        type UpdateInput = <T::Handler as ValidateCollection>::UpdateInput;
        type UpdateError = <T::Handler as ValidateCollection>::UpdateError;
        fn validate_on_update(
            &self,
            input: Self::UpdateInput,
        ) -> Result<Self::UpdateData, Self::UpdateError> {
            T::Handler::singleton().validate_on_update(input)
        }
        type DataInput = <T::Handler as ValidateCollection>::DataInput;
        type DataError = <T::Handler as ValidateCollection>::DataError;
        fn validate_on_insert(
            &self,
            input: Self::DataInput,
        ) -> Result<Self::InsertData, Self::DataError> {
            T::Handler::singleton().validate_on_insert(input)
        }
    }

    impl<T> Collection for PhantomData<T>
    where
        T: HasHandler,
        T::Handler: Collection + Singleton,
    {
        type InputData = <T::Handler as Collection>::InputData;
        type UpdateData = <T::Handler as Collection>::UpdateData;
        type OutputData = <T::Handler as Collection>::OutputData;

        type Id = <T::Handler as Collection>::Id;

        fn id(&self) -> Self::Id {
            T::Handler::id(Singleton::singleton())
        }

        fn table_name(&self) -> &str {
            T::Handler::singleton().table_name()
        }

        fn table_name_lower_case(&self) -> &str {
            T::Handler::singleton().table_name_lower_case()
        }
    }
}
