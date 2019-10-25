use std::path::{Path, PathBuf};

use clap::{App, Arg};
use heck::CamelCase;

mod ast;
mod platform;
mod csproj;
mod codegen;
mod data;
mod path_ext;

use data::BindgenData;
use path_ext::BinBaseName;

struct SourceBinarySpec {
    platform: platform::NativePlatform,
    bin_path: PathBuf,
    base_name: String,
    bindgen_data: data::BindgenData,
}

impl SourceBinarySpec {
    fn new(platform: platform::NativePlatform, bin_path: &Path) -> Result<Self, &'static str> {
        let bin_path = bin_path.to_owned();
        let base_name = bin_path.bin_base_name();
        let bindgen_data = BindgenData::load(&bin_path)?;

        Ok(Self {
            platform,
            bin_path,
            base_name,
            bindgen_data,
        })
    }
}


/// Takes any number of source binary specs, and generates a bindings project.
/// All binaries given must contain the same binding metadata, and target different platforms.
///
/// input_binaries:
///     The binaries that bindings should be generated for.
///     May include builds of the same library for different platforms.
///     May include different libraries.
///
/// source_output_dir:
///     The root directory to write the source code of the generated project to.
fn generate_bindings(
    input_binaries: Vec<SourceBinarySpec>,
    source_output_dir: &Path
) -> Result<(), &'static str> {
    let base_name;
    // Basic validation of the given source binaries.
    match input_binaries.first() {
        None => return Err("Must have at least one binary to generate bindings for"),
        Some(f) => {
            base_name = f.base_name.clone();

            if input_binaries.iter().any(|b| b.base_name != base_name) {
                return Err("The given source binaries have different base names")
            }

            if input_binaries.iter()
                .any(|b| b.bindgen_data.descriptors != f.bindgen_data.descriptors) {
                return Err("The given source binaries expose different descriptors")
            }
        }
    }

    // Ensure the output directory exists + is an empty directory
    if source_output_dir.exists() {
        if !source_output_dir.is_dir() {
            return Err("The given source-output-dir is not a directory")
        }
    } else {
        std::fs::create_dir_all(source_output_dir)
            .map_err(|_| "Failed to create source output directory")?;
    }

    if source_output_dir
        .read_dir()
        .map_err(|_| "Failed to open the source output directory")?
        .any(|_| true)
    {
        return Err("The given source-output-dir is not empty")
    }

    // Generate + write the project file
    let binary_set = csproj::NativeBinarySet::new(
        input_binaries.iter().map(|b| csproj::NativeBinary::new(
            b.platform,
            b.bin_path.to_owned(),
        ))
    );

    let proj = csproj::ProjFile {
        target_framework: "netstandard2.0".to_owned(),
        allow_unsafe: true,
        binary_set
    };

    let proj_filename = format!("{}Bindings.csproj", base_name.to_camel_case());
    let proj_filepath = source_output_dir.join(proj_filename);
    let proj_content = proj.render_proj_xml();

    std::fs::write(proj_filepath, proj_content)
        .map_err(|_| "Failed to write csproj file")?;

    // Generate binding source ast from one set of extracted data
    // Write out a bindings source file from that ast
    let bindings_filename = format!("{}Bindings.cs", base_name.to_camel_case());
    let bindings_filepath = source_output_dir.join(bindings_filename);
    let mut bindings_file = std::fs::File::create(&bindings_filepath).expect(&format!(
        "Can't open {} for writing",
        bindings_filepath.to_str().unwrap()
    ));
    let ast_root = codegen::form_ast_from_data(&input_binaries.first().unwrap().bindgen_data);
    ast_root.render(&mut bindings_file)
        .map_err(|_| "Failed to write bindings C# ast to file")?;

    Ok(())
}

fn main() -> Result<(), &'static str> {
    let matches = App::new("dotnet-bindgen-cli tool")
        .author("Joe Roberts")
        .about("Extract binding data from annotated binaries + generate dotnet bindings")
        .arg(Arg::with_name("source-output-dir")
            .required(true)
            .long("source-output-dir")
            .value_name("DIR")
            .help(r#"The directory the generated bindings are written to.
            NB: This directory must be empty!"#)
            .takes_value(true))
        .arg(Arg::with_name("bin")
            .required(true)
            .long("bin")
            .value_name("BIN")
            .help("The path to the binary to process")
            .takes_value(true))
        .get_matches();

    // Safe to unwrap because --bin is a required arg
    let binary_path = Path::new(matches.value_of("bin").unwrap())
        .canonicalize()
        .map_err(|_| "Failed to canonicalize binary path")?;

    let source_output_dir = Path::new(matches.value_of("source-output-dir").unwrap());

    // TODO: actually parse out some real specs from the command line args
    let source_binaries = vec![
        SourceBinarySpec::new(platform::NativePlatform::LinuxX64, &binary_path)?,
    ];

    generate_bindings(source_binaries, &source_output_dir)?;

    Ok(())
}
