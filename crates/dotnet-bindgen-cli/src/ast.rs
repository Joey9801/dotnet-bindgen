use std::io;
use std::string::ToString;
use std::fmt;

static INDENT_TOK: &'static str = "    ";

fn render_indent(f: &mut dyn io::Write, ctx: &RenderContext) -> Result<(), io::Error> {
    for _ in 0..ctx.indent_level {
        write!(f, "{}", INDENT_TOK)?;
    }

    Ok(())
}

macro_rules! render_ln {
    ($f:ident, &$ctx:ident, $($args:expr),+) => {
        {
            let mut result = render_indent($f, &$ctx);

            if result.is_ok() {
                result = write!($f, $($args),+);
            }

            if result.is_ok() {
                result = write!($f, "\n");
            }
            result
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct RenderContext {
    indent_level: u8,
}

impl RenderContext {
    fn indented(&self) -> Self {
        RenderContext {
            indent_level: self.indent_level + 1,
            ..*self
        }
    }
}

pub trait AstNode {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error>;
}

impl<T: fmt::Display> AstNode for T {
    fn render(&self, f: &mut dyn io::Write, _ctx: RenderContext) -> Result<(), io::Error> {
        write!(f, "{}", self)
    }

}

pub struct Root {
    pub file_comment: Option<BlockComment>,
    pub using_statements: Vec<UsingStatement>,
    pub children: Vec<Box<dyn AstNode>>,
}

impl Root {
    pub fn render(&self, f: &mut dyn io::Write) -> Result<(), io::Error> {
        let ctx = RenderContext::default();

        let mut first = true;

        match &self.file_comment {
            Some(c) => {
                c.render(f, ctx)?;
                first = false;
            }
            None => (),
        }

        if !first && !self.using_statements.is_empty() {
            write!(f, "\n")?;
        }

        for using in &self.using_statements {
            using.render(f, ctx)?;
            first = false;
        }

        for child in &self.children {
            if !first {
                write!(f, "\n")?;
            }

            child.render(f, ctx)?;
            first = false;
        }

        Ok(())
    }
}

pub struct BlockComment {
    pub text: Vec<String>,
}

impl AstNode for BlockComment {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "/*")?;
        for line in &self.text {
            render_ln!(f, &ctx, " * {}", line)?;
        }
        render_ln!(f, &ctx, " */")?;

        Ok(())
    }
}

pub struct UsingStatement {
    pub path: String,
}

impl AstNode for UsingStatement {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "using {};", self.path)
    }
}

pub struct Namespace {
    pub name: String,
    pub children: Vec<Box<dyn AstNode>>,
}

impl AstNode for Namespace {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "namespace {}", self.name)?;
        render_ln!(f, &ctx, "{{")?;

        let mut first = true;
        for child in &self.children {
            if !first {
                write!(f, "\n")?;
            }
            first = false;

            child.render(f, ctx.indented())?;
        }

        render_ln!(f, &ctx, "}}")?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum CSharpType {
    Void,

    /// SByte == Int8, but Int8 isn't a thing for some reason.
    SByte,
    Int16,
    Int32,
    Int64,

    /// Byte == UInt8, but UInt8 isn't a thing for some reason
    Byte,
    UInt16,
    UInt32,
    UInt64,

    Array { elem_type: Box<CSharpType> },

    Ptr { target: Box<CSharpType> },

    Struct { name: Ident }
}


impl fmt::Display for CSharpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CSharpType::Void   => write!(f, "void"),
            CSharpType::SByte  => write!(f, "SByte"),
            CSharpType::Int16  => write!(f, "Int16"),
            CSharpType::Int32  => write!(f, "Int32"),
            CSharpType::Int64  => write!(f, "Int64"),
            CSharpType::Byte   => write!(f, "Byte"),
            CSharpType::UInt16 => write!(f, "UInt16"),
            CSharpType::UInt32 => write!(f, "UInt32"),
            CSharpType::UInt64 => write!(f, "UInt64"),
            CSharpType::Array { elem_type } => write!(f, "{}[]", elem_type),
            CSharpType::Ptr { target } => write!(f, "{}*", target),
            CSharpType::Struct { name } => write!(f, "{}", name),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ident(pub String);

impl Ident {
    pub fn new(s: &str) -> Self {
        Self (s.to_string())
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub enum LiteralValue {
    Integer(i64),
    QuotedString(String),
    Boolean(bool),
    EnumValue(String, String),
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiteralValue::Integer(val) => write!(f, "{}", val),
            LiteralValue::QuotedString(val) => write!(f, "\"{}\"", val),
            LiteralValue::Boolean(val) => write!(f, "{}", val),
            LiteralValue::EnumValue(e, v) => write!(f, "{}.{}", e, v),
        }
    }
}


pub struct Attribute {
    pub name: String,
    pub positional_parameters: Vec<LiteralValue>,
    pub named_parameters: Vec<(Ident, LiteralValue)>,
}

impl Attribute {
    pub fn dll_import(binary: &str, entrypoint: &str) -> Self {
        Self {
            name: "DllImport".to_string(),
            positional_parameters: vec![
                LiteralValue::QuotedString(binary.to_string()),
            ],
            named_parameters: vec![
                (Ident("EntryPoint".to_string()), LiteralValue::QuotedString(entrypoint.to_string()))
            ],
        }
    }

    pub fn struct_layout(layout_kind: &str) -> Self {
        Self {
            name: "StructLayout".to_string(),
            positional_parameters: vec![
                LiteralValue::EnumValue("LayoutKind".to_string(), layout_kind.to_string()),
            ],
            named_parameters: Vec::new(),
        }
    }
}


impl AstNode for Attribute {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_indent(f, &ctx)?;
        write!(f, "[{}", self.name)?;

        if self.positional_parameters.len() + self.named_parameters.len() == 0 {
            write!(f, "]\n")?;
            return Ok(())
        } else {
            write!(f, "(")?;
        }

        let mut first = true;
        for param in &self.positional_parameters {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "{}", param)?;
        }

        for (key, value) in &self.named_parameters {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "{} = {}", key, value)?;
        }

        write!(f, ")]\n")?;

        Ok(())
    }
}

pub struct Statement {
    pub expr: String,
}

impl AstNode for Statement {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "{};", self.expr)
    }
}

pub struct FixedAssignment {
    pub assignment_expr: String,
    pub children: Vec<Box<dyn AstNode>>,
}

impl AstNode for FixedAssignment {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "fixed ({})", self.assignment_expr)?;
        render_ln!(f, &ctx, "{{")?;

        for node in &self.children {
            node.render(f, ctx.indented())?;
        }

        render_ln!(f, &ctx, "}}")
    }
}

pub struct MethodInvocation {
    pub target: Option<Ident>,
    pub method_name: Ident,
    pub args: Vec<Ident>,
}

impl fmt::Display for MethodInvocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(t) = &self.target {
            write!(f, "{}.", t)?;
        }

        write!(f, "{}(", self.method_name)?;

        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}

pub struct ReturnStatement {
    pub value: Option<Box<dyn AstNode>>,
}

impl AstNode for ReturnStatement {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        match &self.value {
            Some(v) => {
                render_indent(f, &ctx)?;
                write!(f, "return ")?;
                v.render(f, ctx)?;
                write!(f, ";\n")
            },
            None => render_ln!(f, &ctx, "return;"),
        }
    }
}

pub struct MethodArgument {
    pub name: Ident,
    pub ty: CSharpType,
}

impl AstNode for MethodArgument {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        write!(f, "{} {}", self.ty, self.name)
    }
}

pub struct Method {
    pub attributes: Vec<Attribute>,
    pub is_public: bool,
    pub is_static: bool,
    pub is_extern: bool,
    pub is_unsafe: bool,
    pub name: String,
    pub return_ty: CSharpType,
    pub args: Vec<MethodArgument>,
    pub body: Option<Vec<Box<dyn AstNode>>>,
}

impl AstNode for Method {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        for attr in &self.attributes {
            attr.render(f, ctx)?;
        }

        render_indent(f, &ctx)?;
        if self.is_public {
            write!(f, "public ")?;
        } else {
            write!(f, "private ")?;
        }

        if self.is_static {
            write!(f, "static ")?;
        }

        if self.is_extern {
            write!(f, "extern ")?;
        }

        if self.is_unsafe {
            write!(f, "unsafe ")?;
        }

        write!(f, "{} {}(", self.return_ty, self.name)?;

        let mut first = true;
        for arg in &self.args {
            if !first {
                write!(f, ", ")?;
            }
            first = false;

            arg.render(f, ctx)?;
        }

        let body = match &self.body {
            Some(b) => b,
            None => {
                write!(f, ");\n")?;
                return Ok(())
            }
        };

        write!(f, ")\n")?;
        render_ln!(f, &ctx, "{{")?;
        for node in body {
            node.render(f, ctx.indented())?;
        }
        render_ln!(f, &ctx, "}}")?;

        Ok(())
    }
}

pub struct Field {
    pub name: String,
    pub ty: CSharpType,
}

impl AstNode for Field {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "public {} {};", self.ty, self.name)
    }
}

pub enum ObjectType {
    Class,
    Struct,
}

pub struct Object {
    pub attributes: Vec<Attribute>,
    pub object_type: ObjectType,
    pub is_static: bool,
    pub name: String,
    pub methods: Vec<Method>,
    pub fields: Vec<Field>,
}

impl AstNode for Object {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        for attr in &self.attributes {
            attr.render(f, ctx)?;
        }

        let static_part = if self.is_static { "static " } else { "" };
        let object_type = match self.object_type {
            ObjectType::Class => "class ",
            ObjectType::Struct => "struct ",
        };

        render_ln!(f, &ctx, "public {}{}{}", static_part, object_type, self.name)?;
        render_ln!(f, &ctx, "{{")?;

        let mut first = true;

        for field in &self.fields {
            first = false;
            field.render(f, ctx.indented())?;
        }

        for method in &self.methods {
            if !first {
                write!(f, "\n")?;
            }
            first = false;

            method.render(f, ctx.indented())?;
        }

        render_ln!(f, &ctx, "}}")?;

        Ok(())
    }
}