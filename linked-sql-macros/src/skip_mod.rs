use proc_macro2::TokenStream;
use quote::quote;

pub fn main(input: TokenStream) -> TokenStream {
    quote! {
        #[allow(unexpected_cfgs)]
        #[cfg(feature = "skip_without_comments")]
        #input
    }
}
