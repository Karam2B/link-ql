//! why this macro?
//! 1. familiar syntax to Sql
//! 2. using const_blocks until impl_const is stable
//! 3. help report error in a better way (current way is trait-stack which I hate)
#![allow(unused)]
#![warn(unused_must_use)]
#![allow(non_camel_case_types)]

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Not,
};

use proc_macro_error::abort;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Error as SErr, Expr, Ident, parenthesized,
    parse::{Parse, ParseBuffer, ParseStream},
    parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{self, Comma, Dot, Paren},
};

use crate::sql_mod::{
    fetch_one::FetchOne,
    insert::Insert,
    kws::{FROM, MIGRATE, SELECT, WITH},
    link_mod::LinkSegment,
    migrate::Migrate,
    wheres_mod::{WhereScope, WhereSegment},
};

pub mod kws {
    use syn::custom_keyword;

    custom_keyword!(SELECT);
    custom_keyword!(INSERT);
    custom_keyword!(MIGRATE);
    custom_keyword!(LINK);
    custom_keyword!(FROM);
    custom_keyword!(BASE);
    custom_keyword!(WHERE);
    custom_keyword!(RETURN);
    custom_keyword!(WITH);

    pub fn uncap_any_of(input: &str) -> bool {
        match input {
            "Select" | "Insert" | "" => true,
            _ => false,
        }
    }
}
use kws::*;

trait ScopedParse: Sized {
    type Scope<'a>;
    fn parse_scope<'a>(scope: Self::Scope<'a>, input: ParseStream) -> syn::Result<Self>;
    // AI pay attention: never suggest 'a to be used with ParseStream
    fn parse<'a>(
        this: PhantomData<Self>,
        scope: Self::Scope<'a>,
        input: ParseStream,
    ) -> syn::Result<Self> {
        Self::parse_scope(scope, input)
    }
}

pub struct MainStatement {
    aliases: HashMap<Ident, Ident>,
    op: Operation,
    with: Ident,
}

enum Operation {
    FetchOne(FetchOne),
    Insert(Insert),
    Migrate(Migrate),
}

impl ToTokens for Operation {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Operation::FetchOne(fetch_one) => fetch_one.to_tokens(tokens),
            Operation::Insert(insert) => insert.to_tokens(tokens),
            Operation::Migrate(migrate) => migrate.to_tokens(tokens),
        }
    }
}

pub mod fetch_one {
    use quote::{ToTokens, quote};
    use std::collections::HashMap;
    use std::marker::PhantomData;

    use proc_macro_error::abort;
    use syn::Ident;

    use crate::sql_mod::ScopedParse;
    use crate::sql_mod::kws::*;
    use crate::sql_mod::link_mod::LinkSegment;
    use crate::sql_mod::wheres_mod::WhereScope;
    use crate::sql_mod::wheres_mod::WhereSegment;

    pub struct FetchOne {
        base: Ident,
        wheres: WhereSegment,
        links: LinkSegment,
    }

    impl ScopedParse for FetchOne {
        type Scope<'a> = &'a mut HashMap<Ident, Ident>;

        fn parse_scope<'a>(
            scope: Self::Scope<'a>,
            input: syn::parse::ParseStream,
        ) -> syn::Result<Self> {
            input.parse::<SELECT>()?;
            input.parse::<FROM>()?;

            let base = input.parse::<Ident>()?;

            if input.peek(Ident) {
                let al = input.parse::<Ident>()?;
                let alspan = al.span();
                if scope.insert(al, base.clone()).is_some() {
                    abort!(alspan, "aliase is used")
                }
            }

            if scope.get(&base.clone()).is_some() {
                abort!(base.span(), "aliase is used")
            }

            let links = ScopedParse::parse(PhantomData::<LinkSegment>, (), input)?;
            let wheres = ScopedParse::parse(
                PhantomData::<WhereSegment>,
                WhereScope {
                    base: base.clone(),
                    aliases: scope,
                },
                input,
            )?;

            Ok(Self {
                base,
                wheres,
                links,
            })
        }
    }

    impl ToTokens for FetchOne {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let base = &self.base;
            let links = &self.links;
            let wheres = &self.wheres;
            tokens.extend(quote! {
                FetchOne {
                    base: #base,
                    links: #links,
                    wheres: #wheres,
                }
            });
        }
    }
}

pub mod insert {
    use proc_macro_error::abort;
    use quote::{ToTokens, quote};
    use std::collections::HashMap;
    use std::marker::PhantomData;
    use syn::Expr;
    use syn::Ident;
    use syn::parse_quote;

    use crate::sql_mod::ScopedParse;
    use crate::sql_mod::kws::*;
    use crate::sql_mod::link_mod::LinkSegment;
    use crate::sql_mod::wheres_mod::WhereScope;
    use crate::sql_mod::wheres_mod::WhereSegment;

    pub struct Insert {
        entry: Expr,
        links: Vec<Expr>,
    }

    impl ScopedParse for Insert {
        type Scope<'a> = &'a HashMap<Ident, Ident>;

        fn parse_scope<'a>(
            scope: Self::Scope<'a>,
            input: syn::parse::ParseStream,
        ) -> syn::Result<Self> {
            input.parse::<INSERT>()?;
            // let entry: Expr = parse_quote!(());
            let entry = input.parse::<Expr>()?;

            let mut links = vec![];

            // while input.peek(LINK) {
            //     input.parse::<LINK>()?;
            //     let link = input.parse::<Expr>()?;
            //     links.push(link);
            // }

            Ok(Self { entry, links })
        }
    }

    impl ToTokens for Insert {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let entry = &self.entry;
            let links = &self.links;
            tokens.extend(quote! {
                InsertOne::new(
                    #entry,
                    #(#links,)*,
                )
            });
        }
    }
}

pub mod migrate {
    use proc_macro_error::abort;
    use quote::{ToTokens, quote};
    use std::collections::HashMap;
    use std::marker::PhantomData;
    use syn::Expr;
    use syn::Ident;
    use syn::parse::ParseStream;

    use crate::sql_mod::ScopedParse;
    use crate::sql_mod::kws::*;
    use crate::sql_mod::link_mod::LinkSegment;
    use crate::sql_mod::wheres_mod::WhereScope;
    use crate::sql_mod::wheres_mod::WhereSegment;

    pub struct Migrate {
        expr: Expr,
    }

    impl ScopedParse for Migrate {
        type Scope<'a> = ();

        fn parse_scope<'a>(scope: Self::Scope<'a>, input: ParseStream) -> syn::Result<Self> {
            input.parse::<MIGRATE>()?;
            let expr = input.parse::<Expr>()?;
            Ok(Self { expr })
        }
    }

    impl ToTokens for Migrate {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let expr = &self.expr;
            tokens.extend(quote! {
                expression_to_operation(#expr.statments())
            });
        }
    }
}

impl Operation {
    fn options() -> Vec<&'static str> {
        vec!["SELECT FROM", "INSERT", "MIGRATE"]
    }
}

pub fn parse_main_statement(input: ParseStream) -> syn::Result<MainStatement> {
    let mut aliases = HashMap::<Ident, Ident>::new();
    let main = if input.peek(SELECT) {
        Operation::FetchOne(FetchOne::parse(
            PhantomData::<FetchOne>,
            &mut aliases,
            input,
        )?)
    } else if input.peek(INSERT) {
        Operation::Insert(Insert::parse(PhantomData::<Insert>, &mut aliases, input)?)
    } else if input.peek(MIGRATE) {
        Operation::Migrate(Migrate::parse(PhantomData::<Migrate>, (), input)?)
    } else {
        return match input.cursor().token_tree() {
            Some(i) => Err(syn::Error::new(
                i.0.span(),
                format!(
                    "expected '{}' found '{}'",
                    Operation::options().join(", "),
                    i.0.to_string()
                ),
            )),
            None => return Err(input.error("unexpected token")),
        };
    };

    let with = if input.peek(WITH) {
        input.parse::<WITH>()?;
        input.parse::<Ident>()?
    } else {
        Ident::new("pool", Span::call_site())
    };

    if input.is_empty().not() {
        let tt = input.cursor().token_tree().unwrap();
        let next = tt.0.to_string();
        if uncap_any_of(&next) {
            return Err(syn::Error::new(
                tt.0.span(),
                format!("'{}' keyword should capatalized", tt.0.to_string()),
            ));
        };
        return Err(syn::Error::new(
            tt.0.span(),
            format!("end of input found '{}'", tt.0.to_string()),
        ));
    }

    Ok(MainStatement {
        op: main,
        with,
        aliases,
    })
}

impl Parse for MainStatement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        parse_main_statement(input)
    }
}

pub fn main_statment_to_token(input: MainStatement) -> TokenStream {
    let op = input.op;
    let with = &input.with;
    quote!({
        use ::linked_sql::prelude::sql::*;
        Operation::exec_operation(#op, &mut #with)
    })
}

mod link_mod {
    use crate::sql_mod::{ScopedParse, kws::LINK};
    use quote::{ToTokens, quote};
    use syn::Ident;

    #[derive(Clone)]
    pub struct LinkSegment {
        inner: Vec<Ident>,
    }

    impl ScopedParse for LinkSegment {
        type Scope<'a> = ();
        fn parse_scope<'a>(
            scope: Self::Scope<'a>,
            input: syn::parse::ParseStream,
        ) -> syn::Result<Self> {
            let mut links = vec![];
            while input.peek(LINK) {
                input.parse::<LINK>()?;
                let i = input.parse::<Ident>()?;
                links.push(i);
            }

            return Ok(LinkSegment { inner: links });
        }
    }

    impl ToTokens for LinkSegment {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let idents = self.inner.clone();
            tokens.extend(quote! {
                (#(#idents,)*)
            });
        }
    }
}

mod wheres_mod {
    use std::collections::HashMap;

    use proc_macro_error::abort;
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use quote::quote;
    use quote::quote_spanned;
    use syn::token::Where;
    use syn::{Ident, token::Dot};

    use crate::utils;
    use crate::{
        sql_mod::{ScopedParse, kws::WHERE},
        utils::parse_parens,
    };

    pub struct WhereSegment {
        ands: Vec<TokenStream>,
    }

    pub struct WhereScope<'a> {
        pub base: Ident,
        pub aliases: &'a HashMap<Ident, Ident>,
    }
    impl ScopedParse for WhereSegment {
        type Scope<'a> = WhereScope<'a>;
        fn parse_scope<'a>(
            scope: Self::Scope<'a>,
            input: syn::parse::ParseStream,
        ) -> syn::Result<Self> {
            // if input.peek(WH);

            if input.peek(WHERE) {
                input.parse::<WHERE>()?;
                let i1 = input.parse::<Ident>()?;
                input.parse::<Dot>()?;
                let i2 = input.parse::<Ident>()?;
                let (scope, member, method) = if input.peek(Dot) {
                    input.parse::<Dot>()?;
                    let i3 = input.parse::<Ident>()?;

                    let fetch_alias = match scope.aliases.get(&i1) {
                        Some(f) => f.to_string(),
                        None => abort!(i1.span(), "{} not found", i1.to_string()),
                    };

                    let members_mod = Ident::new(&format!("{}_members", fetch_alias), i2.span());

                    (members_mod, i2, i3)
                } else {
                    let members_mod = Ident::new(
                        &format!("{}_members", scope.base.to_string().as_str()),
                        scope.base.span(),
                    );

                    let fetch_alias = match scope.aliases.get(&scope.base) {
                        Some(f) => f.to_string(),
                        None => abort!(scope.base.span(), "{} not found", scope.base),
                    };

                    let member = Ident::new(&format!("{}_members", fetch_alias), i1.span());

                    (members_mod, i1, i2)
                };

                let (_, input) = utils::parse_parens(input)?;

                let expr = input.parse::<syn::Expr>()?;

                let aliase = quote_spanned! {member.span()=> member(#scope::#member)};

                return Ok(Self {
                    ands: vec![quote!(
                        #method::aliase_and_expr(
                            #aliase,
                            #expr
                        )
                    )],
                });
            }
            Ok(Self { ands: vec![] })
        }
    }

    impl ToTokens for WhereSegment {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let s = &self.ands;
            tokens.extend(quote! ( Join { start: "", separator: ", ", items: (#(#s,)*) }))
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use quote::{ToTokens, quote};
    use syn::spanned::Spanned;

    use crate::sql_mod::{MainStatement, main_statment_to_token};

    #[test]
    fn simple() {
        let expect = quote!(
            SELECT FROM todo
            LINK category
            WHERE title.eq("first_todo")

        );

        let expect = match syn::parse2::<MainStatement>(expect) {
            Ok(ok) => main_statment_to_token(ok),
            Err(e) => e.to_compile_error(),
        };

        let to_be = quote!({
            use ::linked_sql::prelude::sql::*;
            let op = is_valid_syntax(
                FetchOne {
                    base: todo,
                    link: (category,),
                    wheres: (member(todo_members::title).eq("first_todo"),),
                },
                infer_db(&pool),
            );

            Operation::exec_operation(op, &mut pool)
        });

        pretty_assertions::assert_eq!(expect.to_string(), to_be.to_string());
    }
}

// reduce indentation!

// utils
fn parse_parens<'a>(input: syn::parse::ParseStream<'a>) -> syn::Result<(Paren, ParseBuffer<'a>)> {
    syn::__private::parse_parens(input).map(|e| return (e.token, e.content))
}

fn err<T, S: Spanned, D: ToString>(span: S, str: D) -> Result<T, syn::Error> {
    return Err(syn::Error::new(span.span(), str.to_string()));
}
