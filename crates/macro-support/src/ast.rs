use proc_macro2::{TokenStream, Literal, Ident};
use quote::{quote, format_ident, ToTokens};

use dotnet_bindgen_core::*;

struct ByteString {
    ident: Ident,
    value: Literal,
    len: usize,
}

impl ByteString {
    fn new(name: &str, value: &str) -> Self {
        let ident = format_ident!("{}", name);
        let bytes = value.as_bytes();
        let len = bytes.len();
        let value = Literal::byte_string(bytes);
        Self { ident, value, len }
    }
}

impl ToTokens for ByteString {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let value = &self.value;
        let len = self.len;

        let new_tokens = quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            static #ident: [u8; #len] = *#value;
        };

        tokens.extend(new_tokens);
    }
}

struct AstType {
    source: FfiType,
}

impl ToTokens for AstType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use dotnet_bindgen_core::FfiType::*;

        let new_tokens = match self.source {
            Int { width, signed } => quote! {
                ::dotnet_bindgen_core::FfiType::Int {
                    width: #width,
                    signed: #signed,
                }
            },
            Void => quote! {
                ::dotnet_bindgen_core::FfiType::Void
            }
        };

        tokens.extend(new_tokens);
    }
}

struct AstMethodArgument {
    source: ::dotnet_bindgen_core::MethodArgument<'static>,
    index: usize,
}

impl AstMethodArgument {
    fn name(&self) -> ByteString {
        ByteString::new(&format!("arg_{}_name", self.index), &self.source.name)
    }
}

impl ToTokens for AstMethodArgument {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name_ident = self.name().ident;
        let ffi_type = AstType { source: self.source.ffi_type.clone() };

        let new_tokens = quote! {
            ::dotnet_bindgen_core::MethodArgument {
                name: ::dotnet_bindgen_core::MaybeOwnedString {
                    bytes: ::dotnet_bindgen_core::MaybeOwnedArr::Ref(&#name_ident),
                },
                ffi_type: #ffi_type
            }
        };

        tokens.extend(new_tokens);
    }
}

#[derive(Debug)]
pub struct AstFunction {
    source: ::dotnet_bindgen_core::BindgenFunction<'static>,
}

impl AstFunction {
    fn name(&self) -> ByteString {
        ByteString::new("func_name", &self.source.name)
    }

    fn arguments<'a>(&'a self) -> impl Iterator<Item=AstMethodArgument> + 'a {
        self.source.args
            .iter()
            .enumerate()
            .map(|(i, arg)| AstMethodArgument {
                source: arg.clone(),
                index: i
            })
    }
}

impl From<BindgenFunction<'static>> for AstFunction {
    fn from(func: BindgenFunction<'static>) -> Self {
        Self { source: func }
    }
}

impl ToTokens for AstFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut strings = Vec::new();
        let mut arg_chunks = Vec::new();
        for arg in self.arguments() {
            strings.push(arg.name());
            arg_chunks.push(arg.to_token_stream());
        }

        let mut inner_tokens = TokenStream::new();
        for string in strings {
            inner_tokens.extend(string.to_token_stream());
        }

        let arg_count = self.source.args.len();
        let only_arg = arg_chunks.first().unwrap();
        inner_tokens.extend(quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            pub static method_args: [::dotnet_bindgen_core::MethodArgument; #arg_count] = [
                #only_arg
            ];
        });

        let func_name = self.name();
        let func_name_ident = self.name().ident;

        let return_type = AstType { source: self.source.return_type.clone() };

        let exposed_ident = format_ident!("__bindgen_func_{}", &self.source.name as &str);
        inner_tokens.extend(quote! {
            #func_name

            #[no_mangle]
            #[link_section = #BINDGEN_SECTION_NAME]
            pub static #exposed_ident: ::dotnet_bindgen_core::BindgenFunction = ::dotnet_bindgen_core::BindgenFunction {
                name: ::dotnet_bindgen_core::MaybeOwnedString {
                    bytes: ::dotnet_bindgen_core::MaybeOwnedArr::Ref(&#func_name_ident),
                },
                args: ::dotnet_bindgen_core::MaybeOwnedArr::Ref(&method_args),
                return_type: #return_type,
            };
        });

        let private_mod_name = format_ident!("__bindgen_func_{}_mod", &self.source.name as &str);
        let new_tokens = quote! {
            mod #private_mod_name {
                #inner_tokens
            }
        };

        tokens.extend(new_tokens);
    }
}