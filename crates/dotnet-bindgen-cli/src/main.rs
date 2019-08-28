use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::exit;

use goblin::elf::section_header::SectionHeader;
use goblin::elf::Elf;
use goblin::Object;

use dotnet_bindgen_core::*;

mod ast;

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
struct BindgenData {
    data_section: LoadedSection,
    bindgen_section: LoadedSection,
}

impl BindgenData {
    /// Maps a virtual memory address from the loaded file to address in a loaded section.
    unsafe fn map_addr(&self, vm_addr: *const u8) -> Option<*const u8> {
        let vm_addr = vm_addr as usize;

        if self.bindgen_section.header.vm_range().contains(&vm_addr) {
            let offset = vm_addr - self.bindgen_section.header.vm_range().start;
            Some(self.bindgen_section.data.as_ptr().offset(offset as isize))
        } else if self.data_section.header.vm_range().contains(&vm_addr) {
            let offset = vm_addr - self.data_section.header.vm_range().start;
            Some(self.data_section.data.as_ptr().offset(offset as isize))
        } else {
            None
        }
    }

    /// Maps a virtual memory address from the loaded file to address in a loaded section.
    unsafe fn map_mut_addr(&mut self, vm_addr: *const u8) -> Option<*mut u8> {
        let vm_addr = vm_addr as usize;

        if self.bindgen_section.header.vm_range().contains(&vm_addr) {
            let offset = vm_addr - self.bindgen_section.header.vm_range().start;
            Some(
                self.bindgen_section
                    .data
                    .as_mut_ptr()
                    .offset(offset as isize),
            )
        } else if self.data_section.header.vm_range().contains(&vm_addr) {
            let offset = vm_addr - self.data_section.header.vm_range().start;
            Some(self.data_section.data.as_mut_ptr().offset(offset as isize))
        } else {
            None
        }
    }

    unsafe fn perform_elf_relocs(&mut self, elf: &Elf) -> Result<(), ()> {
        for reloc in elf.dynrelas.iter().chain(elf.dynrels.iter()) {
            match self.map_mut_addr(reloc.r_offset as *const u8) {
                Some(reloc_addr) => {
                    let sym = match elf.dynsyms.get(reloc.r_sym) {
                        Some(sym) => sym,
                        None => {
                            println!("Dynamic relocation refers to symbol not in dynsyms");
                            return Err(());
                        }
                    };

                    let sym_vm_ptr = match reloc.r_addend {
                        Some(addend) => (sym.st_value as i64 + addend) as u64,
                        None => sym.st_value,
                    } as *const u8;

                    match self.map_addr(sym_vm_ptr) {
                        Some(sym_addr) => *(reloc_addr as *mut *const u8) = sym_addr,
                        None => {
                            println!("self section contains a pointer to an unself section");
                            return Err(());
                        }
                    }
                }
                None => (),
            };
        }

        Ok(())
    }

    pub fn load_elf(elf: &Elf, buffer: &[u8]) -> Result<Self, ()> {
        let data_section = LoadedSection::new(elf, buffer, BINDGEN_DATA_SECTION_NAME)?;
        let bindgen_section = LoadedSection::new(elf, buffer, BINDGEN_SECTION_NAME)?;

        let mut loaded = Self {
            data_section,
            bindgen_section,
        };

        unsafe { loaded.perform_elf_relocs(elf)? };

        Ok(loaded)
    }

    pub fn get_function<'a>(&'a self) -> &'a BindgenFunction<'a> {
        let first_ptr = self.bindgen_section.data.as_ptr() as *mut BindgenFunction<'a>;
        unsafe { &*first_ptr }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let path = match args.len() {
        n if n >= 2 => Path::new(&args[1]),
        _ => exit(0),
    };

    let filename = path.file_name()
        .expect("Expect a filename in the path".into())
        .to_str()
        .expect("Expec the filename to be valid unicode");

    let mut fd = File::open(path).unwrap();
    let mut buffer = Vec::new();
    fd.read_to_end(&mut buffer).unwrap();

    let data = match Object::parse(&buffer).unwrap() {
        Object::Elf(elf) => BindgenData::load_elf(&elf, &buffer).expect("Failed to load elf"),
        Object::Unknown(magic) => {
            println!("unknown magic: {:#x}", magic);
            exit(1);
        }
        _ => {
            println!("Unsupported binary type");
            exit(1);
        }
    };

    let root = ast::Root {
        children: vec![
            Box::new(ast::BlockComment {
                text: vec![
                    "This is a generated file, do not modify by hand.".into(),
                ]
            }),
            Box::new(ast::UsingStatement { path: "System.Runtime.InteropServices".into() }),
            Box::new(ast::Namespace {
                name: "Test.Namespace".into(),
                children: vec![
                    Box::new(ast::Class {
                        name: "Imports".into(),
                        is_static: true,
                        methods: vec![
                            ast::ImportedMethod {
                                binary_name: filename.into(),
                                func_data: data.get_function().clone(),
                            }
                        ]
                    })
                ]
            }),
        ]
    };
    let mut f = std::io::stdout();
    root.render(&mut f).unwrap();
}
