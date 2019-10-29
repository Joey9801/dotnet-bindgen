use std::str::FromStr;

/// Subset of the available dotnet RuntimeIds. Roughly corresponds to a rust target triple.
#[derive(Clone, Copy, Debug)]
pub enum NativePlatform {
    WinX64,
    LinuxX64,
    LinuxMuslX64,
    OsxX64,
}

impl NativePlatform {
    /// The string representing this RID that dotnet understands
    pub fn to_dotnet_rid_string(&self) -> &'static str {
        match self {
            NativePlatform::WinX64 => "win-x64",
            NativePlatform::LinuxX64 => "linux-x64",
            NativePlatform::LinuxMuslX64 => "linux-musl-x64",
            NativePlatform::OsxX64 => "osx-x64",
        }
    }

    pub fn host_platform() -> Self {
        // TODO
        NativePlatform::LinuxX64
    }
}

impl FromStr for NativePlatform {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "win-x64" => Ok(NativePlatform::WinX64),
            "linux-x64" => Ok(NativePlatform::LinuxX64),
            "linux-musl-x64" => Ok(NativePlatform::LinuxMuslX64),
            "osx-x64" => Ok(NativePlatform::OsxX64),
            _ => Err("Unrecognized platform")
        }
    }
}