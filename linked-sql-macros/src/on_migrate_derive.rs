use case::CaseExt;
use proc_macro_error::abort;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub fn main(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(data) => data,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    #[derive(Default)]
    struct MainDerive {
        mem_name: Vec<Ident>,
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
                    self.mem_name.push(ident.clone());
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
    let name = Ident::new(&name.to_string().to_snake(), name.span());
    let members = Ident::new(
        &format!("{}_members", name.to_string().to_snake()),
        name.span(),
    );
    let MainDerive { mem_name } = md;

    quote! {
        const _ : () = {
            use ::linked_sql::prelude::on_migrate_derive::*;

            impl OnMigrate for #name {
                type Statements = CreateTable<
                    create_table,
                    table_as_expression<#name>,
                    (
                        <#name as Collection>::Id,
                        #(col_def_for_collection_member<#members::#mem_name>,)*
                    ),
                >;

                fn statments(&self) -> Self::Statements {
                    CreateTable {
                        init: create_table,
                        name: table_as_expression(#name),
                        col_defs: (
                            Collection::id(self).clone(),
                            #(col_def_for_collection_member(#members::#mem_name),)*
                        ),
                    }
                }
            }
        };
    }
}

#[test]
fn main_test() {
    let expect = quote! {
        pub struct Todo {
            pub title: String,
            pub done: bool,
            pub description: Option<String>,
        }
    };

    let expect = main(expect);

    let to_be = quote! {
        const _ : () = {
            use ::linked_sql::prelude::on_migrate_derive::*;

            impl OnMigrate for todo {
                type Statements = CreateTable<
                    create_table,
                    table_as_expression<todo>,
                    (
                        <todo as Collection>::Id,
                        col_def_for_collection_member<todo_members::title>,
                        col_def_for_collection_member<todo_members::done>,
                        col_def_for_collection_member<todo_members::description>,
                    ),
                >;

                fn statments(&self) -> Self::Statements {
                    CreateTable {
                        init: create_table,
                        name: table_as_expression(todo),
                        col_defs: (
                            Collection::id(self).clone(),
                            col_def_for_collection_member(todo_members::title),
                            col_def_for_collection_member(todo_members::done),
                            col_def_for_collection_member(todo_members::description),
                        ),
                    }
                }
            }
        };
    };

    crate::utils::expect_to_eq(expect, to_be);
}
