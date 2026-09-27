use std::ops::Not;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{ReturnType, parse_quote, spanned::Spanned};

type AttrOutput = Vec<syn::Stmt>;

pub fn main(mut input: syn::ItemFn, before_move: AttrOutput) -> TokenStream {
    if input.sig.asyncness.is_none() {
        return quote_spanned! {input.sig.fn_token.span()=> compile_error!("fn must be async")};
    }

    input.sig.asyncness = None;
    input.sig.output = match input.sig.output {
        ReturnType::Default => parse_quote! { -> impl Future + Send },
        ReturnType::Type(_, ty) => parse_quote! { -> impl Future<Output = #ty> + Send },
    };

    let stmts = input.block.stmts;
    input.block = parse_quote! {
         {
            #(#before_move)*
            async move {
                #(#stmts)*
            }
         }
    };

    quote! { #input }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let expect = parse_quote! {
            async fn test() {
                todo!()
            }
        };

        let expect = main(expect, Vec::new());

        let to_be = quote! {
            fn test() -> impl Future + Send {
                async move {
                    todo!()
                }
            }
        };

        pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string());

        let expect = parse_quote! {
            async fn test() -> Result<(), String> {
                Ok(())
            }
        };

        let expect = main(expect, Vec::new());

        let to_be = quote! {
            fn test() -> impl Future<Output = Result<(), String> > + Send {
                async move {
                    Ok(())
                }
            }
        };

        pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string());
    }
    #[test]
    fn test_2() {
        let expect = parse_quote! {
            async fn test(
                arg: &str,
            ) -> Result<(), String> {
                Ok(())
            }
        };
        let attr = quote! {
            before_move = {
                let arg = arg.clone();
            }
        };

        let expect = main(expect, before_move(attr).unwrap());

        let to_be = quote! {
            fn test(arg: &str,) -> impl Future<Output = Result<(), String> > + Send {
                let arg = arg.clone();
                async move {
                    Ok(())
                }
            }
        };

        pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string());
    }
}

pub fn before_move(attr: TokenStream) -> Result<AttrOutput, TokenStream> {
    let s = match syn::parse2::<syn::MetaNameValue>(attr) {
        Ok(ok) => ok,
        Err(_) => return Ok(Vec::new()),
    };

    if s.path.get_ident().map(|e| e.to_string()) != Some("before_move".to_string()) {
        return Err(
            quote_spanned! {s.path.span()=> compile_error!("expected `before_move` attribute")},
        );
    }
    let extract = match s.value {
        syn::Expr::Block(ok) => {
            if ok.attrs.is_empty().not() {
                return Err(
                    quote_spanned! {ok.attrs.first().unwrap().span()=> compile_error!("`before_move` block should not have attributes")},
                );
            }
            if ok.label.is_some() {
                return Err(
                    quote_spanned! {ok.label.unwrap().span()=> compile_error!("`before_move` block should not have a label")},
                );
            }
            ok.block.stmts
        }
        _ => {
            return Err(
                quote_spanned! {s.value.span()=> compile_error!("`before_move` should be a block")},
            );
        }
    };
    Ok(extract)
}
