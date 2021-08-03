//! This module defines an intermediate representation that describes literal blocks of C# source
//! code free of any bindgen specific semantics. It also provides implementations of level_0's
//! ToTokens trait to lower this representation.

use super::level_0::{self as lower, Punct};

pub type LayerEntrypoint = CsSource;

#[derive(Clone, Debug)]
pub enum CSharpType {
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

impl CSharpType {
    pub fn new_ptr(other: &CSharpType) -> CSharpType {
        CSharpType::Ptr {
            target: Box::new(other.clone()),
        }
    }

    pub fn new_struct(name: impl Into<Ident>) -> CSharpType {
        CSharpType::Struct { name: name.into() }
    }

    pub fn array_of(self) -> CSharpType {
        CSharpType::Array {
            elem_type: Box::new(self),
        }
    }

    pub fn ptr_to(self) -> CSharpType {
        CSharpType::Ptr {
            target: Box::new(self),
        }
    }
}

impl lower::ToTokens for CSharpType {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        match self {
            Self::Var => tokens.push("var"),
            Self::Void => tokens.push("void"),
            Self::SByte => tokens.push("SByte"),
            Self::Int16 => tokens.push("Int16"),
            Self::Int32 => tokens.push("Int32"),
            Self::Int64 => tokens.push("Int64"),
            Self::Byte => tokens.push("Byte"),
            Self::UInt16 => tokens.push("UInt16"),
            Self::UInt32 => tokens.push("UInt32"),
            Self::UInt64 => tokens.push("UInt64"),
            Self::Bool => tokens.push("bool"),
            Self::Array { elem_type } => {
                elem_type.to_tokens(tokens);
                tokens.push(lower::Group {
                    delimiter: lower::Delimiter::Bracket,
                    content: lower::TokenStream::new(),
                });
            }
            Self::Ptr { target } => {
                target.to_tokens(tokens);
                tokens.push(lower::Punct::Asterisk);
            }
            Self::Struct { name } => name.to_tokens(tokens),
        }
    }
}

/// A node that can exist at the top level of a C# source file
pub trait TopLevelElement: lower::ToTokens {}

/// A node that has some value in the context of a method body
///
/// No distinction is made between lvalues and rvalues. Technically this means that this
/// representation can describe invalid C# programs, but there are already large classes of invalid
/// programs that can't be prevented by the type system in this representation, eg referencing a
/// variable before it is defined.
pub trait BodyExpression: lower::ToTokens {}

/// A node that can stand alone as part of a method body
pub trait BodyStatement: lower::ToTokens {
    /// If true, all following elements of the method body will be inside a new set of braces.
    /// Eg. a `fixed` statement
    fn requires_block(&self) -> bool {
        false
    }
}

impl lower::ToTokens for [Box<dyn BodyStatement>] {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
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
            tokens.push(lower::Group {
                delimiter: lower::Delimiter::Brace,
                content: self[last_element..].to_token_stream(),
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GeneratedIdentId(i32);

#[derive(Debug, Clone)]
pub enum Ident {
    Named(String),
    Generated(GeneratedIdentId),
}

impl Ident {
    pub fn new(s: impl Into<String>) -> Self {
        Self::Named(s.into())
    }

    #[cfg(test)]
    pub fn new_generated(num: i32) -> Self {
        Self::Generated(GeneratedIdentId(num))
    }
}

impl<T: ToString> From<T> for Ident {
    fn from(s: T) -> Self {
        Self::Named(s.to_string())
    }
}

impl Into<lower::Ident> for Ident {
    fn into(self) -> lower::Ident {
        match self {
            Self::Named(name) => name.to_string(),
            Self::Generated(GeneratedIdentId(num)) => format!("_gen{}", num),
        }
        .into()
    }
}

impl lower::ToTokens for Ident {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        let ident = match self {
            Self::Named(name) => name.to_string(),
            Self::Generated(GeneratedIdentId(num)) => format!("_gen{}", num),
        };

        tokens.push(ident);
    }
}

pub struct IdentGenerator {
    counter: i32,
}

impl IdentGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn generate_ident(&mut self) -> Ident {
        let num = self.counter;
        self.counter += 1;
        Ident::Generated(GeneratedIdentId(num))
    }
}

impl BodyExpression for Ident {}

pub struct UsingStatement {
    pub path: String,
}

impl lower::ToTokens for UsingStatement {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push("using");
        tokens.push(&self.path);
        tokens.push(lower::Punct::Semicolon);
    }
}

impl TopLevelElement for UsingStatement {}

pub struct Namespace {
    pub path: Ident,
    pub contents: Vec<Box<dyn TopLevelElement>>,
}

impl lower::ToTokens for Namespace {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        let mut group_stream = lower::TokenStream::new();
        for content_chunk in &self.contents {
            content_chunk.to_tokens(&mut group_stream);
        }

        tokens.push("namespace");
        tokens.push(self.path.clone());
        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Brace,
            content: group_stream,
        });
    }
}

impl TopLevelElement for Namespace {}

#[derive(Clone)]
pub enum Literal {
    Integer(i32),
    String(String),
}

impl lower::ToTokens for Literal {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        match self {
            Literal::Integer(i) => tokens.push(format!("{}", i)),
            Literal::String(s) => tokens.push(format!("\"{}\"", s)),
        }
    }
}

impl BodyExpression for Literal {}

/// $ty $ident
pub struct DeclareVariableExpr {
    pub ty: CSharpType,
    pub ident: Ident,
}

impl DeclareVariableExpr {
    pub fn stmt(self) -> DeclareVariableStmt {
        DeclareVariableStmt {
            ty: self.ty,
            ident: self.ident,
        }
    }
}

impl lower::ToTokens for DeclareVariableExpr {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.ty.to_tokens(tokens);
        self.ident.to_tokens(tokens);
    }
}

impl BodyExpression for DeclareVariableExpr {}

pub trait DoDeclare {
    fn declare_with_ty(self, ty: CSharpType) -> DeclareVariableExpr;
}

impl DoDeclare for Ident {
    fn declare_with_ty(self, ty: CSharpType) -> DeclareVariableExpr {
        DeclareVariableExpr { ty, ident: self }
    }
}

/// $ty $ident ;
pub struct DeclareVariableStmt {
    pub ty: CSharpType,
    pub ident: Ident,
}

impl lower::ToTokens for DeclareVariableStmt {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.ty.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        tokens.push(lower::Punct::Semicolon);
    }
}

impl BodyStatement for DeclareVariableStmt {}

/// (($ty) $source_ident);
pub struct Cast {
    pub ty: CSharpType,
    pub source: Box<dyn BodyExpression>,
}

impl lower::ToTokens for Cast {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        let mut content = lower::TokenStream::new();
        content.push(lower::Group {
            delimiter: lower::Delimiter::Paren,
            content: self.ty.to_token_stream(),
        });
        self.source.to_tokens(&mut content);

        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Paren,
            content,
        });
    }
}

impl BodyExpression for Cast {}

pub trait DoCast {
    fn cast(self, ty: CSharpType) -> Cast;
}

impl<T: BodyExpression + 'static> DoCast for T {
    fn cast(self, ty: CSharpType) -> Cast {
        Cast {
            ty,
            source: Box::new(self),
        }
    }
}

pub struct FieldAccess {
    pub root: Box<dyn BodyExpression>,
    pub field_name: Ident,
}

impl lower::ToTokens for FieldAccess {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.root.to_tokens(tokens);
        tokens.push(lower::Punct::Period);
        self.field_name.to_tokens(tokens);
    }
}

impl BodyExpression for FieldAccess {}

pub trait DoFieldAccess<I: Into<Ident>> {
    fn access_field(&self, field: I) -> FieldAccess;
}

impl<T: BodyExpression + Clone + 'static, I: Into<Ident>> DoFieldAccess<I> for T {
    fn access_field(&self, field_name: I) -> FieldAccess {
        FieldAccess {
            root: Box::new(self.clone()),
            field_name: field_name.into(),
        }
    }
}

pub struct AddressOf {
    pub target: Box<dyn BodyExpression>,
}

impl lower::ToTokens for AddressOf {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push(lower::Punct::Ampersand);
        self.target.to_tokens(tokens);
    }
}

impl BodyExpression for AddressOf {}

pub trait DoAddressOf {
    fn address_of(self) -> AddressOf;
}

impl<T: BodyExpression + 'static> DoAddressOf for T {
    fn address_of(self) -> AddressOf {
        AddressOf {
            target: Box::new(self),
        }
    }
}

pub struct Index {
    pub target: Box<dyn BodyExpression>,
    pub index: Box<dyn BodyExpression>,
}

impl lower::ToTokens for Index {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.target.to_tokens(tokens);
        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Bracket,
            content: self.index.to_token_stream(),
        })
    }
}

impl BodyExpression for Index {}

pub trait DoIndex<I: BodyExpression> {
    fn index(self, index: I) -> Index;
}

impl<I, T> DoIndex<I> for T
where
    I: BodyExpression + 'static,
    T: BodyExpression + 'static,
{
    fn index(self, index: I) -> Index {
        Index {
            target: Box::new(self),
            index: Box::new(index),
        }
    }
}

pub struct Assignment {
    pub lhs: Box<dyn BodyExpression>,
    pub rhs: Box<dyn BodyExpression>,
}

impl Assignment {
    pub fn new<Lhs, Rhs>(lhs: Lhs, rhs: Rhs) -> Self
    where
        Lhs: BodyExpression + 'static,
        Rhs: BodyExpression + 'static,
    {
        Self {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn stmt(self) -> AssignmentStatment {
        AssignmentStatment(self)
    }

    pub fn fixed(self) -> AssignmentFixed {
        AssignmentFixed(self)
    }
}

impl lower::ToTokens for Assignment {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.lhs.to_tokens(tokens);
        tokens.push(lower::Punct::Equals);
        self.rhs.to_tokens(tokens);
    }
}

impl BodyExpression for Assignment {}

pub struct AssignmentStatment(pub Assignment);

impl lower::ToTokens for AssignmentStatment {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.0.to_tokens(tokens);
        tokens.push(lower::Punct::Semicolon);
    }
}

impl BodyStatement for AssignmentStatment {}

pub struct AssignmentFixed(pub Assignment);

impl lower::ToTokens for AssignmentFixed {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push("fixed");
        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Paren,
            content: self.0.to_token_stream(),
        });
    }
}

impl BodyStatement for AssignmentFixed {
    fn requires_block(&self) -> bool {
        true
    }
}

pub struct UnsafeBlock();

impl lower::ToTokens for UnsafeBlock {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push("unsafe");
    }
}

impl BodyStatement for UnsafeBlock {
    fn requires_block(&self) -> bool {
        true
    }
}

pub struct TernaryExpression {
    pub predicate: Box<dyn BodyExpression>,
    pub true_branch: Box<dyn BodyExpression>,
    pub false_branch: Box<dyn BodyExpression>,
}

impl lower::ToTokens for TernaryExpression {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.predicate.to_tokens(tokens);
        tokens.push(lower::Punct::QuestionMark);
        self.true_branch.to_tokens(tokens);
        tokens.push(lower::Punct::Colon);
        self.false_branch.to_tokens(tokens);
    }
}

impl BodyExpression for TernaryExpression {}

/// $object.$method_name($args,*)
///
/// For method calls on the current object, set MethodCall::object to
/// Ident::Named("this")
#[derive(Debug, Clone)]
pub struct MethodCall {
    pub object: Option<Ident>,
    pub method: Ident,
    pub args: Vec<Ident>,
}

impl MethodCall {
    pub fn stmt(self) -> MethodCallStmt {
        MethodCallStmt(self)
    }
}

impl lower::ToTokens for MethodCall {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        if let Some(object) = &self.object {
            object.to_tokens(tokens);
            tokens.push(lower::Punct::Period);
        }
        self.method.to_tokens(tokens);

        let mut arg_group = lower::Group {
            delimiter: lower::Delimiter::Paren,
            content: lower::TokenStream::new(),
        };
        let mut first = true;
        for arg in &self.args {
            if !first {
                arg_group.content.push(lower::Punct::Comma);
            }
            first = false;
            arg.to_tokens(&mut arg_group.content);
        }

        tokens.push(arg_group);
    }
}

impl BodyExpression for MethodCall {}

pub struct MethodCallStmt(MethodCall);

impl lower::ToTokens for MethodCallStmt {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        self.0.to_tokens(tokens);
        tokens.push(lower::Punct::Semicolon);
    }
}

impl BodyStatement for MethodCallStmt {}

#[derive(Debug, Clone)]
pub struct Return {
    pub ident: Ident,
}

impl lower::ToTokens for Return {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push("return");
        self.ident.to_tokens(tokens);
        tokens.push(lower::Punct::Semicolon);
    }
}

impl BodyStatement for Return {}

/// [$name]
/// [$name(($args),*)]
pub struct Attribute {
    pub name: Ident,
    pub args: Vec<Box<dyn BodyExpression>>,
}

impl lower::ToTokens for Attribute {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        let mut content = lower::TokenStream::new();
        self.name.to_tokens(&mut content);

        if self.args.len() > 0 {
            let mut arg_content = lower::TokenStream::new();
            let mut first = true;
            for arg in &self.args {
                if !first {
                    arg_content.push(lower::Punct::Comma);
                }
                arg.to_tokens(&mut arg_content);
                first = false;
            }
            content.push(lower::Group {
                delimiter: lower::Delimiter::Paren,
                content: arg_content,
            })
        }

        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Bracket,
            content,
        });
        tokens.push(lower::Formatting::Newline);
    }
}

pub struct MethodArg {
    pub name: Ident,
    pub ty: CSharpType,
}

pub enum Visibility {
    Public,
    Private,
    Protected,
}

impl lower::ToTokens for Visibility {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push(match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Protected => "protected",
        });
    }
}

pub struct Method {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_extern: bool,
    pub name: Ident,
    pub return_type: CSharpType,
    pub args: Vec<MethodArg>,
    pub body: Option<Vec<Box<dyn BodyStatement>>>,
}

impl lower::ToTokens for Method {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        for attr in &self.attributes {
            attr.to_tokens(tokens);
        }

        self.visibility.to_tokens(tokens);

        if self.is_static {
            tokens.push("static");
        }

        if self.is_extern {
            tokens.push("extern");
        }

        self.return_type.to_tokens(tokens);
        self.name.to_tokens(tokens);

        let mut arg_tokens = lower::TokenStream::new();
        let mut first_arg = true;
        for arg in &self.args {
            if !first_arg {
                arg_tokens.push(lower::Punct::Comma);
            }
            first_arg = false;
            arg.ty.to_tokens(&mut arg_tokens);
            arg.name.to_tokens(&mut arg_tokens);
        }

        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Paren,
            content: arg_tokens,
        });

        match &self.body {
            Some(body) => tokens.push(lower::Group {
                delimiter: lower::Delimiter::Brace,
                content: body.to_token_stream(),
            }),
            None => tokens.push(Punct::Semicolon),
        }
    }
}

pub struct ObjectField {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub ty: CSharpType,
    pub name: Ident,
}

impl lower::ToTokens for ObjectField {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        for attr in &self.attributes {
            attr.to_tokens(tokens);
        }
        self.visibility.to_tokens(tokens);
        self.ty.to_tokens(tokens);
        self.name.to_tokens(tokens);
        tokens.push(lower::Punct::Semicolon);
    }
}

pub enum ObjectKind {
    Class,
    Struct,
}

impl lower::ToTokens for ObjectKind {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        tokens.push(match self {
            ObjectKind::Class => "class",
            ObjectKind::Struct => "struct",
        });
    }
}

pub struct Object {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub is_sealed: bool,
    pub is_static: bool,
    pub kind: ObjectKind,
    pub name: Ident,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<Method>,
}

impl lower::ToTokens for Object {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        for attr in &self.attributes {
            attr.to_tokens(tokens);
        }

        self.visibility.to_tokens(tokens);
        if self.is_sealed {
            tokens.push("sealed");
        }
        if self.is_static {
            tokens.push("static");
        }

        self.kind.to_tokens(tokens);

        self.name.to_tokens(tokens);

        let mut content = lower::TokenStream::new();

        for field in &self.fields {
            field.to_tokens(&mut content);
        }

        for method in &self.methods {
            method.to_tokens(&mut content);
        }

        tokens.push(lower::Group {
            delimiter: lower::Delimiter::Brace,
            content,
        });
    }
}

impl TopLevelElement for Object {}

/// Represents the entirity of a single C# source file
pub struct CsSource {
    pub elements: Vec<Box<dyn TopLevelElement>>,
}

impl lower::ToTokens for CsSource {
    fn to_tokens(&self, tokens: &mut lower::TokenStream) {
        for element in &self.elements {
            element.to_tokens(tokens);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::representations::level_0::ToTokens;

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
    fn test_using_statement() {
        to_tokens_test_helper(
            UsingStatement {
                path: "System.Runtime.InteropServices".into(),
            },
            "using System.Runtime.InteropServices ;",
        );
    }

    #[test]
    fn test_namespace_tokens_empty() {
        to_tokens_test_helper(
            Namespace {
                path: "Example.Namespace".into(),
                contents: Vec::new(),
            },
            "namespace Example.Namespace { }",
        );
    }

    #[test]
    fn test_namespace_tokens_non_empty() {
        to_tokens_test_helper(
            Namespace {
                path: "Example.Namespace".into(),
                contents: vec![Box::new(UsingStatement {
                    path: "Other.Namespace".into(),
                })],
            },
            "namespace Example.Namespace { using Other.Namespace ; }",
        );
    }

    #[test]
    fn test_cast_tokens() {
        to_tokens_test_helper(
            Cast {
                ty: CSharpType::Int16,
                source: Box::new(Ident::Named("source".to_string())),
            },
            "( ( Int16 ) source )",
        );
    }

    #[test]
    fn test_do_cast() {
        to_tokens_test_helper(
            Ident::Named("source".to_string()).cast(CSharpType::Int16),
            "( ( Int16 ) source )",
        );
    }

    #[test]
    fn test_method_call_tokens() {
        to_tokens_test_helper(
            MethodCall {
                object: Some("this".into()),
                method: "MethodName".into(),
                args: vec!["anArg".into(), "anotherArg".into()],
            },
            "this . MethodName ( anArg , anotherArg )",
        );

        to_tokens_test_helper(
            MethodCall {
                object: None,
                method: "MethodName".into(),
                args: vec!["anArg".into(), "anotherArg".into()],
            },
            "MethodName ( anArg , anotherArg )",
        );
    }

    #[test]
    fn test_return_tokens() {
        to_tokens_test_helper(
            Return {
                ident: Ident::new_generated(12),
            },
            "return _gen12 ;",
        );
    }

    #[test]
    fn test_attribute_tokens() {
        to_tokens_test_helper(
            Attribute {
                name: "TestAttr".into(),
                args: Vec::new(),
            },
            "[ TestAttr ]",
        );

        to_tokens_test_helper(
            Attribute {
                name: "TestAttr".into(),
                args: vec![Box::new(Ident::new("arg1"))],
            },
            "[ TestAttr ( arg1 ) ]",
        );

        to_tokens_test_helper(
            Attribute {
                name: "TestAttr".into(),
                args: vec![Box::new(Ident::new("arg1")), Box::new(Ident::new("arg2"))],
            },
            "[ TestAttr ( arg1 , arg2 ) ]",
        );
    }

    #[test]
    fn test_method_tokens() {
        let method = Method {
            attributes: vec![Attribute {
                name: "TestAttr".into(),
                args: vec![Box::new(Ident::new("arg1")), Box::new(Ident::new("arg2"))],
            }],
            visibility: Visibility::Public,
            is_static: false,
            is_extern: false,
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
            body: Some(vec![
                Box::new(
                    Assignment::new(
                        DeclareVariableExpr {
                            ty: CSharpType::Int64,
                            ident: Ident::new_generated(1),
                        },
                        MethodCall {
                            object: Some("this".into()),
                            method: "MethodName".into(),
                            args: vec!["arg1".into(), "barArg".into()],
                        },
                    )
                    .stmt(),
                ),
                Box::new(Return {
                    ident: Ident::Generated(GeneratedIdentId(1)),
                }),
            ]),
        };

        to_tokens_test_helper(
            method,
            "
            [ TestAttr ( arg1 , arg2 ) ]
            public Int64 FooMethod ( UInt16 arg1 , bool barArg ) 
            {
                Int64 _gen1 = this . MethodName ( arg1 , barArg ) ;
                return _gen1 ;
            }",
        );
    }
}
