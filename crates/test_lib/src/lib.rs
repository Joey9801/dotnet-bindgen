use dotnet_bindgen_core::*;

#[link_section = ".bgendat"]
#[no_mangle]
pub static __func1_bind_def_name: [u8; 6] = *b"func_1";

#[link_section = ".bgendat"]
#[no_mangle]
pub static __func1_bind_def_args_1_name: [u8; 4] = *b"arg1";

#[link_section = ".bgendat"]
#[no_mangle]
pub static __func1_bind_def_args_arr: [MethodArgument; 1] = [
    MethodArgument {
        name_bytes: &__func1_bind_def_args_1_name,
        ffi_type: FfiType::Int { signed: true, width: 16 },
    },
];

#[link_section = ".bindgen"]
#[no_mangle]
pub static __func_1_bind_def: BindGenFunction = BindGenFunction {
    name_bytes: &__func1_bind_def_name,
    return_type: FfiType::Void,
    args: &__func1_bind_def_args_arr,
};


#[no_mangle]
pub extern fn func_1(arg1: i16) {
    println!("Hello from func1, arg1 = {}", arg1);
}
