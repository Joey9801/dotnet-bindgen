//! Core types, methods, and constants to be shared between all components of the bindgen pipeline.
//!
//! This component is intended to be fairly minimal, to reduce the impact of having it included in client code.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum BindgenCoreErr {
    UnknownType,
}

/// Represents a type that is safe/well defined across the FFI boundary.
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FfiType {
    Int { width: u8, signed: bool },
    Void,
}

// TODO: Move this parser into macro-support as part of the rest of the parser.
impl FromStr for FfiType {
    type Err = BindgenCoreErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ty = match s {
            "i8" => FfiType::Int {
                width: 8,
                signed: true,
            },
            "i16" => FfiType::Int {
                width: 16,
                signed: true,
            },
            "i32" => FfiType::Int {
                width: 32,
                signed: true,
            },
            "i64" => FfiType::Int {
                width: 64,
                signed: true,
            },
            "u8" => FfiType::Int {
                width: 8,
                signed: false,
            },
            "u16" => FfiType::Int {
                width: 16,
                signed: false,
            },
            "u32" => FfiType::Int {
                width: 32,
                signed: false,
            },
            "u64" => FfiType::Int {
                width: 64,
                signed: false,
            },
            _ => return Err(BindgenCoreErr::UnknownType),
        };

        Ok(ty)
    }
}

/// Represents any type that may appear in the signature of functinos with generated bindings.
#[repr(C)]
#[derive(Debug, Clone)]
pub enum BoundType {
    /// A simple type that requires no marshalling
    FfiType(FfiType),
}

impl BoundType {
    pub fn is_ffi(&self) -> bool {
        match &self {
            BoundType::FfiType(_) => true,
            _ => false,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MethodArgument<'a> {
    pub name: MaybeOwnedString<'a>,
    pub ty: BoundType,
}

impl<'a> MethodArgument<'a> {
    pub fn requires_thunk(&self) -> bool {
        !self.ty.is_ffi()
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct BindgenFunction<'a> {
    pub name: MaybeOwnedString<'a>,
    pub args: MaybeOwnedArr<'a, MethodArgument<'a>>,
    pub return_ty: BoundType,
}

impl<'a> BindgenFunction<'a> {
    pub fn requires_thunk(&self) -> bool {
        !self.return_ty.is_ffi() || self.args.iter().any(|a| a.requires_thunk())
    }
}

// Important to be <= 8 characters
// PE/PEI binary section name lengths are limited to 8 characters (not including any null
// terminator).
pub const BINDGEN_DATA_SECTION_NAME: &'static str = ".bgendat";

// Important to be <= 8 characters
// PE/PEI binary section name lengths are limited to 8 characters (not including any null
// terminator).
pub const BINDGEN_SECTION_NAME: &'static str = ".bindgen";
