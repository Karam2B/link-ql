use case::CaseExt;
use proc_macro_error::abort;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, visit::Visit};

pub fn main(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(data) => data,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    #[derive(Default)]
    struct MainDerive {
        mem_ty: Vec<syn::Type>,
        mem_name: Vec<Ident>,
        mem_pre: Vec<String>,
    }

    impl Visit<'_> for MainDerive {
        fn visit_generics(&mut self, i: &syn::Generics) {
            if i.lt_token.is_some() {
                abort!(i.span(), "geneerics are not supported");
            }
        }

        fn visit_field(&mut self, field: &syn::Field) {
            match field.ident.as_ref() {
                Some(ident) => {
                    self.mem_ty.push(field.ty.clone());
                    self.mem_name.push(ident.clone());
                    self.mem_pre.push(format!("{{}}{}", ident.to_string()));
                }
                None => {
                    abort!(field.span(), "unamed fields are not supported");
                }
            }
        }
    }

    let mut md = MainDerive::default();
    md.visit_derive_input(&input);
    let name = input.ident;
    let name_lower_case = Ident::new(name.to_string().to_snake().as_str(), name.span());
    let MainDerive {
        mem_ty,
        mem_name,
        mem_pre,
    } = md;

    return quote! {
        const _: () = {
            use ::linked_sql::prelude::from_row_alias::*;

            impl<'r, R: Row> FromRowAlias<'r, R> for #name_lower_case
            where
                #(
                    #mem_ty: Type<R::Database> + Decode<'r, R::Database>,
                )*
                for<'a> &'a str: ColumnIndex<R>,
            {
                fn no_alias(&self, row: &'r R) -> Result<Self::Data, FromRowError> {
                    Ok(#name {
                        #(
                            #mem_name: row
                                .try_get(stringify!(#mem_name))?,
                        )*
                    })
                }

                fn str_alias(
                    &self,
                    row: str_alias<'r, R>,
                ) -> Result<Self::Data, FromRowError>
                {
                    Ok(#name {
                        #(
                            #mem_name: row.0
                                .try_get(format!(#mem_pre, row.1).as_str())?,
                        )*
                    })
                }
            }
        };
    };
}

#[test]
fn test_from_row_ext_derive() {
    use pretty_assertions::assert_eq;
    let mut expect = quote! {
        pub struct Todo {
            pub title: String,
            pub done: bool,
            pub description: Option<String>,
        }
    };

    let expect = main(expect);

    let tobe = quote! {
        const _: () = {
            use ::linked_sql::prelude::from_row_ext::*;

            impl<'r, R: Row> FromRowExt<'r, R> for Todo
            where
                String: Type<R::Database> + Decode<'r, R::Database>,
                bool: Type<R::Database> + Decode<'r, R::Database>,
                Option<String>: Type<R::Database> + Decode<'r, R::Database>,
                for<'a> &'a str: ColumnIndex<R>,
            {
                fn from_row_no_alias(row: &'r R) -> Result<Self, Error>
                where
                    Self: Sized,
                {
                    Ok(Todo {
                        title: row.try_get(stringify!(title))?,
                        done: row.try_get(stringify!(done))?,
                        description: row.try_get(stringify!(description))?,
                    })
                }
                fn from_row_pre_aliased(row: &'r R, alias: &str) -> Result<Self, Error>
                where
                    Self: Sized,
                {
                    Ok(Todo {
                        title: row.try_get(format!("{}title", alias).as_str())?,
                        done: row.try_get(format!("{}done", alias).as_str())?,
                        description: row.try_get(format!("{}description", alias).as_str())?,
                    })
                }
            }
        };
    };

    assert_eq!(expect.to_string(), tobe.to_string());
}
