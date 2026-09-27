use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod utils;

mod on_migrate_derive;
#[proc_macro_derive(OnMigrate)]
#[proc_macro_error]
pub fn on_migrate(input: TokenStream) -> TokenStream {
    on_migrate_derive::main(input.into()).into()
}

mod collection_derive;
#[proc_macro_derive(Collection)]
#[proc_macro_error]
pub fn collection(input: TokenStream) -> TokenStream {
    collection_derive::main(input.into()).into()
}

mod from_row_alias_derive;
#[proc_macro_derive(FromRowAlias)]
#[proc_macro_error]
pub fn from_row_alias(input: TokenStream) -> TokenStream {
    from_row_alias_derive::main(input.into()).into()
}

mod simple_enum_mod;
#[proc_macro_attribute]
#[proc_macro_error]
pub fn simple_enum(_: TokenStream, input: TokenStream) -> TokenStream {
    simple_enum_mod::main(input.into()).into()
}

mod sql_mod;
#[proc_macro]
#[proc_macro_error]
pub fn sql(input: TokenStream) -> TokenStream {
    match syn::parse::<sql_mod::MainStatement>(input) {
        Ok(ok) => sql_mod::main_statment_to_token(ok).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

mod fetch_many_mod;
#[proc_macro]
#[proc_macro_error]
pub fn fetch_many(input: TokenStream) -> TokenStream {
    match syn::parse::<fetch_many_mod::MainStatement>(input) {
        Ok(ok) => fetch_many_mod::main_statement_to_token(ok).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

mod skip_mod;
#[proc_macro_attribute]
pub fn skip(_: TokenStream, input: TokenStream) -> TokenStream {
    skip_mod::main(input.into()).into()
}

mod async_is_send_mod;

/// valid syntax:
/// ```rust
/// #[linked_sql_macros::async_is_send]
/// async fn test() {
///     todo!()
/// }
/// ```
///
/// will expand to:
/// ```rust
/// fn test() -> impl Future + Send {
///     async move {
///         todo!()
///     }
/// }
/// ```
///
/// you can also pass a block to the attribute to run before the async move block:
///
/// ```rust
/// #[linked_sql_macros::async_is_send(before_move = {
///     let arg = arg.to_string();
/// })]
/// async fn test(arg: &str) {}
/// ```
#[proc_macro_attribute]
pub fn async_is_send(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr_output = match async_is_send_mod::before_move(attr.into()) {
        Ok(ok) => ok,
        Err(err) => return err.into(),
    };
    match syn::parse::<syn::ItemFn>(input) {
        Ok(ok) => async_is_send_mod::main(ok, attr_output).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

mod data_flow_mod;
