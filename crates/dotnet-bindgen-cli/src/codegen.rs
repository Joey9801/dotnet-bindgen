use std::fs;
use std::path::{Path, PathBuf};

use heck::{CamelCase, MixedCase};

use dotnet_bindgen_core as core;
use crate::ast;
use crate::data::BindgenData;

fn create_csproj(data: &BindgenData, project_dir: &Path) -> Result<(), std::io::Error> {
    // Generate the proj filename
    // ./foo/bar/libtest_lib.so -> TestLibBindings.csproj

    let source_ext = data.source_file.extension().and_then(|e| e.to_str());

    let source_stem = data
        .source_file
        .file_stem()
        .expect("Source file path doesn't have a filename")
        .to_str()
        .expect("Source filename isn't valid unicode");

    // Silly .so file naming convention has an extra "lib" prefix on the filename
    // Strip it off if in that case
    let base_name = if source_stem.starts_with("lib") && source_ext == Some("so") {
        source_stem.chars().skip(3).collect::<String>()
    } else {
        source_stem.into()
    }
    .to_camel_case();

    let csproj_filename = project_dir.join(format!("{}Bindings.csproj", base_name));

    let contents = r#"<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>netstandard2.0</TargetFramework>
        <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    </PropertyGroup>
</Project>
"#;

    std::fs::create_dir_all(project_dir)?;
    std::fs::write(csproj_filename, contents)?;

    Ok(())
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

pub fn create_project(data: crate::data::BindgenData, project_dir: PathBuf) -> Result<(), ()> {
    create_csproj(&data, &project_dir).unwrap();

    let binary_name = data
        .source_file
        .file_name()
        .expect("Expect a filename in the path".into())
        .to_str()
        .expect("Expec the filename to be valid unicode");

    let mut methods = Vec::new();

    for descriptor in data.descriptors {
        methods.push(
            func_descriptor_to_imported_method(binary_name, &descriptor)
        );

        methods.push(
            func_descriptor_to_idiomatic_wrapper(&descriptor)
        );
    }

    // For the first pass, stick everything in one file
    let root = ast::Root {
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
    };

    fs::create_dir_all(&project_dir).unwrap();

    let file_path = project_dir.join("Bindings.cs");
    let mut file = std::fs::File::create(&file_path).expect(&format!(
        "Can't open {} for writing",
        file_path.to_str().unwrap()
    ));
    root.render(&mut file).unwrap();

    Ok(())
}
