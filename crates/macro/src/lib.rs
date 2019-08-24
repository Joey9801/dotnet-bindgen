extern crate proc_macro;
use self::proc_macro::TokenStream;

use quote::quote;

#[proc_macro_attribute]
pub fn dotnet_bindgen(attr: TokenStream, input: TokenStream) -> TokenStream {
    match dotnet_bindgen_macro_support::expand(attr.into(), input.into()) {
        Ok(tokens) => tokens.into(),
        Err(diag) => (quote! { #diag }).into(),
    }
}
