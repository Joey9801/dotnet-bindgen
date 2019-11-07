use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;

mod error;
pub use crate::error::Diagnostic;

use dotnet_bindgen_core::*;

struct ExportedFunctionArg {
    name: proc_macro2::Ident,
    ty: syn::Type,
}

impl std::fmt::Debug for ExportedFunctionArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty_string = format!("syn::Type({})", self.ty.to_token_stream().to_string());
        write!(
            f,
            "ExportedFunctionArg {{ name: {}, ty: {} }}",
            self.name, ty_string
        )
    }
}

struct ExportedFunction {
    name: proc_macro2::Ident,
    arguments: Vec<ExportedFunctionArg>,
    return_ty: Option<syn::Type>,
}

impl std::fmt::Debug for ExportedFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let return_ty_string = match &self.return_ty {
            Some(t) => format!("Some(syn::Type({}))", t.to_token_stream().to_string()),
            None => "None".to_string(),
        };

        write!(
            f,
            "ExportedFunction {{ name: {}, arguments: {:?}, return_ty: {:?} }}",
            self.name, self.arguments, return_ty_string
        )
    }
}

impl ToTokens for ExportedFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut thunk_args = Vec::new();
        let mut arg_conversions = Vec::new();
        let mut arg_descriptors = Vec::new();

        for arg in &self.arguments {
            let name = &arg.name;
            let ty = &arg.ty;
            thunk_args.push(
                quote! {
                    #name: <#ty as ::dotnet_bindgen::core::BindgenAbiConvert>::AbiType
                }
                .to_token_stream(),
            );

            arg_conversions.push(quote! {
                let #name = <#ty as ::dotnet_bindgen::core::BindgenAbiConvert>::from_abi_type(#name);
            });

            let name_string = name.to_string();
            arg_descriptors.push(quote! {
                ::dotnet_bindgen::core::BindgenFunctionArgumentDescriptor {
                    name: #name_string.to_string(),
                    ty: <#ty as ::dotnet_bindgen::core::BindgenTypeDescribe>::describe(),
                }
            })
        }

        let arg_names = self.arguments.iter().map(|a| a.name.clone());

        let real_name = &self.name;
        let thunk_name = format_ident!("__bindgen_thunk_{}", self.name);
        let descriptor_name = format_ident!("{}_func_{}", BINDGEN_DESCRIBE_PREFIX, self.name);
        let real_name_string = real_name.to_string();
        let thunk_name_string = thunk_name.to_string();

        let thunk = match &self.return_ty {
            Some(ty) => quote!{
                #[no_mangle]
                pub extern "C" fn #thunk_name(
                    #(#thunk_args),*
                ) -> <#ty as ::dotnet_bindgen::core::BindgenAbiConvert>::AbiType {
                    #(#arg_conversions)*
                    let ret = #real_name(#(#arg_names),*);
                    <#ty as ::dotnet_bindgen::core::BindgenAbiConvert>::to_abi_type(ret)
                }
            },
            None => quote! {
                #[no_mangle]
                pub extern "C" fn #thunk_name(#(#thunk_args),*) {
                    #(#arg_conversions)*
                    #real_name(#(#arg_names),*);
                }
            }
        };

        let return_ty_descriptor_frag = match &self.return_ty {
            Some(ty) => quote! {
                <#ty as ::dotnet_bindgen::core::BindgenTypeDescribe>::describe()
            },
            None => quote! {
                ::dotnet_bindgen::core::BindgenTypeDescriptor::Void
            }
        };

        let descriptor = quote! {
            #[no_mangle]
            pub fn #descriptor_name() -> ::dotnet_bindgen::core::BindgenExportDescriptor {
                ::dotnet_bindgen::core::BindgenExportDescriptor::Function(
                    ::dotnet_bindgen::core::BindgenFunctionDescriptor {
                        real_name: #real_name_string.to_string(),
                        thunk_name: #thunk_name_string.to_string(),
                        arguments: vec![#(#arg_descriptors),*],
                        return_ty: #return_ty_descriptor_frag,
                    }
                )
            }
        };

        (quote! {
            #thunk
            #descriptor
        }).to_tokens(tokens);
    }
}

struct ExportedStructField {
    name: proc_macro2::Ident,
    ty: syn::Type,
    span: proc_macro2::Span,
}

impl std::fmt::Debug for ExportedStructField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty_string = format!("syn::Type({})", self.ty.to_token_stream().to_string());
        write!(f, "ExportedStructField {{ name: {}, ty: {} }}", self.name, ty_string)
    }
}

struct ExportedStruct {
    name: proc_macro2::Ident,
    fields: Vec<ExportedStructField>,
    span: proc_macro2::Span,
}

impl std::fmt::Debug for ExportedStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExportedStruct {{ name: {}, fields: {:?} }}", self.name, self.fields)
    }
}

impl ExportedStruct {
    /// For each member, produces an item of the form
    ///     `struct Assert3 where String: FfiStable`
    /// to fail compilation with an appropriate error message with an appropriate span when the
    /// exported struct can not be FfiStable
    fn ffi_stable_member_assertions(&self) -> TokenStream {
        let mut assertions = Vec::new();
        for field in &self.fields {
            let assert_struct_ident = format_ident!("_AssertFfiStable_{}_{}", self.name, field.name);
            let ty = &field.ty;
            let ty_span = ty.span();
            assertions.push(quote_spanned!{ty_span=>
                #[allow(non_camel_case_types)]
                struct #assert_struct_ident where #ty: ::dotnet_bindgen::core::FfiStable {}
            })
        }

        quote!{#(
            #assertions
        )*}
    }

    /// Conditionally implements FfiStable for this struct, if all its underlying members are FfiStable.
    fn conditional_ffi_stable_impl(&self) -> TokenStream {
        let this_ty = &self.name;

        let mut ffi_stable_impl = quote_spanned!{self.span=>
            impl ::dotnet_bindgen::core::FfiStable for #this_ty
            where
        };
        for field in &self.fields {
            let ty = &field.ty;
            ffi_stable_impl = quote_spanned!{field.span=>
                #ffi_stable_impl #ty: ::dotnet_bindgen::core::FfiStable,
            }
        }

        quote_spanned!{self.span=>
            #ffi_stable_impl {}
        }
    }
}

impl ToTokens for ExportedStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let assertions = self.ffi_stable_member_assertions();
        let ffi_stable_impl = self.conditional_ffi_stable_impl();

        // TODO:
        let descriptor_func = TokenStream::new();

        (quote! {
            #assertions
            #ffi_stable_impl
            #descriptor_func
        }).to_tokens(tokens);
    }
}

#[derive(Debug)]
enum Export {
    Func(ExportedFunction),
    Struct(ExportedStruct),
}

impl ToTokens for Export {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Export::Func(f) => f.to_tokens(tokens),
            Export::Struct(s) => s.to_tokens(tokens),
        };
    }
}

struct Program {
    exports: Vec<Export>,
}

impl ToTokens for Program {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for export in &self.exports {
            export.to_tokens(tokens);
        }
    }
}

trait MacroParse {
    fn macro_parse(&self, program: &mut Program) -> Result<(), Diagnostic>;
}

pub fn expand(_attrs: TokenStream, tokens: TokenStream) -> Result<TokenStream, Diagnostic> {
    let mut program = Program {
        exports: Vec::new(),
    };

    let item = syn::parse2::<syn::Item>(tokens)?;
    item.macro_parse(&mut program)?;

    let mut tokens = proc_macro2::TokenStream::new();
    item.to_tokens(&mut tokens);
    program.to_tokens(&mut tokens);

    Ok(tokens)
}

impl MacroParse for syn::Item {
    fn macro_parse(&self, program: &mut Program) -> Result<(), Diagnostic> {
        match self {
            syn::Item::Fn(f) => f.macro_parse(program),
            syn::Item::Struct(s) => s.macro_parse(program),
            _ => Err(Diagnostic::spanned_error(
                self,
                "Can't generate binding metadata for this",
            )),
        }
    }
}

impl MacroParse for syn::ItemFn {
    fn macro_parse(&self, program: &mut Program) -> Result<(), Diagnostic> {
        let mut arguments = Vec::new();

        for arg in self.sig.inputs.iter() {
            arguments.push(match arg {
                syn::FnArg::Receiver(r) => {
                    bail_span!(r, "Can't generate binding metadata for methods")
                }
                syn::FnArg::Typed(pat_type) => {
                    let name = parse_pat(&pat_type.pat)?;
                    let ty = *pat_type.ty.clone();
                    ExportedFunctionArg { name, ty }
                }
            });
        }

        let name = self.sig.ident.clone();
        let return_ty: Option<syn::Type> = match &self.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_arrow, ty) => Some(*ty.clone()),
        };

        program.exports.push(Export::Func(ExportedFunction {
            name,
            arguments,
            return_ty,
        }));

        Ok(())
    }
}

impl MacroParse for syn::ItemStruct {
    fn macro_parse(&self, program: &mut Program) -> Result<(), Diagnostic> {
        let name = self.ident.clone();

        let fields = match &self.fields {
            syn::Fields::Named(n) => parse_named_fields(&n),
            _ => Err(Diagnostic::spanned_error(
                self,
                "Can only structs with named fields"
            ))
        }?;

        let span = self.ident.span();

        program.exports.push(Export::Struct(ExportedStruct {
            name,
            fields,
            span,
        }));

        Ok(())
    }
}

fn parse_named_fields(fields: &syn::FieldsNamed) -> Result<Vec<ExportedStructField>, Diagnostic> {
    let mut fields_parsed = Vec::new();
    for field in fields.named.iter() {
        let name = field.ident.as_ref()
            .expect("Expected syn::FieldNamed to contain fields with names")
            .clone();
        let ty = field.ty.clone();
        let span = fields.span();

        fields_parsed.push(ExportedStructField {
            name,
            ty,
            span,
        })
    }

    Ok(fields_parsed)
}

fn parse_pat(pat: &syn::Pat) -> Result<proc_macro2::Ident, Diagnostic> {
    match pat {
        syn::Pat::Ident(pat_ident) => parse_pat_ident(&pat_ident),
        _ => bail_span!(pat, "Can't generate binding metadata for this pattern"),
    }
}

fn parse_pat_ident(pat_ident: &syn::PatIdent) -> Result<proc_macro2::Ident, Diagnostic> {
    match &pat_ident.by_ref {
        Some(r) => bail_span!(r, "Can't generate binding metadata for ref types"),
        None => (),
    };

    match &pat_ident.subpat {
        Some((_at, pat)) => bail_span!(pat, "Can't generate binding metadata for subpatterns"),
        None => (),
    };

    Ok(pat_ident.ident.clone())
}
