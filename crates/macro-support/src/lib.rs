use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};

mod error;
pub use crate::error::Diagnostic;

use dotnet_bindgen_core::*;

#[derive(Debug)]
enum Export {
    Func(BindgenFunction),
}

impl Export {
    fn name(&self) -> String {
        match self {
            Export::Func(f) => format!("func_{}", f.name),
        }
    }
}

impl ToTokens for Export {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let json_string = match self {
            Export::Func(f) => serde_json::to_string(&f).unwrap(),
        };

        let exposed_ident = format_ident!("__bindgen_{}", self.name());
        let bytes = json_string.as_bytes();
        let len = bytes.len();
        let data = Literal::byte_string(bytes);

        tokens.extend(quote! {
            #[link_section = #BINDGEN_DATA_SECTION_NAME]
            #[no_mangle]
            pub static #exposed_ident: [u8; #len] = *#data;
        });
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
            _ => Err(Diagnostic::spanned_error(
                self,
                "Can't generate binding metadata for this",
            )),
        }
    }
}

impl MacroParse for syn::ItemFn {
    fn macro_parse(&self, program: &mut Program) -> Result<(), Diagnostic> {
        let mut args = Vec::new();

        for arg in self.sig.inputs.iter() {
            args.push(match arg {
                syn::FnArg::Receiver(r) => {
                    bail_span!(r, "Can't generate binding metadata for methods")
                }
                syn::FnArg::Typed(pat_type) => {
                    let name = parse_pat(&pat_type.pat)?;
                    let ty = parse_type(&pat_type.ty)?;
                    MethodArgument { ty, name }
                }
            });
        }

        let args = MaybeOwnedArr::Owned(args);
        let return_ty = match &self.sig.output {
            syn::ReturnType::Default => BoundType::FfiType(FfiType::Void),
            syn::ReturnType::Type(_arrow, ty) => parse_type(&ty)?,
        };
        let name = self.sig.ident.to_string();

        let func = BindgenFunction {
            name,
            args,
            return_ty,
        };

        program.exports.push(Export::Func(func));

        Ok(())
    }
}

fn parse_pat(pat: &syn::Pat) -> Result<String, Diagnostic> {
    match pat {
        syn::Pat::Ident(pat_ident) => parse_pat_ident(&pat_ident),
        _ => bail_span!(pat, "Can't generate binding metadata for this pattern"),
    }
}

fn parse_pat_ident(pat_ident: &syn::PatIdent) -> Result<String, Diagnostic> {
    match &pat_ident.by_ref {
        Some(r) => bail_span!(r, "Can't generate binding metadata for ref types"),
        None => (),
    };

    match &pat_ident.subpat {
        Some((_at, pat)) => bail_span!(pat, "Can't generate binding metadata for subpatterns"),
        None => (),
    };

    Ok(pat_ident.ident.to_string())
}

fn parse_type(ty: &syn::Type) -> Result<BoundType, Diagnostic> {
    let err = Err(err_span!(
        ty,
        "Can't generate binding metadata for this type"
    ));

    let ffi_type = match ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return err;
            }

            let path = &type_path.path;
            if path.leading_colon.is_some() {
                return err;
            }

            let seg_count = path.segments.len();
            if seg_count > 1 || seg_count == 0 {
                return err;
            }

            let only_segment = path.segments.first().unwrap();
            if !only_segment.arguments.is_empty() {
                return err;
            }

            let ident = only_segment.ident.to_string();
            match ident.parse() {
                Ok(ffi_type) => BoundType::FfiType(ffi_type),
                Err(_) => return err,
            }
        }
        _ => {
            return err;
        }
    };

    Ok(ffi_type)
}
