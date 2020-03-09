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
            Self::Array { elem_type } => {
                elem_type.to_tokens(tokens);
                tokens.push(ast::Group {
                    delimiter: ast::Delimiter::Bracket,
                    content: ast::TokenStream::new(),
                });
            }
            Self::Ptr { target } => {
                tokens.push(ast::Punct::Asterisk);
                target.to_tokens(tokens);
            }
            Self::Struct { name } => name.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GeneratedIdentId(i32);

#[derive(Debug, Clone)]
enum Ident {
    Named(String),
    Generated(GeneratedIdentId),
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
        }
        .into()
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
trait BodyElement: ast::ToTokens {
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

/// $return_type $return_ident = $object.$method_name($args,*)
///
/// For method calls on the current object, set MethodCall::object to
/// Ident::Named("this")
#[derive(Debug, Clone)]
struct MethodCall {
    /// Optional as C# doens't have a unit type, so it is illegal to assign the
    /// result of a method returning void
    return_ident: Option<Ident>,
    return_type: CSharpType,
    object: Ident,
    method: Ident,
    args: Vec<Ident>,
}

impl ast::ToTokens for MethodCall {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        if let Some(ident) = &self.return_ident {
            self.return_type.to_tokens(tokens);
            ident.to_tokens(tokens);
            tokens.push(ast::Punct::Equals);
        }

        self.object.to_tokens(tokens);
        tokens.push(ast::Punct::Period);
        self.method.to_tokens(tokens);

        let mut arg_group = ast::Group {
            delimiter: ast::Delimiter::Paren,
            content: ast::TokenStream::new(),
        };
        let mut first = true;
        for arg in &self.args {
            if !first {
                arg_group.content.push(ast::Punct::Comma);
            }
            first = false;
            arg.to_tokens(&mut arg_group.content);
        }

        tokens.push(arg_group);
        tokens.push(ast::Punct::Semicolon);
    }
}

impl BodyElement for MethodCall {}

#[derive(Debug, Clone)]
struct Return {
    ident: Ident,
}

impl ast::ToTokens for Return {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        tokens.push(ast::Ident::new("return"));
        self.ident.to_tokens(tokens);
        tokens.push(ast::Punct::Semicolon);
    }
}

impl BodyElement for Return {}

struct MethodArg {
    name: Ident,
    ty: CSharpType,
}

struct Method {
    name: Ident,
    return_type: CSharpType,
    args: Vec<MethodArg>,
    body: Vec<Box<dyn BodyElement>>,
}

impl ast::ToTokens for Method {
    fn to_tokens(&self, tokens: &mut ast::TokenStream) {
        self.return_type.to_tokens(tokens);
        self.name.to_tokens(tokens);

        let mut arg_tokens = ast::TokenStream::new();
        let mut first_arg = true;
        for arg in &self.args {
            if !first_arg {
                arg_tokens.push(ast::Punct::Comma);
            }
            first_arg = false;
            arg.ty.to_tokens(&mut arg_tokens);
            arg.name.to_tokens(&mut arg_tokens);
        }

        tokens.push(ast::Group {
            delimiter: ast::Delimiter::Paren,
            content: arg_tokens,
        });

        tokens.push(ast::Group {
            delimiter: ast::Delimiter::Brace,
            content: self.body.to_token_stream(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_ast::ToTokens;

    fn to_tokens_test_helper(element: impl ToTokens, expected: &str) {
        let tokens = element.to_token_stream();

        let mut rendered = Vec::new();
        tokens
            .render(&mut rendered)
            .expect("TokenStream::render to not emit an error");
        let rendered = String::from_utf8(rendered).expect("TokenStream::render to emit valid utf8");

        let rendered_tokens = rendered.split_whitespace().collect::<Vec<_>>();
        let expected_tokens = expected.split_whitespace().collect::<Vec<_>>();

        assert_eq!(rendered_tokens, expected_tokens);
    }

    #[test]
    fn test_cast_tokens() {
        to_tokens_test_helper(
            Cast {
                ident: "target".into(),
                ty: CSharpType::Int16,
                source_ident: "source".into(),
            },
            "Int16 target = ( Int16 ) source ;",
        );
    }

    #[test]
    fn test_method_call_tokens() {
        to_tokens_test_helper(
            MethodCall {
                return_ident: Some("ret".into()),
                return_type: CSharpType::Int16,
                object: "this".into(),
                method: "MethodName".into(),
                args: vec!["anArg".into(), "anotherArg".into()],
            },
            "Int16 ret = this . MethodName ( anArg , anotherArg ) ;",
        );
    }

    #[test]
    fn test_return_tokens() {
        to_tokens_test_helper(
            Return {
                ident: Ident::Generated(GeneratedIdentId(12)),
            },
            "return _gen12 ;",
        );
    }

    #[test]
    fn test_method_tokens() {
        let method = Method {
            name: "FooMethod".into(),
            return_type: CSharpType::Int64,
            args: vec![
                MethodArg {
                    name: "arg1".into(),
                    ty: CSharpType::UInt16,
                },
                MethodArg {
                    name: "barArg".into(),
                    ty: CSharpType::Bool,
                },
            ],
            body: vec![
                Box::new(MethodCall {
                    return_ident: Some(Ident::Generated(GeneratedIdentId(1))),
                    return_type: CSharpType::Int64,
                    object: "this".into(),
                    method: Ident::Generated(GeneratedIdentId(0)),
                    args: vec!["arg1".into(), "barArg".into()],
                }),
                Box::new(Return {
                    ident: Ident::Generated(GeneratedIdentId(1)),
                }),
            ],
        };

        to_tokens_test_helper(
            method,
            "Int64 FooMethod ( UInt16 arg1 , bool barArg ) 
            {
                Int64 _gen1 = this . _gen0 ( arg1 , barArg ) ;
                return _gen1 ;
            }",
        );
    }
}
