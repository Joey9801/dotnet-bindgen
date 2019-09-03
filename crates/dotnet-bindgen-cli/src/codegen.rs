use std::fs;
use std::path::{Path, PathBuf};

use heck::CamelCase;

use crate::ast;
use crate::data::BindgenData;

fn create_csproj(data: &BindgenData, project_dir: &Path) -> Result<(), std::io::Error> {
    // Generate the proj filename
    // ./foo/bar/libtest_lib.so -> TestLibBindings.csproj

    let source_ext = data.source_file.extension().and_then(|e| e.to_str());

    let source_stem = data
        .source_file
        .file_stem()
        .expect("Source file path doesn't have a filename")
        .to_str()
        .expect("Source filename isn't valid unicode");

    // Silly .so file naming convention has an extra "lib" prefix on the filename
    // Strip it off if in that case
    let base_name = if source_stem.starts_with("lib") && source_ext == Some("so") {
        source_stem.chars().skip(3).collect::<String>()
    } else {
        source_stem.into()
    }
    .to_camel_case();

    let csproj_filename = project_dir.join(format!("{}Bindings.csproj", base_name));

    let contents = r#"<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>netstandard2.0</TargetFramework>
    </PropertyGroup>
</Project>
"#;

    std::fs::create_dir_all(project_dir)?;
    std::fs::write(csproj_filename, contents)?;

    Ok(())
}

pub fn create_project(data: crate::data::BindgenData, project_dir: PathBuf) -> Result<(), ()> {
    create_csproj(&data, &project_dir).unwrap();

    let binary_name = data
        .source_file
        .file_name()
        .expect("Expect a filename in the path".into())
        .to_str()
        .expect("Expec the filename to be valid unicode");

    // For the first pass, stick everything in one file
    let root = ast::Root {
        file_comment: Some(ast::BlockComment {
            text: vec!["This is a generated file, do not modify by hand.".into()],
        }),
        using_statements: vec![
            ast::UsingStatement {
                path: "System".into(),
            },
            ast::UsingStatement {
                path: "System.Runtime.InteropServices".into(),
            },
        ],
        children: vec![Box::new(ast::Namespace {
            name: "Test.Namespace".into(),
            children: vec![Box::new(ast::Class {
                name: "Imports".into(),
                is_static: true,
                methods: vec![ast::ImportedMethod {
                    binary_name: binary_name.into(),
                    func_data: data.get_function().clone(),
                }],
            })],
        })],
    };

    fs::create_dir_all(&project_dir).unwrap();

    let file_path = project_dir.join("Bindings.cs");
    let mut file = std::fs::File::create(&file_path).expect(&format!(
        "Can't open {} for writing",
        file_path.to_str().unwrap()
    ));
    root.render(&mut file).unwrap();

    Ok(())
}
