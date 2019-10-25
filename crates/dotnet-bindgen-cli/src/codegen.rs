use std::convert::{TryFrom, TryInto};

use heck::{CamelCase, MixedCase};

use crate::ast;
use crate::data::BindgenData;
use crate::path_ext::BinBaseName;

use dotnet_bindgen_core as core;

/// A simple binding type requires no conversion to cross the FFI boundary
#[derive(Clone, Debug)]
struct SimpleBindingType {
    /// The original type descriptor extracted from the binary
    descriptor: core::BindgenTypeDescriptor,

    /// The single C# type that is both idiomatic, and suitable for the extern method.
    cs_type: ast::CSharpType,
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

impl BindingType {
    fn native_type(&self) -> ast::CSharpType {
        match self {
            BindingType::Simple(s) => s.cs_type.clone(),
            BindingType::Complex(c) => c.thunk_type.clone(),
        }
    }

    fn idiomatic_type(&self) -> ast::CSharpType {
        match self {
            BindingType::Simple(s) => s.cs_type.clone(),
            BindingType::Complex(c) => c.idiomatic_type.clone(),
        }
    }
}

impl TryFrom<core::BindgenTypeDescriptor> for BindingType {
    type Error = &'static str;

    fn try_from(descriptor: core::BindgenTypeDescriptor) -> Result<Self, Self::Error> {
        use ast::CSharpType as CS;
        use dotnet_bindgen_core::BindgenTypeDescriptor as Desc;

        let converted = match &descriptor {
            Desc::Void => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::Void,
            }),
            Desc::Int {
                width: 8,
                signed: true,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::SByte,
            }),
            Desc::Int {
                width: 16,
                signed: true,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::Int16,
            }),
            Desc::Int {
                width: 32,
                signed: true,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::Int32,
            }),
            Desc::Int {
                width: 64,
                signed: true,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::Int64,
            }),
            Desc::Int {
                width: 8,
                signed: false,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::Byte,
            }),
            Desc::Int {
                width: 16,
                signed: false,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::UInt16,
            }),
            Desc::Int {
                width: 32,
                signed: false,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::UInt32,
            }),
            Desc::Int {
                width: 64,
                signed: false,
            } => BindingType::Simple(SimpleBindingType {
                descriptor,
                cs_type: CS::UInt64,
            }),
            Desc::Slice { elem_type } => {
                let elem_type = match BindingType::try_from(*elem_type.clone())? {
                    BindingType::Simple(s) => s.cs_type,
                    BindingType::Complex(_) => {
                        return Err("Can't generate code for slices of non-trivial types yet")
                    }
                };

                BindingType::Complex(ComplexBindingType {
                    descriptor,
                    thunk_type: CS::Struct {
                        name: ast::Ident::new("SliceAbi"),
                    },
                    idiomatic_type: CS::Array {
                        elem_type: Box::new(elem_type),
                    },
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
            BindingType::Simple(_) => (
                Vec::new(),
                AbstractIdent::Explicit(self.cs_name.to_string()),
            ),
            BindingType::Complex(complex_ty) => {
                let elements = match &complex_ty.descriptor {
                    core::BindgenTypeDescriptor::Slice { elem_type: _ } => {
                        let elem_type = match &complex_ty.idiomatic_type {
                            ast::CSharpType::Array { elem_type } => elem_type.clone(),
                            _ => unreachable!(),
                        };

                        let source_ident = Box::new(BodyElement::Ident(AbstractIdent::Explicit(
                            self.cs_name.to_string(),
                        )));

                        // TODO: The following is horrendous - replacing with a builder might help.
                        // Eg, something like:
                        //     let elements = ArgTransformFragmentBuilder::new()
                        //        .declare_struct(0.into(), "SliceAbi")
                        //        .assign_field_to_field(0.into(), "Len", self.cs_name.into(), "Length")
                        //        .fixed_assign_arr_ptr(1.into(), self.cs_name)
                        //        .build();

                        vec![
                            BodyElement::DeclareLocal {
                                id: AbstractIdent::Generated(0),
                                ty: ast::CSharpType::Struct {
                                    name: "SliceAbi".into(),
                                },
                            },
                            BodyElement::Assignment {
                                lhs: Box::new(BodyElement::FieldAccess {
                                    element: Box::new(BodyElement::Ident(0.into())),
                                    field_name: "Len".to_string(),
                                }),
                                rhs: Box::new(BodyElement::Cast {
                                    ty: ast::CSharpType::UInt64,
                                    element: Box::new(BodyElement::FieldAccess {
                                        element: source_ident.clone(),
                                        field_name: "Length".to_string(),
                                    }),
                                })
                            },
                            BodyElement::Unsafe,
                            BodyElement::FixedAssignment {
                                ty: ast::CSharpType::Ptr {
                                    target: Box::new((*elem_type.clone()).into()),
                                },
                                id: AbstractIdent::Generated(1),
                                rhs: Box::new(BodyElement::AddressOf {
                                    element: Box::new(BodyElement::IndexAccess {
                                        element: source_ident.clone(),
                                        index: 0,
                                    }),
                                }),
                            },
                            BodyElement::Assignment {
                                lhs: Box::new(BodyElement::FieldAccess {
                                    element: Box::new(BodyElement::Ident(0.into())),
                                    field_name: "Ptr".to_string(),
                                }),
                                rhs: Box::new(BodyElement::Cast {
                                    ty: ast::CSharpType::intptr(),
                                    element: Box::new(BodyElement::Ident(1.into())),
                                }),
                            },
                        ]
                    }

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

    fn to_concrete_ident(&self) -> ast::Ident {
        match self {
            AbstractIdent::Explicit(name) => ast::Ident(
                name.to_string()
            ),
            AbstractIdent::Generated(idx) => ast::Ident(
                format!("_gen{}", idx)
            ),
        }
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
        args: Vec<AbstractIdent>,
    },
    /// A field/property of a variable, eg `foo.Length`.
    FieldAccess {
        element: Box<BodyElement>,
        field_name: String,
    },
    /// An index of some element, eg `foo[12]`.
    IndexAccess {
        element: Box<BodyElement>,
        index: i32,
    },
    /// Takes the address of the given element
    AddressOf {
        element: Box<BodyElement>,
    },
    /// Casts a value to a given type
    Cast {
        ty: ast::CSharpType,
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
        rhs: Box<BodyElement>,
    },
    /// Wraps all elements after it in the rendered AST in an unsafe block
    Unsafe,
    Return {
        element: Option<Box<BodyElement>>,
    }
}

impl BodyElement {
    /// What is the maximum abstract identifier id in this element, if any are present.
    fn max_abstract_id(&self) -> Option<u32> {
        match self {
            BodyElement::Ident(id) => id.generated_id(),
            BodyElement::DeclareLocal { id, ty: _ } => id.generated_id(),
            BodyElement::MethodCall {
                method_name: _,
                args,
            } => args.iter().filter_map(|a| a.generated_id()).max(),
            BodyElement::FieldAccess {
                element,
                field_name: _,
            } => element.max_abstract_id(),
            BodyElement::IndexAccess { element, index: _ } => element.max_abstract_id(),
            BodyElement::AddressOf { element } => element.max_abstract_id(),
            BodyElement::Cast { ty: _, element } => element.max_abstract_id(),
            BodyElement::Assignment { lhs, rhs } => {
                [lhs, rhs].iter().filter_map(|a| a.max_abstract_id()).max()
            }
            BodyElement::FixedAssignment { ty: _, id, rhs } => {
                [id.generated_id(), rhs.max_abstract_id()]
                    .iter()
                    .filter(|a| a.is_some())
                    .map(|a| a.unwrap())
                    .max()
            },
            BodyElement::Unsafe => None,
            BodyElement::Return { element: Some(element) } => element.max_abstract_id(),
            BodyElement::Return { element: None } => None,
        }
    }

    fn apply_abstract_id_offset(&mut self, offset: u32) {
        match self {
            BodyElement::Ident(id) => id.apply_abstract_id_offset(offset),
            BodyElement::DeclareLocal { id, ty: _ } => id.apply_abstract_id_offset(offset),
            BodyElement::MethodCall {
                method_name: _,
                args,
            } => {
                for arg in args.iter_mut() {
                    arg.apply_abstract_id_offset(offset);
                }
            }
            BodyElement::FieldAccess {
                element,
                field_name: _,
            } => element.apply_abstract_id_offset(offset),
            BodyElement::IndexAccess { element, index: _ } => {
                element.apply_abstract_id_offset(offset)
            }
            BodyElement::AddressOf { element } => element.apply_abstract_id_offset(offset),
            BodyElement::Cast { ty: _, element } => element.apply_abstract_id_offset(offset),
            BodyElement::Assignment { lhs, rhs } => {
                lhs.apply_abstract_id_offset(offset);
                rhs.apply_abstract_id_offset(offset);
            }
            BodyElement::FixedAssignment { ty: _, id, rhs } => {
                id.apply_abstract_id_offset(offset);
                rhs.apply_abstract_id_offset(offset);
            },
            BodyElement::Unsafe => (),
            BodyElement::Return { element: Some(element) } => element.apply_abstract_id_offset(offset),
            BodyElement::Return { element: None } => (),
        }
    }

    fn requires_new_scope(&self) -> bool {
        match self {
            BodyElement::Ident (_) => false,
            BodyElement::DeclareLocal {..} => false,
            BodyElement::MethodCall {..} => false,
            BodyElement::FieldAccess {..} => false,
            BodyElement::IndexAccess {..} => false,
            BodyElement::AddressOf {..} => false,
            BodyElement::Cast {..} => false,
            BodyElement::Assignment {..} => false,
            BodyElement::FixedAssignment {..} => true,
            BodyElement::Unsafe => true,
            BodyElement::Return{..} => false,
        }
    }

    fn is_top_level(&self) -> bool {
        match self {
            BodyElement::Ident (_) => false,
            BodyElement::DeclareLocal {..} => true,
            BodyElement::MethodCall {..} => false,
            BodyElement::FieldAccess {..} => false,
            BodyElement::IndexAccess {..} => false,
            BodyElement::AddressOf {..} => false,
            BodyElement::Cast {..} => false,
            BodyElement::Assignment {..} => false,
            BodyElement::FixedAssignment {..} => true,
            BodyElement::Unsafe => true,
            BodyElement::Return{..} => true,
        }
    }

    fn to_ast_node(&self) -> Box<dyn ast::AstNode> {
        match self {
            BodyElement::Ident(id) => Box::new(id.to_concrete_ident()),
            BodyElement::DeclareLocal { id, ty } => Box::new(
                ast::VariableDeclaration {
                    name: id.to_concrete_ident(),
                    ty: ty.clone()
                }
            ),
            BodyElement::MethodCall { method_name, args } => {
                let args = args.iter()
                    .map(|a| a.to_concrete_ident())
                    .collect();
                Box::new(
                    ast::MethodInvocation {
                        target: None,
                        method_name: ast::Ident(method_name.to_string()),
                        args,
                    }
                )
            },
            BodyElement::FieldAccess { element, field_name } => Box::new(
                ast::FieldAccess {
                    element: element.to_ast_node(),
                    field_name: ast::Ident(field_name.to_string()),
                }
            ),
            BodyElement::IndexAccess { element, index } => Box::new(
                ast::IndexAccess {
                    element: element.to_ast_node(),
                    index: *index,
                }
            ),
            BodyElement::AddressOf { element } => Box::new(
                ast::AddressOf {
                    element: element.to_ast_node(),
                }
            ),
            BodyElement::Cast { ty, element } => Box::new(
                ast::Cast {
                    ty: ty.clone(),
                    element: element.to_ast_node(),
                }
            ),
            BodyElement::Assignment { lhs, rhs } => Box::new(
                ast::Assignment {
                    lhs: lhs.to_ast_node(),
                    rhs: rhs.to_ast_node(),
                }
            ),
            BodyElement::FixedAssignment { ty, id, rhs } => Box::new(
                ast::FixedAssignment {
                    ty: ty.clone(),
                    id: id.to_concrete_ident(),
                    rhs: rhs.to_ast_node(),
                }
            ),
            BodyElement::Unsafe => Box::new(
                ast::UnsafeStatement {}
            ),
            BodyElement::Return { element } => {
                Box::new(ast::ReturnStatement {
                    value: match element {
                        Some(element) => Some(element.to_ast_node()),
                        None => None,
                    }
                })
            },
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
        let max = self
            .elements
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

impl BindingMethodBody {
    pub fn new(
        descriptor: &core::BindgenFunctionDescriptor,
        args: &[BindingMethodArgument]
    ) -> Self {
        let mut transform_fragments: Vec<_> =
            args.iter().map(|a| a.transform_body_fragment()).collect();

        // Ensure that their generated idents from each fragment don't intersect
        let mut offset = 0;
        for frag in transform_fragments.iter_mut() {
            match frag.max_abstract_id() {
                Some(m) => {
                    frag.apply_abstract_id_offset(offset);
                    offset += m + 1;
                }
                None => ()
            }
        }

        let mut body_elements: Vec<_> = transform_fragments
            .iter()
            .flat_map(|frag| frag.elements.iter().cloned())
            .collect();

        // Add one final body element, calling the bound method with all of the (possibly) transformed arguments.
        let invocation_args: Vec<AbstractIdent> = transform_fragments
            .iter()
            .map(|frag| frag.output_ident.clone())
            .collect();

        let underlying_call = BodyElement::MethodCall {
            method_name: descriptor.thunk_name.to_string(),
            args: invocation_args,
        };

        if descriptor.return_ty != core::BindgenTypeDescriptor::Void {
            body_elements.push(BodyElement::Return {
                element: Some(Box::new(underlying_call))
            });
        } else {
            body_elements.push(underlying_call);
        }

        Self { body_elements }
    }

    pub fn to_ast_nodes(&self) -> Vec<Box<dyn ast::AstNode>> {
        fn render_elements<'a>(elements: &'a mut impl Iterator<Item = &'a BodyElement>) -> Vec<Box<dyn ast::AstNode>> {
            let mut ast_nodes = Vec::new();
            let mut next = elements.next();
            while let Some(el) = next {
                ast_nodes.push({
                    let node = el.to_ast_node();
                    if el.is_top_level() {
                        node
                    } else {
                        Box::new(ast::Statement {
                            expr: node
                        })
                    }
                });

                if el.requires_new_scope() {
                    ast_nodes.push(Box::new(ast::Scope {
                        children: render_elements(elements),
                    }));
                    break;
                }

                next = elements.next();
            }

            ast_nodes
        }

        render_elements(&mut self.body_elements.iter())
    }
}

#[derive(Clone, Debug)]
struct BindingMethod {
    args: Vec<BindingMethodArgument>,

    return_ty: BindingType,

    /// The name of the binary containing the method, suitable for using directly in a DllImport attribute.
    binary_name: String,

    /// The name of the method that received the original #[dotnet_bindgen] attribute
    /// 
    /// This isn't neccesarily unique among the bindings, or the name of the symbol in the binary,
    /// as the if a thunk is generated the method doens't have to have #[no_mangle] attached.
    rust_name: String,

    /// The symbol name of the generated rust thunk, if one was generated.
    /// 
    /// Guaranteed to be unique among the bindings.
    rust_thunk_name: String,

    /// The name of the C# method to expose from the bindings BindingMethodBody
    /// 
    /// Typically just rust_name.to_camel_case().
    cs_name: String,

    /// If a C# thunk must be generated, the body of that thunk.
    cs_thunk_body: Option<BindingMethodBody>,
}

impl BindingMethod {
    pub fn new(binary_name: &str, descriptor: &core::BindgenFunctionDescriptor) -> Result<Self, &'static str> {
        let binary_name = binary_name.to_string();

        let args = descriptor
            .arguments
            .iter()
            .map(|arg_desc| BindingMethodArgument::try_from(arg_desc.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let return_ty = descriptor.return_ty.clone().try_into()?;

        let rust_name = descriptor.real_name.to_string();
        let rust_thunk_name = descriptor.thunk_name.to_string();
        let cs_name = rust_name.to_camel_case();

        let cs_thunk_body = Some(BindingMethodBody::new(descriptor, &args));

        Ok(Self {
            binary_name,
            args,
            return_ty,
            rust_name,
            rust_thunk_name,
            cs_name,
            cs_thunk_body,
        })
    }

    /// Generate the ast nodes for this bound method
    /// 
    /// This may be more than one method, eg if a thunk is needed to marshall arguments/return values to/from
    /// an FFI stable representation.
    pub fn to_ast_methods(&self) -> Vec<ast::Method> {
        vec![
            self.dll_imported_method(),
            self.thunk_method(),
        ]
    }

    fn dll_imported_method(&self) -> ast::Method {
        let attributes = vec![
            ast::Attribute::dll_import(&self.binary_name, &self.rust_thunk_name)
        ];

        let return_ty = self.return_ty.native_type();

        let args = self.args
            .iter()
            .map(|arg| ast::MethodArgument {
                name: arg.rust_name.as_str().into(),
                ty: arg.ty.native_type(),
            })
            .collect();

        ast::Method {
            attributes,
            is_public: false,
            is_static: true,
            is_extern: true,
            is_unsafe: false,
            name: self.rust_thunk_name.to_string(),
            return_ty,
            args,
            body: None,
        }
    }

    fn thunk_method(&self) -> ast::Method {
        let attributes = Vec::new();

        let name = self.cs_name.to_string();

        // TODO: Make this the idiomatic type + add the relevant marshalling to the body.
        let return_ty = self.return_ty.native_type();

        let args = self.args
            .iter()
            .map(|arg| ast::MethodArgument {
                name: arg.cs_name.as_str().into(),
                ty: arg.ty.idiomatic_type(),
            })
            .collect();
        
        let body = Some(self.cs_thunk_body
            .as_ref()
            .unwrap()
            .to_ast_nodes()
        );

        ast::Method {
            attributes,
            is_public: true,
            is_static: true,
            is_extern: false,
            is_unsafe: false,
            name,
            return_ty,
            args,
            body,
        }
    }
}

/// Maps a BindgenTypeDescriptor to the type it appears as in the generated thunk
struct CodegenInfo<'a> {
    /// Raw descriptor data extracted from the binary
    data: &'a BindgenData,

    /// The parsed name of the library. Eg "libbindings_demo.so" -> "bindings_demo".
    ///
    /// It should be sufficient to use this string as the first argument to a DllImportAttribute.
    lib_name: String,
}

impl<'a> CodegenInfo<'a> {
    fn new(data: &'a BindgenData) -> Self {
        let lib_name = data.source_file.bin_base_name();
        Self {
            data,
            lib_name,
        }
    }

    fn form_ast(&self) -> ast::Root {
        let methods = self.data.descriptors.iter()
            .map(|descriptor| BindingMethod::new(&self.lib_name, descriptor))
            .collect::<Result<Vec<_>, _>>().expect("Failed to process method")
            .iter()
            .flat_map(|bmethod| bmethod.to_ast_methods())
            .collect();

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
                name: self.lib_name.to_camel_case().into(),
                children: vec![
                    Box::new(ast::Object {
                        attributes: vec![ast::Attribute::struct_layout("Sequential")],
                        object_type: ast::ObjectType::Struct,
                        is_static: false,
                        name: "SliceAbi".into(),
                        methods: Vec::new(),
                        fields: vec![
                            ast::Field {
                                name: "Ptr".to_string(),
                                ty: ast::CSharpType::Struct {
                                    name: ast::Ident::new("IntPtr"),
                                },
                            },
                            ast::Field {
                                name: "Len".to_string(),
                                ty: ast::CSharpType::UInt64,
                            },
                        ],
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

pub fn form_ast_from_data(data: &BindgenData) -> ast::Root {
    let info = CodegenInfo::new(data);
    info.form_ast()
}