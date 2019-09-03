use std::fs::File;
use std::io::Read;

use goblin::elf::section_header::SectionHeader;
use goblin::elf::Elf;
use goblin::Object;

use dotnet_bindgen_core::*;

#[derive(Debug)]
struct LoadedSection {
    header: SectionHeader,
    data: Vec<u8>,
}

impl LoadedSection {
    fn new(elf: &Elf, buffer: &[u8], section_name: &str) -> Result<Self, ()> {
        let header = match elf
            .section_headers
            .iter()
            .filter(|hdr| match elf.shdr_strtab.get(hdr.sh_name) {
                Some(Ok(name)) if name == section_name => true,
                _ => false,
            })
            .next()
        {
            Some(header) => header.clone(),
            None => return Err(()),
        };

        let data = match buffer.get(header.file_range()) {
            Some(slice) => slice.to_vec(),
            None => return Err(()),
        };

        Ok(Self { header, data })
    }
}

#[derive(Debug)]
pub struct BindgenData {
    pub source_file: std::path::PathBuf,

    loaded_func: BindgenFunction
}

impl BindgenData {
    fn load_elf(elf: &Elf, buffer: &[u8], source_file: std::path::PathBuf) -> Result<Self, ()> {
        let data_section = LoadedSection::new(elf, buffer, BINDGEN_DATA_SECTION_NAME)?;
        let metadata_string = std::str::from_utf8(&data_section.data)
            .expect("Expected valid UTF-8 data in bindgen data section");

        let loaded_func = serde_json::from_str(metadata_string)
            .expect("Expected valid json data in bindgen data section");

        Ok(Self { source_file, loaded_func })
    }

    pub fn load(file: std::path::PathBuf) -> Result<Self, ()> {
        let mut fd = File::open(&file).unwrap();

        let mut buffer = Vec::new();
        fd.read_to_end(&mut buffer).unwrap();

        let data = match Object::parse(&buffer).unwrap() {
            Object::Elf(elf) => Self::load_elf(&elf, &buffer, file).expect("Failed to load elf"),
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

    pub fn get_function(&self) -> &BindgenFunction {
        &self.loaded_func
    }
}
