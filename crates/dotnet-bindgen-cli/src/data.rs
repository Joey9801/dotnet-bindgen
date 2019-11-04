use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use goblin::elf::Elf;
use goblin::Object;

use dotnet_bindgen_core::*;

#[derive(Clone, Debug)]
pub struct BindgenData {
    pub source_file: PathBuf,
    pub descriptors: Vec<BindgenExportDescriptor>,
}

impl BindgenData {
    fn load_elf(elf: &Elf, file_path: &Path) -> Result<Self, &'static str> {
        let mut descriptors = Vec::new();
        let lib = libloading::Library::new(file_path).unwrap();
        for sym in elf.dynsyms.iter() {
            let name = match elf.dynstrtab.get(sym.st_name) {
                Some(Ok(s)) => s,
                _ => continue,
            };

            if !name.starts_with(BINDGEN_DESCRIBE_PREFIX) {
                continue;
            }

            unsafe {
                let descriptor_func: libloading::Symbol<unsafe fn() -> BindgenExportDescriptor> =
                    lib.get(name.as_bytes()).unwrap();
                descriptors.push(descriptor_func());
            }
        }

        Ok(Self {
            source_file: file_path.to_owned(),
            descriptors,
        })
    }

    /// Sorts the descriptors in this binding data set, to simplify comparisons with other sets.
    fn sort_descriptors(&mut self) { 
        self.descriptors.sort_by_cached_key(|d| match d {
            BindgenExportDescriptor::Function(f) => f.real_name.clone(),
            BindgenExportDescriptor::Struct(s) => s.name.clone(),
        });
    }

    pub fn load(file_path: &Path) -> Result<Self, &'static str> {
        let mut fd = File::open(file_path).unwrap();

        let mut buffer = Vec::new();
        fd.read_to_end(&mut buffer).unwrap();

        let mut data = match Object::parse(&buffer).unwrap() {
            Object::Elf(elf) => Self::load_elf(&elf, file_path),
            Object::Unknown(magic) => {
                println!("unknown magic: {:#x}", magic);
                Err("unknown magic number")
            },
            _ => Err("Unsupported binary type"),
        }?;

        data.sort_descriptors();

        Ok(data)
    }
}
