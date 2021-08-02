use heck::{CamelCase, MixedCase};

use super::level_1 as lower;
use lower::{DoAddressOf, DoCast, DoDeclare, DoFieldAccess, DoIndex};

pub type LayerEntrypoint = BindingModule;

/// An element of this rep ty: (), ident: ()  ty: (), ident: ()  ty: (), ident: () resentation which can be lowered to a part of a method body
pub trait BodyFragment {
    fn to_statements(&self) -> Vec<Box<dyn lower::BodyStatement>>;
}

/// Converts an identifier from an idiomatic C# type to something able to cross ident: ()  the FFI boundary
pub trait CsToFfiStableConversion: BodyFragment {
    fn source_ident(&self) -> &lower::Ident;
    fn source_type(&self) -> &lower::CSharpType;
    fn dest_ident(&self) -> &lower::Ident;
    fn dest_type(&self) -> &lower::CSharpType;
    fn requires_unsafe(&self) -> bool;
}

/// Converts an FFI stable type back into an idiomatic C# type
pub trait FfiStableToCsConversion: BodyFragment {
    fn source_ident(&self) -> &lower::Ident;
    fn source_type(&self) -> &lower::CSharpType;
    fn dest_ident(&self) -> &lower::Ident;
    fn dest_type(&self) -> &lower::CSharpType;
    fn requires_unsafe(&self) -> bool;
}

/// A transformation that does nothing.
///
/// Used for types which are simultaneously idiomatic and FFI stable
pub struct IdentityTypeConversion {
    pub ident: lower::Ident,
    pub ty: lower::CSharpType,
}

impl BodyFragment for IdentityTypeConversion {
    fn to_statements(&self) -> Vec<Box<dyn lower::BodyStatement>> {
        Vec::new()
    }
}

impl CsToFfiStableConversion for IdentityTypeConversion {
    fn source_ident(&self) -> &lower::Ident {
        &self.ident
    }

    fn source_type(&self) -> &lower::CSharpType {
        &self.ty
    }

    fn dest_ident(&self) -> &lower::Ident {
        &self.ident
    }

    fn dest_type(&self) -> &lower::CSharpType {
        &self.ty
    }

    fn requires_unsafe(&self) -> bool {
        false
    }
}

impl FfiStableToCsConversion for IdentityTypeConversion {
    fn source_ident(&self) -> &lower::Ident {
        &self.ident
    }

    fn source_type(&self) -> &lower::CSharpType {
        &self.ty
    }

    fn dest_ident(&self) -> &lower::Ident {
        &self.ident
    }

    fn dest_type(&self) -> &lower::CSharpType {
        &self.ty
    }

    fn requires_unsafe(&self) -> bool {
        false
    }
}

/// Convert a managed C# 1D array of FFI-stable elements into a SliceAbi struct
pub struct CsArrToSliceAbi {
    pub source_ident: lower::Ident,
    pub source_type: lower::CSharpType,
    pub dest_ident: lower::Ident,
    pub dest_type: lower::CSharpType,
    pub element_type: lower::CSharpType,
    // The ident used to temporarily hold the pointer in the fixed statement
    pub temp_ptr_indent: lower::Ident,
}

impl CsArrToSliceAbi {
    pub fn new(
        source_ident: impl Into<lower::Ident>,
        element_type: lower::CSharpType,
        ident_gen: &mut lower::IdentGenerator,
    ) -> Self {
        Self {
            source_ident: source_ident.into(),
            source_type: element_type.clone().array_of(),
            dest_ident: ident_gen.generate_ident(),
            dest_type: lower::CSharpType::new_struct("SliceAbi"),
            element_type: element_type.clone(),
            temp_ptr_indent: ident_gen.generate_ident(),
        }
    }
}

impl BodyFragment for CsArrToSliceAbi {
    fn to_statements(&self) -> Vec<Box<dyn lower::BodyStatement>> {
        vec![
            Box::new(
                self.dest_ident
                    .clone()
                    .declare_with_ty(lower::CSharpType::new_struct("SliceAbi"))
                    .stmt(),
            ),
            Box::new(
                lower::Assignment::new(
                    self.dest_ident.access_field("Len"),
                    self.source_ident
                        .access_field("Length")
                        .cast(lower::CSharpType::UInt64),
                )
                .stmt(),
            ),
            Box::new(
                lower::Assignment::new(
                    self.temp_ptr_indent
                        .clone()
                        .declare_with_ty(self.element_type.clone().ptr_to()),
                    self.source_ident
                        .clone()
                        .index(lower::Literal::Integer(0))
                        .address_of(),
                )
                .fixed(),
            ),
            Box::new(
                lower::Assignment::new(
                    self.dest_ident.access_field("Ptr"),
                    self.temp_ptr_indent
                        .clone()
                        .cast(lower::CSharpType::new_struct("IntPtr")),
                )
                .stmt(),
            ),
        ]
    }
}

impl CsToFfiStableConversion for CsArrToSliceAbi {
    fn source_ident(&self) -> &lower::Ident {
        &self.source_ident
    }

    fn source_type(&self) -> &lower::CSharpType {
        &self.source_type
    }

    fn dest_ident(&self) -> &lower::Ident {
        &self.dest_ident
    }

    fn dest_type(&self) -> &lower::CSharpType {
        &self.dest_type
    }

    fn requires_unsafe(&self) -> bool {
        true
    }
}

/// Transforms a C# bool to a u8
pub struct BoolToUint {
    pub source_ident: lower::Ident,
    pub dest_ident: lower::Ident,
}

impl BodyFragment for BoolToUint {
    fn to_statements(&self) -> Vec<Box<dyn lower::BodyStatement>> {
        let ty = lower::CSharpType::Byte;
        let predicate = Box::new(self.source_ident.clone());
        let true_branch = Box::new(lower::Literal::Integer(1).cast(ty.clone()));
        let false_branch = Box::new(lower::Literal::Integer(0).cast(ty.clone()));

        vec![Box::new(
            lower::Assignment::new(
                self.dest_ident.clone().declare_with_ty(ty),
                lower::TernaryExpression {
                    predicate,
                    true_branch,
                    false_branch,
                },
            )
            .stmt(),
        )]
    }
}

impl CsToFfiStableConversion for BoolToUint {
    fn source_ident(&self) -> &lower::Ident {
        &self.source_ident
    }

    fn source_type(&self) -> &lower::CSharpType {
        &lower::CSharpType::Bool
    }

    fn dest_ident(&self) -> &lower::Ident {
        &self.dest_ident
    }

    fn dest_type(&self) -> &lower::CSharpType {
        &lower::CSharpType::Byte
    }

    fn requires_unsafe(&self) -> bool {
        false
    }
}

pub struct BindingMethod {
    pub source_descriptor: dotnet_bindgen_core::BindgenFunctionDescriptor,
    pub dll_name: String,
    pub args: Vec<Box<dyn CsToFfiStableConversion>>,
}

impl BindingMethod {
    pub fn thunk_method(&self) -> lower::Method {
        let args = self
            .args
            .iter()
            .map(|arg| lower::MethodArg {
                name: arg.source_ident().clone(),
                ty: arg.source_type().clone(),
            })
            .collect::<Vec<_>>();

        let mut body: Vec<Box<dyn lower::BodyStatement>> = Vec::new();

        // If any of the conversion fragments require unsafe, put an unsafe block first.
        if self.args.iter().any(|arg| arg.requires_unsafe()) {
            body.push(Box::new(lower::UnsafeBlock {}));
        }

        // Include the conversion fragments for each of the args
        for arg in &self.args {
            body.extend(arg.to_statements().drain(..));
        }

        // todo: handle returned values + conversions back to C# types
        let return_type = lower::CSharpType::Void;
        body.push(Box::new(
            lower::MethodCall {
                object: None,
                method: self.source_descriptor.thunk_name.clone().into(),
                args: self.args.iter().map(|a| a.dest_ident()).cloned().collect(),
            }
            .stmt(),
        ));

        lower::Method {
            attributes: Vec::new(),
            visibility: lower::Visibility::Public,
            is_static: true,
            is_extern: false,
            name: self.source_descriptor.real_name.to_camel_case().into(),
            return_type,
            args,
            body: Some(body),
        }
    }
}

/// A static class that contains a set of bindings to free functions, and their idiomatic
/// conversion wrappers.
pub struct MethodContainer {
    pub name: lower::Ident,
    pub methods: Vec<BindingMethod>,
}

pub struct BindingModule {
    pub namespace: lower::Ident,
    pub free_methods: Option<MethodContainer>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::representations::{level_0::ToTokens, level_1 as lower};

    fn fragment_test_helper(frag: impl BodyFragment, expected: &str) {
        let tokens = frag.to_statements().to_token_stream();
        let mut rendered = Vec::new();
        tokens
            .render(&mut rendered)
            .expect("TokenStream::render returned an error");
        let rendered =
            String::from_utf8(rendered).expect("TokenStream::render did not emit valid utf8");

        let rendered_tokens = rendered.split_whitespace().collect::<Vec<_>>();
        let expected_tokens = expected.split_whitespace().collect::<Vec<_>>();

        assert_eq!(rendered_tokens, expected_tokens);
    }

    #[test]
    fn test_cs_arr_to_slice_abi() {
        let mut id_gen = lower::IdentGenerator::new();
        let frag = CsArrToSliceAbi::new("source", lower::CSharpType::Int16, &mut id_gen);

        fragment_test_helper(
            frag,
            "SliceAbi _gen0 ;
            _gen0 . Len = ( ( UInt64 ) source . Length ) ;
            fixed ( Int16 * _gen1 = & source [ 0 ] )
            {
                _gen0 . Ptr = ( ( IntPtr ) _gen1 ) ;
            }",
        );
    }

    #[test]
    fn test_bool_to_ffi() {
        let frag = BoolToUint {
            source_ident: "source".into(),
            dest_ident: "dest".into(),
        };

        fragment_test_helper(
            frag,
            "Byte dest = source ? ( ( Byte ) 1 ) : ( ( Byte ) 0 ) ;",
        );
    }
}
