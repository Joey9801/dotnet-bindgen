use std::convert::TryFrom;
use std::path::PathBuf;

use heck::{CamelCase, MixedCase};

use dotnet_bindgen_core as core;
use crate::ast;
use crate::data::BindgenData;

/// A simple binding type requires no conversion to cross the FFI boundary
#[derive(Clone, Debug)]
struct SimpleBindingType {
    /// The original type descriptor extracted from the binary
    descriptor: core::BindgenTypeDescriptor,

    /// The single C# type that is both idiomatic, and suitable for the extern method.
    cs: ast::CSharpType,
}

/// A Complex BindingType is one that requires some manual marshalling.
#[derive(Clone, Debug)]
struct ComplexBindingType {
    /// The original type descriptor extracted from the binary
    descriptor: core::BindgenTypeDescriptor,

    /// The type as it appears in the generated Rust thunk
    thunk_type: ast::CSharpType,

    /// The type as it appears in the idiomatic C# wrapper
    idiomatic_type: ast::CSharpType,
}

/// Represents a type being passed between Rust/dotnet
#[derive(Clone, Debug)]
enum BindingType {
    Simple(SimpleBindingType),
    Complex(ComplexBindingType),
}

impl TryFrom<core::BindgenTypeDescriptor> for BindingType {
    type Error = &'static str;

    fn try_from(descriptor: core::BindgenTypeDescriptor) -> Result<Self, Self::Error> {
        use dotnet_bindgen_core::BindgenTypeDescriptor as Desc;
        use ast::CSharpType as CS;

        let converted = match &descriptor {
            Desc::Void => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::Void,
            }),
            Desc::Int { width: 8, signed: true } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::SByte,
            }),
            Desc::Int { width: 16, signed: true } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::Int16,
            }),
            Desc::Int { width: 32, signed: true } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::Int32,
            }),
            Desc::Int { width: 64, signed: true } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::Int64,
            }),
            Desc::Int { width: 8, signed: false } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::Byte,
            }),
            Desc::Int { width: 16, signed: false } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::UInt16,
            }),
            Desc::Int { width: 32, signed: false } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::UInt32,
            }),
            Desc::Int { width: 64, signed: false } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs: CS::UInt64,
            }),
            Desc::Slice { elem_type } => {
                let elem_type = match BindingType::try_from(*elem_type.clone())? {
                    BindingType::Simple(s) => s.cs,
                    BindingType::Complex(_) => return Err("Can't generate code for slices of non-trivial types yet"),
                };

                BindingType::Complex(ComplexBindingType {
                    descriptor,
                    thunk_type: CS::Struct { name: ast::Ident::new("SliceAbi") },
                    idiomatic_type: CS::Array { elem_type: Box::new(elem_type) }
                })
            }
            _ => unreachable!(),
        };

        Ok(converted)
    }
}

/// Maps a BindgenTypeDescriptor to the type it appears as in the generated thunk
fn map_descriptor_to_thunk_type(descriptor: &core::BindgenTypeDescriptor) -> ast::CSharpType {
    use dotnet_bindgen_core::BindgenTypeDescriptor as Desc;
    use ast::CSharpType as CS;
    match descriptor {
        Desc::Void => CS::Void,
        Desc::Int { width: 8, signed: true } => CS::SByte,
        Desc::Int { width: 16, signed: true } => CS::Int16,
        Desc::Int { width: 32, signed: true } => CS::Int32,
        Desc::Int { width: 64, signed: true } => CS::Int64,
        Desc::Int { width: 8, signed: false } => CS::Byte,
        Desc::Int { width: 16, signed: false } => CS::UInt16,
        Desc::Int { width: 32, signed: false } => CS::UInt32,
        Desc::Int { width: 64, signed: false } => CS::UInt64,
        Desc::Slice { elem_type: _ } => CS::Struct { name: ast::Ident("SliceAbi".to_string()) },
        _ => panic!("Untranslatable type")
    }
}

pub fn func_descriptor_to_imported_method(
    binary_name: &str,
    descriptor: &core::BindgenFunctionDescriptor
) -> ast::Method {

    let attributes = vec![
        ast::Attribute::dll_import(binary_name, &descriptor.thunk_name)
    ];
    let name = descriptor.thunk_name.to_string();
    let return_ty = map_descriptor_to_thunk_type(&descriptor.return_ty);
    let args = descriptor.arguments
        .iter()
        .map(|arg| ast::MethodArgument {
            name: ast::Ident(arg.name.to_string()),
            ty: map_descriptor_to_thunk_type(&arg.ty)
        }).collect();

    ast::Method {
        attributes,
        is_public: false,
        is_static: true,
        is_extern: true,
        is_unsafe: false,
        name,
        return_ty,
        args,
        body: None,
    }
}

/// Maps a BindgenTypeDescriptor to the type it should appear as in the generated C# wrapper
fn map_descriptor_to_idiomatic_type(descriptor: &core::BindgenTypeDescriptor) -> ast::CSharpType {
    use dotnet_bindgen_core::BindgenTypeDescriptor as Desc;
    use ast::CSharpType as CS;
    match descriptor {
        Desc::Slice { elem_type: slice_elem_type } =>
            CS::Array { elem_type: Box::new(map_descriptor_to_idiomatic_type(&slice_elem_type)) },
        other => map_descriptor_to_thunk_type(other),
    }
}

pub fn func_descriptor_to_idiomatic_wrapper(
    descriptor: &core::BindgenFunctionDescriptor
) -> ast::Method {
    let name = descriptor.real_name.to_camel_case();
    let return_ty = map_descriptor_to_idiomatic_type(&descriptor.return_ty);
    let args = descriptor.arguments
        .iter()
        .map(|arg| ast::MethodArgument {
            name: ast::Ident(arg.name.to_mixed_case()),
            ty: map_descriptor_to_idiomatic_type(&arg.ty)
        }).collect();

    let mut body = Vec::<Box<dyn ast::AstNode>>::new();

    let mut transformed_args = Vec::new();
    for arg in &descriptor.arguments {
        match &arg.ty {
            core::BindgenTypeDescriptor::Slice { elem_type } => {
                let elem_type = map_descriptor_to_thunk_type(&elem_type);

                let raw_name = arg.name.to_mixed_case();
                let transformed_name = format!("{}_transformed", raw_name);
                transformed_args.push(ast::Ident::new(&transformed_name));

                body.push(Box::new(ast::Statement {
                    expr: format!("SliceAbi {}", transformed_name)
                }));

                body.push(Box::new(ast::FixedAssignment {
                    assignment_expr: format!("{}* p = &{}[0]", elem_type, raw_name),
                    children: vec![
                        Box::new(ast::BlockComment {
                            text: vec![
                                "This is a bug, the rest of this method should be in this fixed block.".to_string(),
                                "Without this, the GC could move the array while Rust has a pointer to it.".to_string(),
                            ]
                        }),
                        Box::new(ast::Statement {
                            expr: format!("{}.Ptr = (IntPtr) p", transformed_name)
                        })
                    ],
                }));

                body.push(Box::new(ast::Statement {
                    expr: format!("{}.Len = (UInt64) {}.Length", transformed_name, raw_name)
                }));
            },

            // If not a slice, no transform needed
            _ => transformed_args.push(ast::Ident(arg.name.to_mixed_case())),
        }
    }

    let final_invocation = ast::MethodInvocation {
        target: None,
        method_name: ast::Ident::new(&descriptor.thunk_name),
        args: transformed_args,
    };

    if descriptor.return_ty == core::BindgenTypeDescriptor::Void {
        body.push(Box::new(final_invocation));
    } else {
        body.push(Box::new(ast::ReturnStatement {
            value: Some(Box::new(final_invocation)),
        }));
    }

    ast::Method {
        attributes: Vec::new(),
        is_public: true,
        is_static: true,
        is_extern: false,
        is_unsafe: true,
        name,
        return_ty,
        args,
        body: Some(body),
    }
}

struct CodegenInfo {
    /// Raw descriptor data extracted from the binary
    data: BindgenData,

    /// The parsed name of the library. Eg "libbindings_demo.so" -> "bindings_demo".
    ///
    /// It should be sufficient to use this string as the first argument to a DllImportAttribute.
    lib_name: String,

    /// The root directory for the outputted project / artifacts
    output_base: PathBuf,
}

impl CodegenInfo {
    fn new(data: BindgenData, output_base: PathBuf) -> Self {
        let source_ext = data.source_file.extension().and_then(|e| e.to_str());
        let source_stem = data
            .source_file
            .file_stem()
            .expect("Expect a filename in the path".into())
            .to_str()
            .expect("Expec the filename to be valid unicode");

        let lib_name = if source_stem.starts_with("lib") && source_ext == Some("so") {
            source_stem.chars().skip(3).collect::<String>()
        } else {
            source_stem.into()
        };

        Self {
            data,
            lib_name,
            output_base,
        }
    }

    fn write_all(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.output_base)?;
        // TODO: clean the directory?

        self.write_proj_file()?;

        let bindings_filepath = self.output_base.join("Bindings.cs");
        let mut file = std::fs::File::create(&bindings_filepath).expect(&format!(
            "Can't open {} for writing",
            bindings_filepath.to_str().unwrap()
        ));
        let ast = self.form_ast();
        ast.render(&mut file).unwrap();

        Ok(())
    }

    fn write_proj_file(&self) -> Result<(), std::io::Error> {
        let csproj_filename = format!("{}Bindings.csproj", self.lib_name.to_camel_case());
        let contents = r#"<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>netstandard2.0</TargetFramework>
        <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    </PropertyGroup>
</Project>
"#;
        std::fs::write(self.output_base.join(csproj_filename), contents)?;

        Ok(())
    }

    fn form_ast(&self) -> ast::Root {
        let mut methods = Vec::new();

        for descriptor in &self.data.descriptors {
            methods.push(
                func_descriptor_to_imported_method(&self.lib_name, &descriptor)
            );
            methods.push(
                func_descriptor_to_idiomatic_wrapper(&descriptor)
            );
        }

        ast::Root {
            file_comment: Some(ast::BlockComment {
                text: vec!["This is a generated file, do not modify by hand.".into()],
            }),
            using_statements: vec![
                ast::UsingStatement {
                    path: "System".into(),
                },
                ast::UsingStatement {
                    path: "System.Runtime.InteropServices".into(),
                },
            ],
            children: vec![Box::new(ast::Namespace {
                name: "Test.Namespace".into(),
                children: vec![
                    Box::new(ast::Object {
                        attributes: vec![
                            ast::Attribute::struct_layout("Sequential"),
                        ],
                        object_type: ast::ObjectType::Struct,
                        is_static: false,
                        name: "SliceAbi".into(),
                        methods: Vec::new(),
                        fields: vec![
                            ast::Field {
                                name: "Ptr".to_string(),
                                ty: ast::CSharpType::Struct { name: ast::Ident::new("IntPtr"), }
                            },
                            ast::Field {
                                name: "Len".to_string(),
                                ty: ast::CSharpType::UInt64,
                            },
                        ]
                    }),
                    Box::new(ast::Object {
                        attributes: Vec::new(),
                        object_type: ast::ObjectType::Class,
                        is_static: true,
                        name: "Imports".into(),
                        methods,
                        fields: Vec::new(),
                    }),
                ],
            })],
        }
    }
}

pub fn create_project(data: crate::data::BindgenData, project_dir: PathBuf) -> Result<(), ()> {
    let info = CodegenInfo::new(data, project_dir);
    info.write_all().unwrap();
    Ok(())
}
