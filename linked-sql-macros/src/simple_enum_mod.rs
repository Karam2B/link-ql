use convert_case::Casing;
use proc_macro_error::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Field, ItemEnum, parse_quote, punctuated::Punctuated, spanned::Spanned, visit_mut::VisitMut,
};

pub fn main(input: TokenStream) -> TokenStream {
    let mut input = match syn::parse2::<ItemEnum>(input) {
        Ok(data) => data,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    #[derive(Default)]
    struct Variants {
        units: Vec<syn::Ident>,
        units2: Vec<syn::Ident>,
        ones: Vec<(syn::Ident, syn::Type)>,
    }

    impl VisitMut for Variants {
        fn visit_variant_mut(&mut self, i: &mut syn::Variant) {
            let ident = i.ident.clone();
            match &i.fields {
                syn::Fields::Unit => {
                    let f: Field = parse_quote!(#ident);
                    i.fields = syn::Fields::Unnamed(syn::FieldsUnnamed {
                        paren_token: Default::default(),
                        unnamed: Punctuated::from_iter([f]),
                    });
                    self.units.push(ident);
                }
                syn::Fields::Named(fields_named) => {
                    abort!(
                        fields_named.span(),
                        "nammed fields cannot be supported for simple enums!"
                    );
                }
                syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
                    let ty = fields_unnamed.unnamed.first().unwrap().ty.clone();
                    self.ones.push((ident, ty));
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    abort!(
                        fields_unnamed.span(),
                        "unnamed fields has to have one field"
                    );
                }
            }
        }
    }

    let mut v = Variants::default();
    v.visit_item_enum_mut(&mut input);
    let units = v.units;
    let units2 = v.units2;
    let ones_ident = v
        .ones
        .iter()
        .map(|(ident, _)| ident.clone())
        .collect::<Vec<_>>();
    let ones_ty = v.ones.iter().map(|(_, ty)| ty.clone()).collect::<Vec<_>>();
    let this = &input.ident;

    let mut ret = quote! {
        #input
    };
    let mut macros = Vec::<(syn::Ident, syn::Ident)>::new();

    {
        let macro_name = syn::Ident::new(
            &format!(
                "{}_is_subset_of",
                this.to_string().to_case(convert_case::Case::Snake)
            ),
            this.span(),
        );

        ret.extend(quote! {
            #[macro_export]
            macro_rules! #macro_name {
                ($of:ident) => {
                    impl From<#this> for $of {
                        fn from(value: #this) -> Self {
                            match value {
                                #(#this::#units(v) => Self::#units(v)),*
                                #(#this::#units2() => Self::#units2()),*
                                #(#this::#ones_ident(v) => Self::#ones_ident(v)),*
                            }
                        }
                    }
                };
            }
        });

        macros.push((macro_name, syn::Ident::new("is_subset_of", this.span())));
    }

    {
        let macro_name = syn::Ident::new(
            &format!(
                "{}_closure",
                this.to_string().to_case(convert_case::Case::Snake)
            ),
            this.span(),
        );

        ret.extend(quote! {
            #[macro_export]
            macro_rules! #macro_name {
                ($this:expr, |$new_ident:ident| $method:tt) => {
                    match $this {
                        #(#this::#units(v) => (|$new_ident: #units| $method)(v),)*
                        #(#this::#ones_ident(v) => (|$new_ident: #ones_ty| $method)(v)),*
                    }
                };
            }
        });

        macros.push((macro_name, syn::Ident::new("closure", this.span())));
    }

    {
        let mod_name = syn::Ident::new(
            &format!(
                "{}_macros",
                this.to_string().to_case(convert_case::Case::Snake)
            ),
            this.span(),
        );
        let (macro_, alias) = macros
            .into_iter()
            .unzip::<_, _, Vec<syn::Ident>, Vec<syn::Ident>>();

        ret.extend(quote!(
            pub mod #mod_name {
                #(pub use #macro_ as #alias ;)*
            }
        ));
    }

    for each in units {
        ret.extend(quote! {
            impl From<#each> for #this {
                fn from(value: #each) -> Self {
                    Self::#each(value)
                }
            }
        });
    }

    for (each, ty) in ones_ident.iter().zip(ones_ty.iter()) {
        ret.extend(quote! {
            impl From<#ty> for #this {
                fn from(value: #ty) -> Self {
                    Self::#each(value)
                }
            }
        });
    }

    ret
}

#[test]
pub fn failing() {
    let expect = quote!(
        #[derive(Debug)]
        enum Bar {
            Hi(String),
        }
    );

    // todo: create test_entry_point
    let expect = proc_macro_error::entry_point(|| main(expect).into(), true);

    pretty_assertions::assert_eq!(
        quote!(::core::compile_error! {"expected `enum`"}).to_string(),
        expect.to_string()
    );
}

#[test]
fn simple_enum_test() {
    let expect = quote!(
        #[derive(Debug)]
        enum Bar {
            I32(i32),
            String,
            Custom,
        }
    );

    let expect = main(expect);
    let to_be = quote! {
        #[derive(Debug)]
        enum Bar {
            I32(i32),
            String(String),
            Custom(Custom),
        }

        #[macro_export]
        macro_rules! bar_is_subset_of {
            ($of:ident) => {
                impl From<Bar> for $of {
                    fn from(value: Bar) -> Self {
                        match value {
                            Bar::I32(v) => Self::I32(v),
                            Bar::String(v) => Self::String(v),
                            Bar::Custom(v) => Self::Custom(v),
                        }
                    }
                }
            };
        }

        #[macro_export]
        macro_rules! bar_closure {
            ($this:expr, |$new_ident:ident| $method:tt) => {
                match $this {
                    Bar::I32(v) => (|$new_ident: i32| $method)(v),
                    Bar::String(v) => (|$new_ident: String| $method)(v),
                    Bar::Custom(v) => (|$new_ident: Custom| $method)(v),
                }
            };
        }

        pub mod bar_macros {
            pub use bar_is_subset_of as is_subset_of;
            pub use bar_closure as closure;
        }

        impl From<i32> for Bar {
            fn from(value: i32) -> Self {
                Self::I32(value)
            }
        }

        impl From<String> for Bar {
            fn from(value: String) -> Self {
                Self::String(value)
            }
        }

        impl From<Custom> for Bar {
            fn from(value: Custom) -> Self {
                Self::Custom(value)
            }
        }
    };

    pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string());
}
