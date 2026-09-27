use syn::{
    parse::ParseBuffer,
    token::{Brace, Bracket, Paren},
};

#[cfg(test)]
#[track_caller]
pub fn expect_to_eq(expect: proc_macro2::TokenStream, to_be: proc_macro2::TokenStream) {
    pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string())
}

pub fn parse_parens<'a>(
    input: syn::parse::ParseStream<'a>,
) -> syn::Result<(Paren, ParseBuffer<'a>)> {
    syn::__private::parse_parens(input).map(|e| return (e.token, e.content))
}

pub fn _parse_brackets<'a>(
    input: syn::parse::ParseStream<'a>,
) -> syn::Result<(Bracket, ParseBuffer<'a>)> {
    syn::__private::parse_brackets(input).map(|e| return (e.token, e.content))
}

pub fn _parse_braces<'a>(
    input: syn::parse::ParseStream<'a>,
) -> syn::Result<(Brace, ParseBuffer<'a>)> {
    syn::__private::parse_braces(input).map(|e| return (e.token, e.content))
}
