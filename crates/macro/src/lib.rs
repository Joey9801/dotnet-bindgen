/*
extern crate proc_macro;
use self::proc_macro::TokenStream as StdTokenStream;

use proc_macro2::{Ident, Literal, Group, TokenStream, Span, Punct, Spacing};

use quote::{quote, format_ident, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, ItemFn};

use dotnet_bindgen_core::*;

struct GlobalByteString {
    ident_prefix: String,
    name: String,
    value: String,
}

impl GlobalByteString {
    fn new(ident_prefix: String, name: String, value: String) -> Self {
        // TODO: Assert that all just ascii values
        Self { ident_prefix, name, value }
    }

    fn ident(&self) -> Ident {
        format_ident!("{}_{}_byte_string", self.ident_prefix, self.name)
    }
}

impl ToTokens for GlobalByteString {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = self.ident();
        let bytes = Literal::byte_string(self.value.as_bytes());
        let len = self.value.as_bytes().len();

        tokens.extend(quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            #[no_mangle]
            pub static #ident: [u8; #len] = *#bytes;
        });
    }
}


// struct MethodArgument {
//     ident_prefix: String,
//     name: String,
//     pos: u8,
//     ffi_type: FfiType,
// }
//
// impl MethodArgument {
//     fn new(ident_prefix: String, name: String, pos: u8, ffi_type: FfiType) -> Self {
//         Self { ident_prefix, name, pos, ffi_type }
//     }
//
//     fn name_bytes(&self) -> GlobalByteString {
//         GlobalByteString::new(
//             self.ident_prefix.to_string(),
//             format!("arg_{}_name", self.pos),
//             self.name.to_string()
//         )
//     }
// }

#[proc_macro_attribute]
pub fn dotnet_bindgen(_args: StdTokenStream, input: StdTokenStream) -> StdTokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let prefix = format!("__{}_{}", BINDGEN_GLOBAL_NAME_PREFIX, input.sig.ident);

    let func_name_bytes = GlobalByteString::new(
        prefix.to_string(),
        "func_name".to_string(),
        input.sig.ident.to_string()
    );

    // let mut error = TokenStream::new();
    // error.append(Ident::new("compile_error", input.sig.ident.span()));
    // error.append(Punct::new('!', Spacing::Alone));
    // let mut message = TokenStream::new();
    // message.append(Literal::string("This is a test error"));
    // let mut group = proc_macro2::Group::new(proc_macro2::Delimiter::Brace, message);
    // group.set_span(input.sig.ident.span());
    // error.append(group);
    // return error.into();

    let arg_arr_ident;
    let arg_arr;
    {
        arg_arr_ident = format_ident!("{}_arg_arr", prefix);
        let arg_count = input.sig.inputs.len();

        for arg in input.sig.inputs.iter() {
            let _arg = match arg {
                syn::FnArg::Receiver(_) => println!("Cannot generate bindings for methods yet."),
                _ => continue,
            };
        }

        let arg_1_type = FfiType::Int { width: 16, signed: true };

        let arg_1_name_bytes = GlobalByteString::new(
            prefix.to_string(),
            "arg_1_name".to_string(),
            "arg_1".to_string()
        );

        let arg_1_name_bytes_ident = arg_1_name_bytes.ident();
        let arg_1 = quote! {
            MethodArgument {
                name_bytes: &#arg_1_name_bytes_ident,
                ffi_type: #arg_1_type
            }
        };

        arg_arr = quote! {
            #arg_1_name_bytes

            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            #[no_mangle]
            pub static #arg_arr_ident: [MethodArgument; #arg_count] = [
                #arg_1
            ];
        };
    }

    let binddef_ident = format_ident!("{}_bind_def", prefix);
    let func_name_bytes_ident = func_name_bytes.ident();
    let binddef = quote! {
        #[link_section = #BINDGEN_SECTION_NAME]
        #[no_mangle]
        pub static #binddef_ident: BindGenFunction = BindGenFunction {
            name_bytes: &#func_name_bytes_ident,
            return_type: FfiType::Void,
            args: &#arg_arr_ident,
        };
    };

    (quote! {
        #func_name_bytes
        #arg_arr
        #binddef
    }).into()
}
*/

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
