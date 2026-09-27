#![allow(unused)]
#![warn(unused_must_use)]
pub fn main(token: proc_macro::TokenStream, ident: syn::Ident) -> proc_macro2::TokenStream {

    token.into()
}
