use dotnet_bindgen::dotnet_bindgen;

#[dotnet_bindgen]
fn demo_function(first_arg: &[i32], second_arg: u64) -> u16 {
    dbg!(first_arg);
    dbg!(second_arg);
    first_arg.len() as u16 + second_arg as u16
}

#[dotnet_bindgen]
fn another_func(a: u8, b: u16) -> i8 {
    dbg!(a);
    dbg!(b);
    10
}