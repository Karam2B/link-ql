use convert_case::{Case, Casing};
use proc_macro_error::abort;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident,
    parse::{Parse, ParseStream},
};

pub struct Input {
    ident: syn::Ident,
    rest: TokenStream,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        let rest = input.parse::<TokenStream>()?;

        Ok(Self { ident, rest })
    }
}

pub struct TwoIdent {
    from: Ident,
    to: Ident,
}

impl Parse for TwoIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            from: input.parse()?,
            to: input.parse()?,
        })
    }
}

pub fn many_to_many(rest: TwoIdent) -> TokenStream {
    let mut q = quote::quote!();
    let to_lower_case = Ident::new(
        rest.to.to_string().to_case(Case::Snake).as_str(),
        rest.to.span(),
    );
    // let to = rest.to;
    let from_lower_case = Ident::new(
        rest.from.to_string().to_case(Case::Snake).as_str(),
        rest.from.span(),
    );

    if cfg!(feature = "inventory") {
        q = quote::quote!(
            const _: () = {
                use ::linked_sql::prelude::inventory::*;

                submit!(Migration { obj: || Box::new(Relation(from: #from_lower_case, to: #to_lower_case)) });
                submit!(Migration { obj: || Box::new(Relation(to: #to_lower_case, from: #from_lower_case)) });
                submit!(Link { obj: || Box::new(Relation(from: #from_lower_case, to: #to_lower_case)) });
                submit!(Link { obj: || Box::new(Relation(to: #to_lower_case, from: #from_lower_case)) });
            };
        );
    }

    quote! {
        const _: () = {
            use ::linked_sql::prelude::macro_relation::*;
            impl LinkData<#from_lower_case> for Relation<#from_lower_case, #to_lower_case> {
                type Spec = ManyToMany<#from_lower_case, #to_lower_case>;
                fn spec(self, table_1: #from_lower_case) -> Self::Spec {
                    let junction = format!(
                        "{start}{end}",
                        start = table_1.table_name(),
                        end = self.to.table_name()
                    );
                    ManyToMany {
                        junction,
                        id_1: format!("{}_id", stringify!(#from_lower_case)),
                        table_1,
                        id_2: format!("{}_id", stringify!(#to_lower_case)),
                        table_2: self.to,
                    }
                }
            }
            impl LinkData<#to_lower_case> for Relation<#to_lower_case, #from_lower_case> {
                type Spec = ManyToMany<#to_lower_case, #from_lower_case>;
                fn spec(self, table_1: #to_lower_case) -> Self::Spec {
                    let junction = format!(
                        "{start}{end}",
                        end = table_1.table_name(),
                        start = self.to.table_name()
                    );
                    ManyToMany {
                        junction,
                        id_1: format!("{}_id", stringify!(#to_lower_case)),
                        table_1,
                        id_2: format!("{}_id", stringify!(#from_lower_case)),
                        table_2: self.to,
                    }
                }
            }
        };

        #q
    }
}

pub fn one_to_many(rest: TwoIdent) -> TokenStream {
    let to_lower_case = Ident::new(
        rest.to.to_string().to_case(Case::Snake).as_str(),
        rest.to.span(),
    );
    let to = rest.to;
    let from_lower_case = Ident::new(
        rest.from.to_string().to_case(Case::Snake).as_str(),
        rest.from.span(),
    );
    let foriegn_key = format!("{}_id", to.to_string().to_case(Case::Snake));
    quote! {
        const _: () = {
            use ::linked_sql::prelude::macro_relation::*;
            impl LinkData<#to_lower_case> for Relation<#to_lower_case, #from_lower_case> {
                type Spec = OneToManyInverse<#to_lower_case, #from_lower_case>;
                fn spec(self, from: #to_lower_case) -> Self::Spec {
                    OneToManyInverse {
                        foriegn_key: #foriegn_key.to_string(),
                        from,
                        to: #from_lower_case,
                    }
                }
            }
            impl LinkData<#from_lower_case> for Relation<#from_lower_case, #to_lower_case> {
                type Spec = OneToMany<#from_lower_case, #to_lower_case>;
                fn spec(self, from: #from_lower_case) -> Self::Spec {
                    OneToMany {
                        foriegn_key: #foriegn_key.to_string(),
                        from,
                        to: #to_lower_case,
                    }
                }
            }
        };
    }
}

pub fn main(input: Input) -> TokenStream {
    match input.ident.to_string().as_str() {
        "one_to_many" => one_to_many(match syn::parse2::<TwoIdent>(input.rest) {
            Ok(ok) => ok,
            Err(err) => {
                return err.to_compile_error();
            }
        }),
        "many_to_many" => many_to_many(match syn::parse2::<TwoIdent>(input.rest) {
            Ok(ok) => ok,
            Err(err) => {
                return err.to_compile_error();
            }
        }),
        _ => abort!(
            input.ident.span(),
            "unknown relation, only {} are supported, consider implementing Related manually",
            ["one_to_many", "many_to_many"].join(", ")
        ),
    }
}
