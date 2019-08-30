use std::path::Path;

use clap::{Arg, App};

mod ast;
mod codegen;
mod data;

use data::BindgenData;

fn main() {
    let matches = App::new("dotnet-bindgen-cli tool")
        .author("Joe Roberts")
        .about("Extract binding data from annotated binaries + generate dotnet bindings")
        .arg(Arg::with_name("bin")
            .required(true)
            .long("bin")
            .value_name("BIN")
            .help("The path to the binay to process")
            .takes_value(true)
        )
        .get_matches();


    // Safe to unwrap because --bin is a required arg
    let binary_path = Path::new(matches.value_of("bin").unwrap()).canonicalize().unwrap();
    let generated_poject_dir = binary_path.parent()
            .unwrap_or(Path::new("."))
            .join(Path::new("dotnet_bindgen"));

    let data = BindgenData::load(binary_path.clone()).unwrap();

    codegen::create_project(data, generated_poject_dir).unwrap();
}
