use heck::MixedCase;

use dotnet_bindgen_core::{BindgenExportDescriptor, BindgenFunctionDescriptor};
use level_1::CSharpType;

use crate::{
    representations::{level_1, level_2},
};

fn int_cs_type(width: u8, signed: bool) -> CSharpType {
    match (width, signed) {
        (8, false) => CSharpType::Byte,
        (16, false) => CSharpType::UInt16,
        (32, false) => CSharpType::UInt32,
        (64, false) => CSharpType::UInt64,
        (8, true) => CSharpType::SByte,
        (16, true) => CSharpType::Int16,
        (32, true) => CSharpType::Int32,
        (64, true) => CSharpType::Int64,
        _ => panic!("Don't know how to handle strange width integers"),
    }
}

/// Create a conversion from an idiomatic C# type to the FFI type expected for the given
/// BindgenTypeDescriptor.
///
/// The ident given to this method is the ident of the input idiomatic C# variable
fn convert_to_ffi_stable(
    ident: level_1::Ident,
    ty: dotnet_bindgen_core::BindgenTypeDescriptor,
    id_gen: &mut level_1::IdentGenerator,
) -> Box<dyn level_2::CsToFfiStableConversion> {
    use dotnet_bindgen_core::BindgenTypeDescriptor as Desc;

    match ty {
        Desc::Int { width, signed } => Box::new(level_2::IdentityTypeConversion {
            ident,
            ty: int_cs_type(width, signed),
        }),
        Desc::Slice { elem_type } => {
            let element_type = match *elem_type {
                Desc::Int { width, signed } => int_cs_type(width, signed),
                _ => panic!("Can't handle slices of non-integer types"),
            };

            Box::new(level_2::CsArrToSliceAbi {
                source_ident: ident,
                source_type: element_type.clone().array_of(),
                dest_ident: id_gen.generate_ident(),
                dest_type: CSharpType::new_struct("SliceAbi"),
                element_type,
                temp_ptr_indent: id_gen.generate_ident(),
            })
        }
        Desc::Void => todo!(),
        Desc::Bool => Box::new(level_2::BoolToUint {
            source_ident: ident,
            dest_ident: id_gen.generate_ident(),
        }),
        Desc::Struct(_) => todo!(),
    }
}

fn create_binding_method(
    desc: &BindgenFunctionDescriptor,
    dll_name: &str,
) -> level_2::BindingMethod {
    let mut id_gen = level_1::IdentGenerator::new();

    let args = desc
        .arguments
        .iter()
        .map(|arg_desc| {
            let cs_name = arg_desc.name.to_mixed_case().into();
            convert_to_ffi_stable(cs_name, arg_desc.ty.clone(), &mut id_gen)
        })
        .collect();

    level_2::BindingMethod {
        source_descriptor: desc.clone(),
        dll_name: dll_name.to_string(),
        args,
    }
}

fn free_methods(input: &crate::SourceBinarySpec) -> Option<level_2::MethodContainer> {
    let name = "FreeMethods".into();
    let methods = input
        .bindgen_data
        .descriptors
        .iter()
        .filter_map(|d| match d {
            BindgenExportDescriptor::Function(f) => Some(f),
            BindgenExportDescriptor::Struct(_) => None,
        })
        .map(|d| create_binding_method(d, &input.base_name))
        .collect::<Vec<_>>();

    if methods.len() > 0 {
        Some(level_2::MethodContainer { name, methods })
    } else {
        None
    }
}

/// Add any struct definitions that should be implicitly available for bindgen methods
fn add_default_structs(structs: &mut Vec<level_2::BindingStruct>) {
    structs.push(level_2::BindingStruct {
        size: 16,
        alignment: 8,
        name: "SliceAbi".into(),
        fields: vec![
            level_2::BindingField {
                offset: 0,
                ty: level_1::CSharpType::new_struct("IntPtr"),
                name: "Pointer".to_string(),
            },
            level_2::BindingField {
                offset: 8,
                ty: level_1::CSharpType::UInt64,
                name: "Length".to_string(),
            },
        ],
    });
}

#[derive(Debug)]
pub struct EntryPass {}

impl super::Pass for EntryPass {
    type Input = crate::SourceBinarySpec;
    type Output = level_2::BindingModule;

    fn perform(&self, input: &Self::Input) -> Self::Output {
        let namespace = "Bindings.Generated".into();
        let free_methods = free_methods(input);

        let mut structs = Vec::new();
        add_default_structs(&mut structs);

        level_2::BindingModule {
            namespace,
            free_methods,
            structs,
        }
    }
}
