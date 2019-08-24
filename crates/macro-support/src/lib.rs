use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;

mod error;
pub use crate::error::Diagnostic;

use dotnet_bindgen_core::*;

enum Export<'a> {
    Func(BindgenFunction<'a>),
}

struct Program<'a> {
    exports: Vec<Export<'a>>,
}

impl<'a> ToTokens for Program<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {}
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
        let func_name = self.sig.ident.to_string();
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

        dbg!(args);

        Ok(())
    }
}

fn parse_pat(pat: &syn::Pat) -> Result<MaybeOwnedString, Diagnostic> {
    match pat {
        syn::Pat::Ident(pat_ident) => parse_pat_ident(&pat_ident),
        _ => bail_span!(pat, "Can't generate binding metadata for this pattern"),
    }
}

fn parse_pat_ident(pat_ident: &syn::PatIdent) -> Result<MaybeOwnedString, Diagnostic> {
    match &pat_ident.by_ref {
        Some(r) => bail_span!(r, "Can't generate binding metadata for ref types"),
        None => (),
    };

    match &pat_ident.subpat {
        Some((at, pat)) => bail_span!(pat, "Can't generate binding metadata for subpatterns"),
        None => (),
    };

    Ok(MaybeOwnedString::from_str(&pat_ident.ident.to_string()).unwrap())
}

fn parse_type(ty: &syn::Type) -> Result<FfiType, Diagnostic> {
    let ffi_type = match ty {
        _ => {
            return Err(err_span!(
                ty,
                "Can't generate binding metadata for this type"
            ))
        }
    };

    Ok(ffi_type)
}
