use std::io;

use heck::{CamelCase, MixedCase};

use dotnet_bindgen_core::*;

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

#[derive(Clone, Default)]
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

impl AstNode for FfiType {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        match self {
            FfiType::Int { width, signed } => {
                let base = if *signed { "Int" } else { "UInt" };
                write!(f, "{}{}", base, width)?;
            },
            FfiType::Void => write!(f, "void")?,
        };

        Ok(())
    }
}

pub struct Root<'a> {
    pub children: Vec<Box<dyn AstNode + 'a>>,
}

impl<'a> Root<'a> {
    pub fn render(&self, f: &mut dyn io::Write) -> Result<(), io::Error> {
        let ctx = RenderContext::default();

        let mut first = true;
        for child in &self.children {
            if !first {
                // Extra blank line between top level blocks
                write!(f, "\n")?;
            }

            child.render(f, ctx.clone())?;
            first = false;
        }

        Ok(())
    }
}

pub struct LineComment {
    pub text: String,
}

impl AstNode for LineComment {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "// {}", self.text)
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

pub struct Namespace<'a> {
    pub name: String,
    pub children: Vec<Box<dyn AstNode + 'a>>,
}

impl<'a> AstNode for Namespace<'a> {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "namespace {}", self.name)?;
        render_ln!(f, &ctx, "{{")?;

        for child in &self.children {
            child.render(f, ctx.indented())?;
        }

        render_ln!(f, &ctx, "}}")?;

        Ok(())
    }
}

pub struct ImportedMethod<'a> {
    pub binary_name: String,
    pub func_data: BindgenFunction<'a>,
}

impl<'a> ImportedMethod<'a> {
    fn csharp_name(&self) -> String {
        self.func_data.name.to_camel_case()
    }
}

impl<'a> AstNode for ImportedMethod<'a> {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        render_ln!(f, &ctx, "[DllImport(\"{}\", entrypoint=\"{}\")]", self.binary_name, self.func_data.name)?;

        render_indent(f, &ctx)?;

        self.func_data.return_type.render(f, ctx.clone())?;
        write!(f, " {}(", self.csharp_name())?;

        // TODO: Implement Iterator for MaybeOwnedArr
        let mut first = true;
        for arg in &self.func_data.args[..] {
            if !first {
                write!(f, ", ")?;
            }

            arg.ffi_type.render(f, ctx.clone())?;
            write!(f, " {}", arg.name.to_mixed_case())?;
            first = false;
        }

        write!(f, ");\n")?;

        Ok(())
    }
}

pub struct Class<'a> {
    pub name: String,
    pub methods: Vec<ImportedMethod<'a>>,
    pub is_static: bool
}

impl<'a> AstNode for Class<'a> {
    fn render(&self, f: &mut dyn io::Write, ctx: RenderContext) -> Result<(), io::Error> {
        let static_part = if self.is_static { "static " } else { "" };
        render_ln!(f, &ctx, "public {}class {}", static_part, self.name)?;
        render_ln!(f, &ctx, "{{")?;

        for method in &self.methods {
            method.render(f, ctx.indented())?;
        }

        render_ln!(f, &ctx, "}}")?;

        Ok(())
    }
}