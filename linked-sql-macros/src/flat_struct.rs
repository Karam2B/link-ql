#![allow(unused)]
#![warn(unused_must_use)]
use proc_macro::Span;
use quote::ToTokens;
use std::{borrow::Cow, ops::Not};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream}, parse_quote, punctuated::Punctuated, token::Brace, Attribute, Field, FieldsNamed, Generics, ItemEnum, ItemStruct, Token, Type, Visibility
};
use syn::{Item as SynItem, parenthesized};

#[derive(Clone)]
pub struct Nest {
    pub vis: Visibility,
    pub attrs: Vec<Attribute>,
    pub ident: syn::Ident,
    pub generics: Generics,
}

pub enum Item {
    Struct(ItemStruct),
    Enum(ItemEnum),
}

pub struct ItemNestedStruct {
    pub result: Vec<Item>,
    pub semi_token: Option<Token![;]>,
}

fn parse_enum(
    enum_token: Token![enum],
    nest: Nest,
    input: ParseStream,
    output: &mut Vec<Item>,
) -> syn::Result<()> {
    output.push(Item::Enum(ItemEnum {
        attrs: nest.attrs,
        vis: nest.vis,
        enum_token,
        ident: nest.ident,
        generics: nest.generics,
        brace_token: todo!(),
        variants: todo!(),
    }));
    Ok(())
}

fn parse_struct(
    struct_token: Token![struct],
    nest: Nest,
    input: ParseStream,
    output: &mut Vec<Item>,
) -> syn::Result<()> {
    use syn::Fields;
    let fields = Fields::Unit;
    // || syn::__private::parse_brackets(&input).is_ok()
    if let Ok(s) = syn::__private::parse_parens(&input) {
        use syn::spanned::Spanned;
        return Err(syn::parse::Error::new(
            nest.ident.span(),
            "unamed structs are not supported yet!",
        ));
    }
    // TODO: hanle Fields::Unit!!
    fn parse_braces(input: ParseBuffer) -> Result<(Brace, ParseBuffer), syn::Error> {
        let p = syn::__private::parse_braces(&input)?;
        // let brace_token = p.token;
        // let input = p.content;
        Ok((p.token, p.content))
    }

        let p = syn::__private::parse_braces(&input)?;
        let brace_token = p.token;
        let input = p.content;
    let mut named = Punctuated::default();
    while input.is_empty().not() {
        let vis: Visibility = input.parse()?;
        let ident = input.parse()?;
        let colon_token = input.parse::<Token!(:)>()?; // named always have colon_token!
        let ty = match input.parse::<Type>() {
            Ok(ok) => ok,
            Err(_) => {
                if let Ok(s) = input.parse::<Token!(#)>() {
                    use syn::spanned::Spanned;
                    return Err(syn::Error::new(
                        s.span(),
                        "nested attributes are not supported yet!",
                    ));
                }
                match (
                    input.parse::<Token!(struct)>(),
                    input.parse::<Token!(enum)>(),
                ) {
                    (Ok(s), _) => {
                        let ident: syn::Ident = input.parse()?;
                        if let Ok(s) = input.parse::<Generics>() {
                            use syn::spanned::Spanned;
                            return Err(syn::Error::new(
                                s.span(),
                                "nested generics are not supported yet!",
                            ));
                        }
                        parse_struct(
                            s,
                            Nest {
                                vis: vis.clone(),
                                attrs: Vec::default(),
                                ident: ident.clone(),
                                generics: nest.generics.clone(),
                            },
                            &input,
                            output,
                        )?;
                        let return_ty: Type = parse_quote!(#ident);
                        return_ty
                    }
                    _ => {
                        todo!("nested enums are not supported yet")
                    }
                }
            }
        };
        named.push_value(Field {
            attrs: Vec::default(),
            vis,
            mutability: syn::FieldMutability::None,
            ident,
            colon_token: Some(colon_token),
            ty,
        });
        named.push_punct(input.parse()?);
    }
    let fields = Fields::Named(FieldsNamed { brace_token, named });
    output.push(Item::Struct(ItemStruct {
        attrs: nest.attrs,
        vis: nest.vis,
        struct_token,
        ident: nest.ident,
        generics: nest.generics,
        fields,
        semi_token: None,
    }));
    Ok(())
}

impl Parse for ItemNestedStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(s) = input.parse::<Token!(#)>() {
            use syn::spanned::Spanned;
            return Err(syn::Error::new(
                s.span(),
                "attributes are not supported yet!",
            ));
        }
        let mut result = Default::default();
        let vis = input.parse::<Visibility>()?;

        match (
            input.parse::<Token![struct]>(),
            input.parse::<Token![enum]>(),
        ) {
            (Ok(s), _) => {
                let ident = input.parse()?;
                let generics = input.parse()?;
                parse_struct(
                    s,
                    Nest {
                        vis,
                        attrs: Vec::default(),
                        ident,
                        generics,
                    },
                    input,
                    &mut result,
                )?;
            }
            (Err(_), Ok(s)) => {
                let ident = input.parse()?;
                let generics = input.parse()?;
                parse_enum(
                    s,
                    Nest {
                        vis,
                        attrs: Vec::default(),
                        ident,
                        generics,
                    },
                    input,
                    &mut result,
                )?;
            }
            (Err(mut e1), Err(e0)) => {
                e1.combine(e0);
                return Err(e1);
            }
        };

        let mut semi_token = input.parse()?;

        Ok(Self { result, semi_token })
    }
}

impl From<ItemNestedStruct> for proc_macro::TokenStream {
    fn from(value: ItemNestedStruct) -> Self {
        let semi_token = value.semi_token;
        let result = value.result;
        quote::quote!( #(#result #semi_token)*).into()
    }
}

impl ToTokens for ItemNestedStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let semi_token = self.semi_token;
        let result = &self.result;

        tokens.extend(quote::quote!( #(#result #semi_token)*))
    }
    fn into_token_stream(self) -> proc_macro2::TokenStream
    where
        Self: Sized,
    {
        let semi_token = self.semi_token;
        let result = self.result;
        quote::quote!( #(#result #semi_token)*)
    }
}

impl ToTokens for Item {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Item::Struct(i) => tokens.extend(i.into_token_stream()),
            Item::Enum(i) => tokens.extend(i.into_token_stream()),
        }
    }
}

#[cfg(test)]
mod flat_struct_test {
    use super::*;
    use quote::ToTokens;

    #[test]
    fn main() {
        let input = quote::quote!(
            struct Schema {
                name: String,
                pub tables: struct Hi {
                    sub: String
                }
            }
        );

        let expect = match syn::parse::<ItemNestedStruct>(input.into()) {
            Ok(data) => data.into_token_stream().to_string(),
            Err(err) => err.to_string(),
        };

        let to_be = quote::quote!(sdlkfj).to_string();

        pretty_assertions::assert_eq!(expect, to_be);
    }
}
