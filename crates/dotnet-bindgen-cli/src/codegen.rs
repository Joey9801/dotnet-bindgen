use std::convert::{TryFrom, TryInto};
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

#[derive(Clone, Debug)]
struct BindingMethodArgument {
    ty: BindingType,
    rust_name: String,
    cs_name: String,
}

impl TryFrom<core::BindgenFunctionArgumentDescriptor> for BindingMethodArgument {
    type Error = &'static str;

    fn try_from(descriptor: core::BindgenFunctionArgumentDescriptor) -> Result<Self, Self::Error> {
        let ty = descriptor.ty.try_into()?;
        let rust_name = descriptor.name.to_string();
        let cs_name = descriptor.name.to_mixed_case();
        Ok(Self {
            ty,
            rust_name,
            cs_name,
        })
    }
}

impl BindingMethodArgument {
    fn transform_body_fragment(&self) -> ArgTransformBodyFragment {
        let (elements, output_ident) = match &self.ty {
            BindingType::Simple(_) => (Vec::new(), AbstractIdent::Explicit(self.cs_name.to_string())),
            BindingType::Complex(complex_ty) => {
                let elements = match &complex_ty.descriptor {
                    core::BindgenTypeDescriptor::Slice { elem_type: _ } => {
                        let elem_type = match &complex_ty.idiomatic_type {
                            ast::CSharpType::Array { elem_type } => elem_type.clone(),
                            _ => unreachable!(),
                        };

                        let source_ident = Box::new(BodyElement::Ident(
                            AbstractIdent::Explicit(self.cs_name.to_string())
                        ));

                        // TODO: The following is horrendous - replacing with a builer might help.
                        // Eg, something like:
                        //     let elements = ArgTransformFragmentBuilder::new()
                        //        .declare_struct(0.into(), "SliceAbi")
                        //        .assign_field_to_field(0.into(), "Len", self.cs_name.into(), "Length")
                        //        .fixed_assign_arr_ptr(1.into(), self.cs_name)
                        //        .build();

                        vec! [
                            BodyElement::DeclareLocal {
                                id: AbstractIdent::Generated(0),
                                ty: ast::CSharpType::Struct { name: "SliceAbi".into() }
                            },
                            BodyElement::Assignment {
                                lhs: Box::new(BodyElement::FieldAccess {
                                    element: Box::new(BodyElement::Ident(0.into())),
                                    field_name: "Len".to_string(),
                                }),
                                rhs: Box::new(BodyElement::FieldAccess {
                                    element: source_ident.clone(),
                                    field_name: "Length".to_string(),
                                }),
                            },
                            BodyElement::FixedAssignment {
                                ty: ast::CSharpType::Ptr {
                                    target: Box::new((*elem_type.clone()).into())
                                },
                                id: AbstractIdent::Generated(1),
                                rhs: Box::new(BodyElement::AddressOf {
                                    element: Box::new(BodyElement::IndexAccess {
                                        element: source_ident.clone(),
                                        index: 0,
                                    }),
                                })
                            },
                            BodyElement::Assignment {
                                lhs: Box::new(BodyElement::FieldAccess {
                                    element: Box::new(BodyElement::Ident(0.into())),
                                    field_name: "Ptr".to_string(),
                                }),
                                rhs: Box::new(BodyElement::Ident(0.into())),
                            },
                        ]
                    },

                    // Other descriptor types should fall under the Simple variant
                    _ => unreachable!(),
                };

                (elements, AbstractIdent::Generated(0))
            }
        };

        ArgTransformBodyFragment {
            elements,
            output_ident,
        }
    }
}

/// Abstract identifier for a variable, eventually resolved to a concrete ast::Ident.
#[derive(Clone, Debug)]
enum AbstractIdent {
    Explicit(String),
    Generated(u32),
}

impl From<u32> for AbstractIdent {
    fn from(num: u32) -> Self {
        AbstractIdent::Generated(num)
    }
}

impl From<&str> for AbstractIdent {
    fn from(name: &str) -> Self {
        AbstractIdent::Explicit(name.to_string())
    }
}

impl AbstractIdent {
    fn generated_id(&self) -> Option<u32> {
        match self {
            AbstractIdent::Explicit(_) => None,
            AbstractIdent::Generated(x) => Some(*x),
        }
    }

    fn apply_abstract_id_offset(&mut self, offset: u32) {
        match self {
            AbstractIdent::Explicit(_) => (),
            AbstractIdent::Generated(x) => *x += offset,
        };
    }
}

/// An abstract part of a method body, roughly mapping 1-1 with an ast element.
#[derive(Clone, Debug)]
enum BodyElement {
    Ident(AbstractIdent),
    /// Declares a new local variable of the given type.
    DeclareLocal {
        id: AbstractIdent,
        ty: ast::CSharpType,
    },
    /// Just calls a method.
    MethodCall {
        method_name: String,
        args: Vec<AbstractIdent>
    },
    /// A field/property of a variable, eg `foo.Length`.
    FieldAccess {
        element: Box<BodyElement>,
        field_name: String,
    },
    /// An index of some element, eg `foo[12]`.
    IndexAccess {
        element: Box<BodyElement>,
        index: i64,
    },
    /// Takes the address of the given element
    AddressOf {
        element: Box<BodyElement>,
    },
    Assignment {
        lhs: Box<BodyElement>,
        rhs: Box<BodyElement>,
    },
    /// Generates a fixed assignment, with subsequent operations inside its scope
    FixedAssignment {
        ty: ast::CSharpType,
        id: AbstractIdent,
        rhs: Box<BodyElement>
    }
}

impl BodyElement {
    /// What is the maximum abstract identifier id in this element, if any are present.
    fn max_abstract_id(&self) -> Option<u32> {
        match self {
            BodyElement::Ident(id) => id.generated_id(),
            BodyElement::DeclareLocal { id, ty: _ } => id.generated_id(),
            BodyElement::MethodCall { method_name: _, args } =>
                args.iter()
                    .filter_map(|a| a.generated_id())
                    .max(),
            BodyElement::FieldAccess { element, field_name: _ } => element.max_abstract_id(),
            BodyElement::IndexAccess { element, index: _ } => element.max_abstract_id(),
            BodyElement::AddressOf { element } => element.max_abstract_id(),
            BodyElement::Assignment { lhs, rhs } =>
                [lhs, rhs].iter()
                    .filter_map(|a| a.max_abstract_id())
                    .max(),
            BodyElement::FixedAssignment { ty: _, id, rhs } =>
                [id.generated_id(), rhs.max_abstract_id()].iter()
                    .filter(|a| a.is_some())
                    .map(|a| a.unwrap())
                    .max(),
        }
    }

    fn apply_abstract_id_offset(&mut self, offset: u32) {
        match self {
            BodyElement::Ident(id) => id.apply_abstract_id_offset(offset),
            BodyElement::DeclareLocal { id, ty: _ } => id.apply_abstract_id_offset(offset),
            BodyElement::MethodCall { method_name: _, args } => {
                for arg in args.iter_mut() {
                    arg.apply_abstract_id_offset(offset);
                }
            },
            BodyElement::FieldAccess { element, field_name: _ } => element.apply_abstract_id_offset(offset),
            BodyElement::IndexAccess { element, index: _ } => element.apply_abstract_id_offset(offset),
            BodyElement::AddressOf { element } => element.apply_abstract_id_offset(offset),
            BodyElement::Assignment { lhs, rhs } => {
                lhs.apply_abstract_id_offset(offset);
                rhs.apply_abstract_id_offset(offset);
            },
            BodyElement::FixedAssignment { ty: _, id, rhs } => {
                id.apply_abstract_id_offset(offset);
                rhs.apply_abstract_id_offset(offset);
            }
        }
    }
}

/// Represents a single part of method body, responsible for converting idiomatic C# types to their 
/// underlying FFI stable equivalents.
/// 
/// Instances of this struct for types which are already FFI stable will look something like:
/// ```
/// #let arg_name = "foo".to_string();
/// let frag = ArgTransformBodyElement {
///     elements: Vec::new(),
///     output_ident: AbstractIdent::Explicit(arg_name)
/// };
/// ```
#[derive(Clone, Debug)]
struct ArgTransformBodyFragment {
    elements: Vec<BodyElement>,
    output_ident: AbstractIdent,
}

impl ArgTransformBodyFragment {
    fn max_abstract_id(&self) -> Option<u32> {
        let max = self.elements
            .iter()
            .filter_map(|e| e.max_abstract_id())
            .max();

        match self.output_ident.generated_id() {
            Some(id) => assert!(id <= max.unwrap()),
            None => (),
        }

        max
    }

    fn apply_abstract_id_offset(&mut self, offset: u32) {
        for el in self.elements.iter_mut() {
            el.apply_abstract_id_offset(offset);
        }

        self.output_ident.apply_abstract_id_offset(offset)
    }
}

#[derive(Clone, Debug)]
struct BindingMethodBody {
    body_elements: Vec<BodyElement>,
}

#[derive(Clone, Debug)]
struct BindingMethod {
    args: Vec<BindingMethodArgument>,
    body: BindingMethodBody,
}

impl BindingMethod {
    fn new(descriptor: &core::BindgenFunctionDescriptor) -> Result<Self, &'static str> {
        let args = descriptor.arguments.iter()
            .map(|arg_desc| BindingMethodArgument::try_from(arg_desc.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        // Collect up the transform fragments, and ensure that their generated variable names don't collide.
        let mut transform_fragments: Vec<_> = args.iter()
            .map(|a| a.transform_body_fragment())
            .collect();
        let mut offset = 0;
        for frag in transform_fragments.iter_mut() {
            let offset_incr = frag.max_abstract_id().unwrap_or(0);
            frag.apply_abstract_id_offset(offset);
            offset += offset_incr;
        }

        let mut body_elements: Vec<_> = transform_fragments.iter()
            .flat_map(|frag| frag.elements.iter().cloned())
            .collect();

        let invocation_args: Vec<AbstractIdent> = transform_fragments.iter()
            .map(|frag| frag.output_ident.clone())
            .collect();

        body_elements.push(BodyElement::MethodCall {
            method_name: descriptor.thunk_name.to_string(),
            args: invocation_args,
        });

        Ok(Self { args, body: BindingMethodBody { body_elements }})
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

        // This definition leaves a load of extraneous whitespace in the generated csproj file.
        // I don't care, the code here will be read much more often than the generated file.
        let contents = r#"
            <Project Sdk="Microsoft.NET.Sdk">
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