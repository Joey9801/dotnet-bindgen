use dotnet_bindgen::dotnet_bindgen;


#[dotnet_bindgen]
fn i32_return() -> i32 {
    10
}

#[dotnet_bindgen]
fn i8_arg(arg: i8) -> i32 {
    dbg!(arg);
    10
}

#[dotnet_bindgen]
fn void_return(arg: i32) {
    dbg!(arg);
}

#[dotnet_bindgen]
fn slice_arg(slice: &[i32]) {
    dbg!(slice);
}

#[dotnet_bindgen]
#[derive(Debug)]
struct SimpleStruct {
    field_1: i32,
    field_2: u64,
}

#[dotnet_bindgen]
fn struct_arg_val(arg: SimpleStruct) {
    dbg!(arg);
}