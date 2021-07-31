//! This module handles the generation of a complete bindings csproj file

use crate::path_ext::BinBaseName;
use crate::platform::NativePlatform;
use std::path::PathBuf;

/// A single native binary which should be linked into the generated project
#[derive(Clone, Debug)]
pub struct NativeBinary {
    platform: NativePlatform,
    filepath: PathBuf,
}

impl NativeBinary {
    pub fn new(platform: NativePlatform, filepath: PathBuf) -> Self {
        Self { platform, filepath }
    }

    fn filename(&self) -> String {
        self.filepath
            .file_name()
            .expect("Expect a native binary path to have a filename")
            .to_str()
            .expect("Expect a native binary filename to be valid unicode".into())
            .to_owned()
    }

    fn render_proj_xml(&self) -> String {
        let filepath = self
            .filepath
            .to_str()
            .expect("Expect native binary path to be valid unicode");
        let filename = self.filename();

        format!(
            r#"
        <Content Include="{}" Link="{}" PackagePath="runtimes/{}/native/{}">
            <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
        </Content>
"#,
            filepath,
            filename,
            self.platform.to_dotnet_rid_string(),
            filename
        )
    }
}

/// A set of different builds of the same native binary for various platforms.
#[derive(Clone, Debug)]
pub struct NativeBinarySet {
    base_name: String,
    binaries: Vec<NativeBinary>,
}

impl NativeBinarySet {
    pub fn new<I: IntoIterator<Item = NativeBinary>>(binaries: I) -> Self {
        let binaries: Vec<_> = binaries.into_iter().collect();
        let base_name = match &binaries.first() {
            Some(b) => b.filepath.bin_base_name(),
            None => panic!("Attempting to construct a NativeBinarySet from zero binaries"),
        };

        assert!(
            binaries
                .iter()
                .all(|b| b.filepath.bin_base_name() == base_name),
            "Constructing a NativeBinarySet for binaries with different base names"
        );

        Self {
            base_name,
            binaries,
        }
    }

    fn render_proj_xml(&self) -> String {
        let mut xml_str = format!(
            r#"    <ItemGroup Label = "{} native libs">"#,
            self.base_name
        );

        for bin in &self.binaries {
            xml_str.push_str(&bin.render_proj_xml());
        }

        xml_str.push_str("    </ItemGroup>");

        xml_str
    }
}

pub struct ProjFile {
    pub target_framework: String,
    pub allow_unsafe: bool,
    pub binary_set: NativeBinarySet,
}

impl ProjFile {
    pub fn render_proj_xml(&self) -> String {
        format!(
            r#"<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>{}</TargetFramework>
        <AllowUnsafeBlocks>{}</AllowUnsafeBlocks>
    </PropertyGroup>
{}
</Project>
"#,
            self.target_framework,
            if self.allow_unsafe { "true" } else { "false" },
            self.binary_set.render_proj_xml()
        )
    }
}
