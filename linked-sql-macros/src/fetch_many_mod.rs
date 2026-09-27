use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};

pub struct MainStatement {}

pub mod kws {
    use syn::custom_keyword;

    custom_keyword!(SELECT);
    custom_keyword!(FROM);
    custom_keyword!(LINK);
    custom_keyword!(ORDER);
    custom_keyword!(LIMIT);
    custom_keyword!(START_FROM);
}

impl Parse for MainStatement {
    fn parse(_: ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

pub fn main_statement_to_token(stmt: MainStatement) -> TokenStream2 {
    let _ = stmt;
    quote! {}
}
