use dotnet_bindgen::dotnet_bindgen;

#[no_mangle]
#[dotnet_bindgen]
pub extern "C" fn func_1(arg1: i16) {
    println!("Hello from func_1, arg1 = {}", arg1);
}
