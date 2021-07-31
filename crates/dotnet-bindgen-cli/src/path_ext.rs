use std::path::Path;

pub trait BinBaseName {
    fn bin_base_name(&self) -> String;
}

impl BinBaseName for Path {
    fn bin_base_name(&self) -> String {
        let ext = self.extension().and_then(|e| e.to_str());
        let stem = self
            .file_stem()
            .expect("Expect a native binary path to have a filename".into())
            .to_str()
            .expect("Expect a native binary filename to be valid unicode".into());

        if stem.starts_with("lib") && ext == Some("so") {
            stem.chars().skip(3).collect::<String>()
        } else {
            stem.into()
        }
    }
}
