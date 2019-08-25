use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;

mod error;
pub use crate::error::Diagnostic;

mod ast;

use dotnet_bindgen_core::*;

#[derive(Debug)]
enum Export {
    Func(ast::AstFunction),
}

impl ToTokens for Export {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Export::Func(f) => f.to_tokens(tokens),
        }
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
                    let ffi_type = parse_type(&pat_type.ty)?;
                    MethodArgument { ffi_type, name }
                }
            });
        }

        let args = MaybeOwnedArr::Owned(args);
        let return_type = match &self.sig.output {
            syn::ReturnType::Default => FfiType::Void,
            syn::ReturnType::Type(_arrow, ty) => parse_type(&ty)?,
        };
        let name = MaybeOwnedString::from_str(&self.sig.ident.to_string()).unwrap();

        let func = BindgenFunction {
            name,
            args,
            return_type,
        };

        program.exports.push(Export::Func(func.into()));

        Ok(())
    }
}

fn parse_pat(pat: &syn::Pat) -> Result<MaybeOwnedString<'static>, Diagnostic> {
    match pat {
        syn::Pat::Ident(pat_ident) => parse_pat_ident(&pat_ident),
        _ => bail_span!(pat, "Can't generate binding metadata for this pattern"),
    }
}

fn parse_pat_ident(pat_ident: &syn::PatIdent) -> Result<MaybeOwnedString<'static>, Diagnostic> {
    match &pat_ident.by_ref {
        Some(r) => bail_span!(r, "Can't generate binding metadata for ref types"),
        None => (),
    };

    match &pat_ident.subpat {
        Some((_at, pat)) => bail_span!(pat, "Can't generate binding metadata for subpatterns"),
        None => (),
    };

    Ok(MaybeOwnedString::from_str(&pat_ident.ident.to_string()).unwrap())
}

fn parse_type(ty: &syn::Type) -> Result<FfiType, Diagnostic> {
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
                Ok(ty) => ty,
                Err(_) => return err,
            }
        }
        _ => {
            return err;
        }
    };

    Ok(ffi_type)
}
