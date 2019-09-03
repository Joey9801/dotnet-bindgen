//! Core types, methods, and constants to be shared between all components of the bindgen pipeline.
//!
//! This component is intended to be fairly minimal, to reduce the impact of having it included in client code.

use std::str::FromStr;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum BindgenCoreErr {
    UnknownType,
}

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FfiType {
    Int { width: u8, signed: bool },
    Void,
}

impl FromStr for FfiType {
    type Err = BindgenCoreErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ty = match s {
            "i8" => FfiType::Int { width: 8, signed: true },
            "i16" => FfiType::Int { width: 16, signed: true },
            "i32" => FfiType::Int { width: 32, signed: true },
            "i64" => FfiType::Int { width: 64, signed: true },
            "u8" => FfiType::Int { width: 8, signed: false },
            "u16" => FfiType::Int { width: 16, signed: false },
            "u32" => FfiType::Int { width: 32, signed: false },
            "u64" => FfiType::Int { width: 64, signed: false },
            _ => return Err(BindgenCoreErr::UnknownType),
        };

        Ok(ty)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodArgument {
    pub name: String,
    pub ffi_type: FfiType,
}

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindgenFunction {
    pub name: String,
    pub args: Vec<MethodArgument>,
    pub return_type: FfiType,
}

// Important to be <= 8 characters
// PE/PEI binary section name lengths are limited to 8 characters (not including any null
// terminator).
pub const BINDGEN_DATA_SECTION_NAME: &'static str = ".bgendat";

// Important to be <= 8 characters
// PE/PEI binary section name lengths are limited to 8 characters (not including any null
// terminator).
pub const BINDGEN_SECTION_NAME: &'static str = ".bindgen";
