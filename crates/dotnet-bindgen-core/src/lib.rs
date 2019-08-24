//! Core types, methods, and constants to be shared between all components of the bindgen pipeline.
//!
//! This component is intended to be fairly minimal, to reduce the impact of having it included in client code.

use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

/// Abstraction over the two useful ways of representing arrays in bindgen data
///
/// # Motivation
///
/// When dumping data into the binary, we want tight control of where the bytes
/// live, but managing references is super tricky when generating these structs
///  -> Use the owned variant for easy generation of the data
///  -> Use the reference variant for serialising/parsing the structures into link sections
///
/// # Examples
///
/// ```
/// // The array [1, 2, 3] lives somewhere on the heap
/// // Can't initialise a static variable like this.
/// let a = MaybeOwnedArr::Owned(vec![1, 2, 3]);
/// ```
///
/// ```
/// // Statically data that will be written to a well defined binary section
/// #[link_section(".foo")]
/// pub static DATA: [i32; 3] = [1, 2, 3];
///
/// #[no_mangle]
/// pub static ARR: MaybeOwnedArr<'static, i32> = MaybeOwnedArr::Ref(&DATA);
/// ```
#[derive(Debug)]
#[repr(C)]
pub enum MaybeOwnedArr<'a, T> {
    Owned(Vec<T>),
    Ref(&'a [T]),
}

impl<'a, T> Deref for MaybeOwnedArr<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeOwnedArr::Ref(r) => r,
            MaybeOwnedArr::Owned(v) => &v[..],
        }
    }
}

/// Thin wrapper around MaybeOwnedArr for strings
#[repr(C)]
#[derive(Debug)]
pub struct MaybeOwnedString<'a> {
    bytes: MaybeOwnedArr<'a, u8>,
}

impl<'a> Display for MaybeOwnedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            std::str::from_utf8(&self.bytes).expect("MaybeOwnedString contains non-utf8 data")
        )
    }
}

impl<'a> FromStr for MaybeOwnedString<'a> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = MaybeOwnedArr::Owned(s.as_bytes().to_vec());
        Ok(MaybeOwnedString { bytes })
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum FfiType {
    Int { width: u8, signed: bool },
    Void,
}

#[repr(C)]
#[derive(Debug)]
pub struct MethodArgument<'a> {
    pub name: MaybeOwnedString<'a>,
    pub ffi_type: FfiType,
}

#[repr(C)]
#[derive(Debug)]
pub struct BindgenFunction<'a> {
    pub name: MaybeOwnedString<'a>,
    pub args: MaybeOwnedArr<'a, MethodArgument<'a>>,
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
