use dotnet_bindgen::dotnet_bindgen;

#[no_mangle]
#[dotnet_bindgen]
pub extern "C" fn demo_function(first_arg: i16, second_arg: i16) {
    println!("Hello from the demo function: arg1 = {}", first_arg);
}
