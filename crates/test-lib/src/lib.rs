use dotnet_bindgen::dotnet_bindgen;

#[no_mangle]
#[dotnet_bindgen]
pub extern "C" fn demo_function(arg1: i16) {
    println!("Hello from the demo function: arg1 = {}", arg1);
}
