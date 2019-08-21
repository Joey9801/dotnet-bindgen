use dotnet_bindgen_core::*;
use dotnet_bindgen_macro::dotnet_bindgen;

#[no_mangle]
#[dotnet_bindgen]
pub extern "C" fn func_2(arg1: i16) {
    println!("Hello from func1, arg1 = {}", arg1);
}
