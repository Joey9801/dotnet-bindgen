extern crate proc_macro;
use self::proc_macro::TokenStream;

use proc_macro2::Literal;

use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn};

use dotnet_bindgen_core::*;


#[proc_macro_attribute]
pub fn dotnet_bindgen(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let func_name = input.sig.ident.to_string();

    let func_name_bytes_ident;
    let func_name_bytes;
    {
        func_name_bytes_ident = format_ident!("__{}_name_bytes", func_name);
        let func_name_len = func_name.len();

        let func_name_as_bytes = Literal::byte_string(func_name.as_bytes());
        func_name_bytes = quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            #[no_mangle]
            pub static #func_name_bytes_ident: [u8; #func_name_len] = *#func_name_as_bytes;
        };
    }

    let arg_arr_ident;
    let arg_arr;
    {
        arg_arr_ident = format_ident!("__{}_arg_arr", func_name);
        let arg_count = 1usize;

        let arg_1_type = FfiType::Int { width: 16, signed: true };
        let arg_1_name_bytes_ident = format_ident!("__{}_arg_{}_name_bytes", func_name, format!("{}", 1));
        let arg_1_name_as_bytes = Literal::byte_string("arg_1".as_bytes());
        let arg_1_name_len = "arg_1".len();
        let arg_1_name_bytes = quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            #[no_mangle]
            pub static #arg_1_name_bytes_ident: [u8; #arg_1_name_len] = *#arg_1_name_as_bytes;
        };

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

    let binddef_ident = format_ident!("__{}_bind_def", func_name);
    let binddef = quote! {
        #[link_section = #BINDGEN_SECTION_NAME]
        #[no_mangle]
        pub static #binddef_ident: BindGenFunction = BindGenFunction {
            name_bytes: &#func_name_bytes_ident,
            return_type: FfiType::Void,
            args: &#arg_arr_ident,
        };
    };

    TokenStream::from(quote! {
        #func_name_bytes
        #arg_arr
        #binddef
        #input
    })
}
