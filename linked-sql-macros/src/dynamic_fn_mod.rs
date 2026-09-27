#![allow(unused)]
#![warn(unused_must_use)]
use convert_case::Casing;
use proc_macro_error::{abort, abort_call_site};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, ItemFn, spanned::Spanned, visit_mut::VisitMut};

pub fn main(input: TokenStream) -> TokenStream {
    let mut ret = quote!();
    let mut item_fn = match syn::parse2::<syn::ItemFn>(input) {
        Ok(data) => data,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    // ret.extend(item_fn.to_token_stream());

    let handler = item_fn
        .sig
        .ident
        .to_string()
        .to_case(convert_case::Case::Snake);

    {
        let handler = Ident::new(&format!("{}_handler", handler), item_fn.sig.ident.span());
        ret.extend(quote!(pub struct #handler;));
    }

    {
        struct Checks;
        impl VisitMut for Checks {
            fn visit_generic_param_mut(&mut self, i: &mut syn::GenericParam) {
                abort!(i.span(), "generics are forbiden")
            }
            fn visit_item_mut(&mut self, i: &mut syn::Item) {
                abort!(i.span(), "don't place items inside dynamic function please")
            }
            fn visit_pat_type_mut(&mut self, i: &mut syn::PatType) {
                if (i.to_token_stream().to_string() != String::from("sdf")) {
                    abort!(i.span(), "should have client: PLG")
                }
            }
        }

        let mut s = Checks;
        s.visit_item_fn_mut(&mut item_fn);

        struct AddInfo(Vec<Ident>);
        impl VisitMut for AddInfo {
            fn visit_expr_field_mut(&mut self, i: &mut syn::ExprField) {
                let base = &i.base;
                if (base.to_token_stream().to_string() == "client") {
                    match &i.member {
                        syn::Member::Named(ident) => self.0.push(ident.clone()),
                        syn::Member::Unnamed(index) => {
                            abort!(i.member.span(), "don't use unambed member with ")
                        }
                    }
                    return;
                }

                syn::visit_mut::visit_expr_field_mut(self, i);
            }
        }

        let mut s = AddInfo(Default::default());
        s.visit_item_fn_mut(&mut item_fn);
        let inner = s.0;

        ret.extend(quote!(
            struct PLG {
                #(#inner: String,)*
            }
        ));
    }

    ret
}

fn test_1() {
    let input = quote!(
        fn dynamic_fn(client: PLH) {
            let title = client.todo;
        }
    );

    let output = main(input);

    let expected = quote!(
        fn dynamic_fn(client: PLH) {
            let title = client.todo;
        }

        pub struct dynamic_fn_handler;

        pub struct PLH {
            todo: String,
        }
    );

    assert_eq!(expected.to_string(), output.to_string());

    let input = quote!(
        fn ref_fn(client: PLF) {
            let done = client.done;
        }
        fn dynamic_fn(client: PLH) {
            let title = client.todo;
            ref_fn_handler::call(client)
        }
    );

    let expected = quote!(
        type PLF = Box<dyn __>;

        fn ref_fn(client: PLF) {
            let done = client.access("done");
        }

        pub struct ref_fn_handler;

        impl ref_fn_handler {
            const MEMEBERS = ["done"];
        }

        fn dynamic_fn(client: PLH) {
            let title = client.access("title");
        }

        pub struct dynamic_fn_handler;

        impl dynamic_fn_handler {
            const MEMBERS = ["title"]
        }
    );
}
