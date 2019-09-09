use std::fs::File;
use std::io::Read;

use goblin::elf::Elf;
use goblin::Object;

use dotnet_bindgen_core::*;

#[derive(Debug)]
pub struct BindgenData {
    pub source_file: std::path::PathBuf,
    pub descriptors: Vec<BindgenFunctionDescriptor>,
}

impl BindgenData {
    fn load_elf(elf: &Elf, source_file: std::path::PathBuf) -> Result<Self, ()> {
        let mut descriptors = Vec::new();
        let lib = libloading::Library::new(&source_file).unwrap();
        for sym in elf.dynsyms.iter() {
            let name = match elf.dynstrtab.get(sym.st_name) {
                Some(Ok(s)) => s,
                _ => continue,
            };

            if !name.starts_with(BINDGEN_DESCRIBE_PREFIX) {
                continue;
            }

            unsafe {
                let descriptor_func: libloading::Symbol<unsafe fn() -> BindgenFunctionDescriptor> =
                    lib.get(name.as_bytes()).unwrap();
                descriptors.push(descriptor_func());
            }
        }

        Ok(Self {
            source_file,
            descriptors,
        })
    }

    pub fn load(file: std::path::PathBuf) -> Result<Self, ()> {
        let mut fd = File::open(&file).unwrap();

        let mut buffer = Vec::new();
        fd.read_to_end(&mut buffer).unwrap();

        let data = match Object::parse(&buffer).unwrap() {
            Object::Elf(elf) => Self::load_elf(&elf, file).expect("Failed to load elf"),
            Object::Unknown(magic) => {
                println!("unknown magic: {:#x}", magic);
                return Err(());
            }
            _ => {
                println!("Unsupported binary type");
                return Err(());
            }
        };

        Ok(data)
    }
}
