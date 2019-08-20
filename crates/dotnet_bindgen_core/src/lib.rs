#[repr(C)]
#[derive(Debug)]
pub enum FfiType {
    Int { width: u8, signed: bool },
    Bool,
    String,
    // Array { element_type: &'a FfiType, length: usize },
    Void,
}


#[repr(C)]
#[derive(Debug)]
pub struct MethodArgument<'a> {
    pub name_bytes: &'a [u8],
    pub ffi_type: FfiType,
}

impl<'a> MethodArgument<'a> {
    pub fn name(&self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(self.name_bytes) }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct BindGenFunction<'a> {
    pub name_bytes: &'a [u8],
    pub args: &'a [MethodArgument<'a>],
    pub return_type: FfiType,
}

impl<'a> BindGenFunction<'a> {
    pub fn name(&self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(self.name_bytes) }
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
