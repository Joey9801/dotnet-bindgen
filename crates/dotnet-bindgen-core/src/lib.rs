//! Core types, methods, and constants to be shared between all components of the bindgen pipeline.
//!
//! This component is intended to be fairly minimal, to reduce the impact of having it included in client code.

use std::str::FromStr;

/// Marker trait for types that are trivially stable ABI types
trait BindgenAbiStable {}

impl BindgenAbiStable for i8 {}
impl BindgenAbiStable for i16 {}
impl BindgenAbiStable for i32 {}
impl BindgenAbiStable for i64 {}
impl BindgenAbiStable for u8 {}
impl BindgenAbiStable for u16 {}
impl BindgenAbiStable for u32 {}
impl BindgenAbiStable for u64 {}

/// Defines how to translate a non-trivial type to/from a stable ABI type
trait BindgenAbiTranslatable {
    type AbiType: BindgenAbiStable;

    fn from_abi_type(abi_value: Self::AbiType) -> Self;
    fn to_abi_type(&self) -> Self::AbiType;
}

#[repr(C)]
struct SliceAbi<T: BindgenAbiStable> {
    ptr: *const T,
    len: u64,
}

impl<T: BindgenAbiStable> BindgenAbiStable for SliceAbi<T> {}

impl<T: BindgenAbiStable> BindgenAbiTranslatable for &[T] {
    type AbiType = SliceAbi<T>;

    fn from_abi_type(abi_value: Self::AbiType) -> Self {
        unsafe { std::slice::from_raw_parts(abi_value.ptr, abi_value.len as usize) }
    }

    fn to_abi_type(&self) -> Self::AbiType {
        let ptr = self.as_ptr();
        let len = self.len() as u64;
        Self::AbiType { ptr, len }
    }
}

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
#[derive(Debug, Clone)]
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
#[derive(Clone)]
#[repr(C)]
pub struct MaybeOwnedString<'a> {
    pub bytes: MaybeOwnedArr<'a, u8>,
}

impl<'a> Display for MaybeOwnedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let parsed_str =
            std::str::from_utf8(&self.bytes).expect("MaybeOwnedString contains non-utf8 data");
        write!(f, "{}", parsed_str)
    }
}

impl<'a> Debug for MaybeOwnedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let parsed_str =
            std::str::from_utf8(&self.bytes).expect("MaybeOwnedString contains non-utf8 data");

        match self.bytes {
            MaybeOwnedArr::Owned(_) => write!(f, "MaybeOwnedString::Owned({:?})", parsed_str),
            MaybeOwnedArr::Ref(_) => write!(f, "MaybeOwnedString::Ref({:?})", parsed_str),
        }
    }
}

impl<'a> FromStr for MaybeOwnedString<'a> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = MaybeOwnedArr::Owned(s.as_bytes().to_vec());
        Ok(MaybeOwnedString { bytes })
    }
}

impl<'a> Deref for MaybeOwnedString<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.bytes).expect("MaybeOwnedString contains non-utf8 data")
    }
}

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

/// Represents any type that may appear in the signature of functions with generated bindings.
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
