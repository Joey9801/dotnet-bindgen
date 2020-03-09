use crate::new_ast as ast;

#[derive(Clone, Debug)]
enum CSharpType {
    /// Pseudotype - cannot appear in all typename positions
    Var,

    Void,

    /// SByte should be called Int8, but isn't for some reason
    SByte,
    Int16,
    Int32,
    Int64,

    /// Byte should be called UInt8, but isn't for some reason
    Byte,
    UInt16,
    UInt32,
    UInt64,

    Bool,

    Array {
        elem_type: Box<CSharpType>,
    },

    Ptr {
        target: Box<CSharpType>,
    },

    Struct {
        name: Ident,
    },
}

impl ast::ToTokens for CSharpType {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        match self {
            Self::Var => tokens.push(ast::Ident::new("var")),
            Self::Void => tokens.push(ast::Ident::new("void")),
            Self::SByte => tokens.push(ast::Ident::new("SByte")),
            Self::Int16 => tokens.push(ast::Ident::new("Int16")),
            Self::Int32 => tokens.push(ast::Ident::new("Int32")),
            Self::Int64 => tokens.push(ast::Ident::new("Int64")),
            Self::Byte => tokens.push(ast::Ident::new("Byte")),
            Self::UInt16 => tokens.push(ast::Ident::new("UInt16")),
            Self::UInt32 => tokens.push(ast::Ident::new("UInt32")),
            Self::UInt64 => tokens.push(ast::Ident::new("UInt64")),
            Self::Bool => tokens.push(ast::Ident::new("bool")),
            Self::Array { elem_type, } => {
                elem_type.to_tokens(tokens);
                tokens.push(ast::Group {
                    delimiter: ast::Delimiter::Bracket,
                    content: ast::TokenStream::new(),
                });
            },
            Self::Ptr { target } => {
                tokens.push(ast::Punct::Asterisk);
                target.to_tokens(tokens);
            },
            Self::Struct { name } => name.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GeneratedIdentId(i32);

#[derive(Debug, Clone)]
enum Ident {
    Named(String),
    Generated(GeneratedIdentId)
}

impl<T: ToString> From<T> for Ident {
    fn from(s: T) -> Self {
        Self::Named(s.to_string())
    }
}

impl Into<ast::Ident> for Ident {
    fn into(self) -> ast::Ident {
        match self {
            Self::Named(name) => name.to_string(),
            Self::Generated(GeneratedIdentId(num)) => format!("_gen{}", num),
        }.into()
    }
}

impl ast::ToTokens for Ident {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        let ident = match self {
            Self::Named(name) => name.to_string(),
            Self::Generated(GeneratedIdentId(num)) => format!("_gen{}", num),
        };

        tokens.push(ast::Ident::new(ident));
    }
}

/// A higher level primitive than an AST node.
trait BodyElement : ast::ToTokens {
    /// If true, renders all following elements inside a new scope.
    fn requires_block(&self) -> bool {
        false
    }
}

impl ast::ToTokens for [Box<dyn BodyElement>] {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        let mut last_element = 0;
        for element in self {
            element.to_tokens(tokens);
            last_element += 1;
            if element.requires_block() {
                break;
            }
        }

        if last_element < self.len() {
            // Create a new block for the remainder of this set of elements
            tokens.push(ast::Group {
                delimiter: ast::Delimiter::Brace,
                content: self[last_element..].to_token_stream(),
            });
        }
    }
}

/// $ty $ident = ($ty) $source_ident;
#[derive(Debug, Clone)]
struct Cast {
    ident: Ident,
    ty: CSharpType,
    source_ident: Ident,
}

impl ast::ToTokens for Cast {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        self.ty.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        tokens.push(ast::Punct::Equals);
        tokens.push(ast::Group {
            delimiter: ast::Delimiter::Paren,
            content: self.ty.to_token_stream(),
        });
        self.source_ident.to_tokens(tokens);
        tokens.push(ast::Punct::Semicolon);
    }
}

impl BodyElement for Cast {}