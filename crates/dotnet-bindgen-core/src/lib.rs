//! Core types, methods, and constants to be shared between all components of the bindgen pipeline.
//!
//! This component is intended to be fairly minimal, to reduce the impact of having it included in client code.

/// Marker trait for types that are trivially stable ABI types
pub trait FfiStable {}

macro_rules! trivially_ffi_stable {
    ($($ty:ident),*) => { $( impl FfiStable for $ty {})* }
}

trivially_ffi_stable!(i8, i16, i32, i64, u8, u16, u32, u64);

// All reference types and pointer types to FfiStable types are also FfiStable
impl<'a, T: FfiStable> FfiStable for &'a T {}
impl<'a, T: FfiStable> FfiStable for &'a mut T {}
impl<T: FfiStable> FfiStable for *const T {}
impl<T: FfiStable> FfiStable for *mut T {}

/// Defines how to translate a non-trivial type to/from a stable ABI type
pub trait BindgenAbiConvert {
    type AbiType: FfiStable;

    fn from_abi_type(abi_value: Self::AbiType) -> Self;
    fn to_abi_type(self) -> Self::AbiType;
}

/// Types which are already FfiStable need no marshalling across the boundary
///
/// Any self-respecting optimiser will realise that these methods do nothing, and
/// completely optimise them out of any release build.
impl<T: FfiStable> BindgenAbiConvert for T {
    type AbiType = T;

    fn from_abi_type(abi_value: Self::AbiType) -> Self {
        abi_value
    }

    fn to_abi_type(self) -> Self::AbiType {
        self
    }
}

/// FfiStable representation of a slice type
///
/// This representation is written to look very similar to the actual underlying
/// representation of a slice type, such that the conversion functions are likely
/// to optimise away to nothing. This has been confirmed on rustc 1.37.0 in release
/// mode on an intel 2500k cpu.
#[repr(C)]
pub struct SliceAbi<T: FfiStable> {
    ptr: *const T,
    len: u64,
}

impl<T: FfiStable> FfiStable for SliceAbi<T> {}

impl<T: FfiStable> BindgenAbiConvert for &[T] {
    type AbiType = SliceAbi<T>;

    fn from_abi_type(abi_value: Self::AbiType) -> Self {
        unsafe { std::slice::from_raw_parts(abi_value.ptr, abi_value.len as usize) }
    }

    fn to_abi_type(self) -> Self::AbiType {
        let ptr = self.as_ptr();
        let len = self.len() as u64;
        Self::AbiType { ptr, len }
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindgenTypeDescriptor {
    Void,
    Int {
        width: u8,
        signed: bool,
    },
    Slice {
        elem_type: Box<BindgenTypeDescriptor>,
    },
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindgenFunctionArgumentDescriptor {
    pub name: String,
    pub ty: BindgenTypeDescriptor,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindgenFunctionDescriptor {
    /// The original name of the function that the #[dotnet_bindgen] attribute was placed on
    pub real_name: String,

    /// The no_mangle'd name of the generated thunk
    pub thunk_name: String,

    pub arguments: Vec<BindgenFunctionArgumentDescriptor>,
    pub return_ty: BindgenTypeDescriptor,
}

/// Trait to inform the generator of resolved types
///
/// At macro invocation time, all types are still just bags of tokens. There
/// isn't enough information or context to full resolve any types. Instead,
/// all types which are safe to pass across the ffi boundary should implement
/// this trait, such that the generator can invoke the resolved describe method
/// to find out what the type eventually became.
pub trait BindgenTypeDescribe {
    fn describe() -> BindgenTypeDescriptor;
}

macro_rules! simple_describe {
    ($ty:ident => $description:expr) => {
        impl BindgenTypeDescribe for $ty {
            fn describe() -> BindgenTypeDescriptor {
                use BindgenTypeDescriptor::*;
                $description
            }
        }
    };
    [$($ty:ident => $description:expr),*] => {
        $(simple_describe!($ty => $description);)*
    };
    [$($ty:ident => $description:expr,)*] => {
        $(simple_describe!($ty => $description);)*
    };
}

simple_describe![
    i8  => Int { width: 8,  signed: true  },
    i16 => Int { width: 16, signed: true  },
    i32 => Int { width: 32, signed: true  },
    i64 => Int { width: 64, signed: true  },
    u8  => Int { width: 8,  signed: false },
    u16 => Int { width: 16, signed: false },
    u32 => Int { width: 32, signed: false },
    u64 => Int { width: 64, signed: false },
];

impl<'a, T: FfiStable + BindgenTypeDescribe> BindgenTypeDescribe for &'a [T] {
    fn describe() -> BindgenTypeDescriptor {
        let elem_type = Box::new(<T as BindgenTypeDescribe>::describe());
        BindgenTypeDescriptor::Slice { elem_type }
    }
}

/// The generator discovers descriptors by scanning the binary for symbols that start with this prefix.
pub const BINDGEN_DESCRIBE_PREFIX: &'static str = "__bindgen_describe";
