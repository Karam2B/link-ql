use convert_case::{Case, Casing};
use proc_macro_error::abort;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Visibility;
use syn::{spanned::Spanned, visit::Visit};

pub fn main(input: TokenStream) -> TokenStream {
    let input = match syn::parse2::<syn::DeriveInput>(input) {
        Ok(data) => data,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };
    let mut ts = quote!();
    if let Visibility::Public(_) = input.vis.clone() {
    } else {
        abort!(input.vis.span(), "collections has to be public");
    }

    #[derive(Default)]
    struct MainDerive {
        mem_ty: Vec<syn::Type>,
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
                    ();
                    self.mem_ty.push(field.ty.clone());
                    self.mem_name.push(ident.clone());
                }
                None => {
                    abort!(field.span(), "unamed fields are not supported");
                }
            }
        }
    }

    let this = &input.ident;
    let this_lowercase = Ident::new(
        input.ident.to_string().to_case(Case::Snake).as_str(),
        input.ident.span(),
    );
    let partial = Ident::new(&format!("{}Partial", this), proc_macro2::Span::call_site());

    let mut main_derive = MainDerive::default();
    main_derive.visit_derive_input(&input);
    let MainDerive { mem_ty, mem_name } = main_derive;

    let serde_derive = if cfg!(feature = "serde") {
        Some(quote!(,::linked_sql::prelude::macro_derive_collection::Deserialize))
    } else {
        None
    };

    ts.extend(quote!(
        #[derive(Default, Debug #serde_derive)]
        pub struct #partial {
            #(pub #mem_name: ::linked_sql::prelude::macro_derive_collection::update<#mem_ty>,)*
        }

        #[derive(Clone, Default)]
        #[allow(non_camel_case_types)]
        pub struct #this_lowercase;
    ));

    let members_mod = Ident::new(
        &format!("{}_members", this_lowercase.to_string()),
        this.span(),
    );

    ts.extend(quote!(
        #[allow(non_camel_case_types)]
        pub mod #members_mod {
            use ::linked_sql::prelude::macro_derive_collection::*;
            use super::#this_lowercase;

            #(
                #[derive(Clone, Default)]
                pub struct #mem_name;
                impl MemberBasic for #mem_name {
                    fn name(&self) -> &str {
                        stringify!(#mem_name)
                    }
                }
                impl Member for #mem_name {
                    type CollectionHandler = #this_lowercase;
                    type Data = #mem_ty;
                }
            )*

            // because Collection::id = SigngleIncId
            pub struct id;
            impl MemberBasic for id {
                fn name(&self) -> &str {
                    SingleIncremintalInt.ident()
                }
            }
            impl Member for id {
                type CollectionHandler = #this_lowercase;
                type Data = <SingleIncremintalInt as Id>::Data;
            }
        }
    ));

    ts.extend(quote!( const _: () = {
        use ::linked_sql::prelude::macro_derive_collection::*;
        use #members_mod::*;

        impl CollectionBasic for  #this_lowercase {
            fn table_name_lower_case(&self) -> &'static str {
                stringify!(#this_lowercase)
            }
            fn table_name(&self) -> &'static str {
                stringify!(#this)
            }
        }

        impl Collection for #this_lowercase {
            type InputData = #this;
            type UpdateData = #partial;
            type OutputData = #this;
            type Id = SingleIncremintalInt;
            fn id(&self) -> &Self::Id {
                &SingleIncremintalInt
            }
        }

        impl HasHandler for #this {
            type Handler = #this_lowercase;
        }

        impl HasHandler for #partial {
            type Handler = #this_lowercase;
        }
        
        impl<S> Members<S> for #this_lowercase
        {
            fn members_names(&self) -> Vec<String> {
                vec![
                    #(stringify!(#mem_name).to_string(),)*
                ]
            }
        }
    };));

    if cfg!(feature = "inventory") {
        ts.extend(quote::quote!(
            const _: () = {
                use ::linked_sql::prelude::inventory::*;

                submit!(Migration { obj: || Box::new(#this_lowercase) });
                submit!(Collection { obj: || Box::new(#this_lowercase) });
            };
        ));
    }

    return ts;
}

#[test]
fn test_collection_derive() {
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
        #[derive(Default, Debug)]
        pub struct TodoPartial {
            pub title: ::linked_sql::prelude::macro_derive_collection::update<String>,
            pub done: ::linked_sql::prelude::macro_derive_collection::update<bool>,
            pub description: ::linked_sql::prelude::macro_derive_collection::update<Option<String> >,
        }

        #[derive(Clone, Default)]
        #[allow(non_camel_case_types)]
        pub struct todo;

        #[allow(non_camel_case_types)]
        pub mod todo_members {
            use ::linked_sql::prelude::macro_derive_collection::*;
            use super::todo;

            #[derive(Clone, Default)]
            pub struct title;
            impl MemberBasic for title {
                fn name(&self) -> &str {
                    stringify!(title)
                }
            }
            impl Member for title {
                type CollectionHandler = todo;
                type Data = String;
            }
            #[derive(Clone, Default)]
            pub struct done;
            impl MemberBasic for done {
                fn name(&self) -> &str {
                    stringify!(done)
                }
            }
            impl Member for done {
                type CollectionHandler = todo;
                type Data = bool;
            }
            #[derive(Clone, Default)]
            pub struct description;
            impl MemberBasic for description {
                fn name(&self) -> &str {
                    stringify!(description)
                }
            }
            impl Member for description {
                type CollectionHandler = todo;
                type Data = Option<String>;
            }

            pub struct id;
            impl MemberBasic for id {
                fn name(&self) -> &str {
                    "id"
                }
            }
            impl Member for id {
                type CollectionHandler = todo;
                type Data = <SingleIncremintalInt as Id>::Data;
            }
        }

        const _: () = {
            use ::linked_sql::prelude::macro_derive_collection::*;
            use todo_members::*;

            impl CollectionBasic for todo {
                fn table_name_lower_case(&self) -> &'static str {
                    stringify!(todo)
                }
                fn table_name(&self) -> &'static str {
                    stringify!(Todo)
                }
            }

            impl Collection for todo {
                type InputData = Todo;
                type UpdateData = TodoPartial;
                type OutputData = Todo;
                type Members = (title, done, description,);
                type Id = SingleIncremintalInt;
                fn members(&self) -> &Self::Members {
                    &(title, done, description,)
                }
                fn id(&self) -> &Self::Id {
                    &SingleIncremintalInt
                }
            }

            impl HasHandler for Todo {
                type Handler = todo;
            }

            impl HasHandler for TodoPartial {
                type Handler = todo;
            }

            impl<S> Members<S> for todo
            {
                fn members_names(&self) -> impl Iterator<Item = &str> {
                    [
                        stringify!(title),
                        stringify!(done),
                        stringify!(description),
                    ].into_iter()
                }
                fn members_as_boxed_expretions<'q>(&self) -> Vec<Box<dyn BoxedExpression<'q, S> + 'q>> {
                    vec![
                        Box::new(member_as_expression(title)),
                        Box::new(member_as_expression(done)),
                        Box::new(member_as_expression(description)),
                    ]
                }
            }
        };
    };

    assert_eq!(expect.to_string(), tobe.to_string());
}
