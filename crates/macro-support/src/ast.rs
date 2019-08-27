use proc_macro2::{TokenStream, Literal, Ident, Punct, Spacing, Group, Delimiter};
use quote::{quote, format_ident, ToTokens, TokenStreamExt};

use dotnet_bindgen_core::*;

struct AstArray<T> {
    ident: Ident,
    values: Vec<T>,
}

// TODO: Could this actually be generic? clearly the following isn't the way to
// go if many/any other array types are needed.
impl AstArray<AstMethodArgument> {
    fn new(name: &str, values: Vec<AstMethodArgument>) -> Self {
        let ident = format_ident!("{}", name);
        AstArray { ident, values }
    }

    fn storage_tokens(&self) -> TokenStream {
        let ident = self.ident.clone();
        let len = self.values.len();

        let mut tokens = quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            static #ident: [::dotnet_bindgen_core::MethodArgument; #len] =
        };

        let mut values_tokens = TokenStream::new();
        for element in &self.values {
            values_tokens.extend(element.to_token_stream());
            values_tokens.append(Punct::new(',', Spacing::Alone));
        }
        tokens.append(Group::new(Delimiter::Bracket, values_tokens));
        tokens.append(Punct::new(';', Spacing::Alone));

        for value in &self.values {
            tokens.extend(value.storage_tokens());
        }

        tokens
    }

    fn reference_tokens(&self) -> TokenStream {
        let ident = self.ident.clone();
        quote! {
            ::dotnet_bindgen_core::MaybeOwnedArr::Ref(
                &#ident,
            )
        }
    }
}

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

    fn storage_tokens(&self) -> TokenStream {
        let ident = self.ident.clone();
        let value = self.value.clone();
        let len = self.len;

        quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            static #ident: [u8; #len] = *#value;
        }
    }

    fn reference_tokens(&self) -> TokenStream {
        let ident = self.ident.clone();
        quote! {
            ::dotnet_bindgen_core::MaybeOwnedString {
                bytes: ::dotnet_bindgen_core::MaybeOwnedArr::Ref(&#ident),
            }
        }
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

    fn storage_tokens(&self) -> TokenStream {
        self.name().storage_tokens()
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

    fn arguments(&self) -> AstArray<AstMethodArgument> {
        let ast_arguments = self.source.args
            .iter()
            .enumerate()
            .map(|(i, arg)| AstMethodArgument {
                source: arg.clone(),
                index: i
            })
            .collect();

        AstArray::new("arguments", ast_arguments)
    }

    fn storage_tokens(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        tokens.extend(self.name().storage_tokens());
        tokens.extend(self.arguments().storage_tokens());
        tokens
    }
}

impl From<BindgenFunction<'static>> for AstFunction {
    fn from(func: BindgenFunction<'static>) -> Self {
        Self { source: func }
    }
}

impl ToTokens for AstFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut inner_tokens = TokenStream::new();

        inner_tokens.extend(self.storage_tokens());

        let func_name_reference = self.name().reference_tokens();
        let arguments_reference = self.arguments().reference_tokens();
        let return_type = AstType { source: self.source.return_type.clone() };
        let exposed_ident = format_ident!("__bindgen_func_{}", &self.source.name as &str);
        inner_tokens.extend(quote! {
            #[no_mangle]
            #[link_section = #BINDGEN_SECTION_NAME]
            pub static #exposed_ident: ::dotnet_bindgen_core::BindgenFunction = ::dotnet_bindgen_core::BindgenFunction {
                name: #func_name_reference,
                args: #arguments_reference,
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