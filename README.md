<div align="center">
  <h1><code>dotnet-bindgen</code></h1>
  <p>
    <strong>Generate dotnet bindings to Rust code</strong>
  </p>
</div>

dotnet-bindgen aims to generate idiomatic C# wrappers around idiomatic Rust methods.

dotnet-bindgen works by injecting a small amount of metadata into your Rust
binary at build time. A separate tool then extracts that metadata, and
generates C# glue code.

The tool outputs a complete project which is ready to be packed into a NuGet
package with `dotnet pack`. The resultant NuGet package bundles the native
Rust binaries.


## Quick start

To generate a dotnet bindings package:
  - Ensure your crate has a dependency on`"dotnet-bindgen"`
  - Ensure your crate is being built as a `cdylib`
  - Add the `#[dotnet_bindgen]` attribute to each item you want to generate bindings for
  - Build your crate as normal
  - Run the `"dotnet-bindgen-cli"` tool, pointing at the cdylib file you just built (eg `./target/debug/libyour_crate.so`)
  - Run `dotnet pack` on the generated bindings project.


## Example

```
joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> tree
.
├── Cargo.toml
└── src
    └── lib.rs

1 directory, 2 files

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> cat Cargo.toml 
[package]
name = "dotnet-bindgen-demo"
version = "0.1.0"
authors = ["Joe Roberts <joe@jwjr.co.uk>"]
edition = "2018"

[lib]
crate-type = ["cdylib"]

[dependencies]
dotnet-bindgen = { git = "https://github.com/Joey9801/dotnet-bindgen", branch = "master" }

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> cat src/lib.rs 
use dotnet_bindgen::dotnet_bindgen;

#[dotnet_bindgen]
fn sum_numbers(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> cargo build
<snip>

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> cargo install dotnet-bindgen-cli
<snip>

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> dotnet-bindgen-cli --bin ./target/debug/libdotnet_bindgen_demo.so  --source-output-dir ./bindings

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master) [1]> tree -I target
.
├── bindings
│   ├── DotnetBindgenDemoBindings.cs
│   └── DotnetBindgenDemoBindings.csproj
├── Cargo.lock
├── Cargo.toml
└── src
    └── lib.rs

2 directories, 5 files

joe@Mangosteen ~/D/d/r/dotnet-bindgen-demo (master)> cd bindings

joe@Mangosteen ~/D/d/r/d/bindings (master)> cat DotnetBindgenDemoBindings.csproj 
<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
        <TargetFramework>netstandard2.0</TargetFramework>
        <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    </PropertyGroup>
    <ItemGroup Label = "dotnet_bindgen_demo native libs">
        <Content Include="/home/joe/Documents/dev/rust/dotnet-bindgen-demo/target/debug/libdotnet_bindgen_demo.so" Link="libdotnet_bindgen_demo.so" PackagePath="runtimes/linux-x64/native/libdotnet_bindgen_demo.so">
            <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
        </Content>
    </ItemGroup>
</Project>

joe@Mangosteen ~/D/d/r/d/bindings (master)> cat DotnetBindgenDemoBindings.cs 
/*
 * This is a generated file, do not modify by hand.
 */

using System;
using System.Runtime.InteropServices;

namespace DotnetBindgenDemo
{
    [StructLayout(LayoutKind.Sequential)]
    public struct SliceAbi
    {
        public IntPtr Ptr;
        public UInt64 Len;
    }

    public static class TopLevelFunctinos
    {
        [DllImport("dotnet_bindgen_demo", EntryPoint = "__bindgen_thunk_sum_numbers")]
        private static extern Int32 __bindgen_thunk_sum_numbers(SliceAbi numbers);

        public static Int32 SumNumbers(Int32[] numbers)
        {
            SliceAbi _gen0;
            (_gen0).Len = (UInt64)((numbers).Length);
            unsafe
            {
                fixed (Int32* _gen1 = &((numbers)[0]))
                {
                    (_gen0).Ptr = (IntPtr)(_gen1);
                    return __bindgen_thunk_sum_numbers(_gen0);
                }
            }
        }
    }
}

joe@Mangosteen ~/D/d/r/d/bindings (master)> dotnet pack
Microsoft (R) Build Engine version 16.0.450+ga8dc7f1d34 for .NET Core
Copyright (C) Microsoft Corporation. All rights reserved.

  Restore completed in 376.16 ms for /home/joe/Documents/dev/rust/dotnet-bindgen-demo/bindings/DotnetBindgenDemoBindings.csproj.
  DotnetBindgenDemoBindings -> /home/joe/Documents/dev/rust/dotnet-bindgen-demo/bindings/bin/Debug/netstandard2.0/DotnetBindgenDemoBindings.dll
  Successfully created package '/home/joe/Documents/dev/rust/dotnet-bindgen-demo/bindings/bin/Debug/DotnetBindgenDemoBindings.1.0.0.nupkg'.

joe@Mangosteen ~/D/d/r/d/bindings (master)> unzip -l bin/Debug/DotnetBindgenDemoBindings.1.0.0.nupkg 
Archive:  bin/Debug/DotnetBindgenDemoBindings.1.0.0.nupkg
  Length      Date    Time    Name
---------  ---------- -----   ----
      515  2019-10-29 23:20   _rels/.rels
      522  2019-10-29 23:20   DotnetBindgenDemoBindings.nuspec
     4608  2019-10-29 23:20   lib/netstandard2.0/DotnetBindgenDemoBindings.dll
  2583000  2019-10-29 23:13   runtimes/linux-x64/native/libdotnet_bindgen_demo.so
      520  2019-10-29 23:20   [Content_Types].xml
      637  2019-10-29 23:20   package/services/metadata/core-properties/69408ef0c00a4a64ad26c40cdd42ca11.psmdcp
---------                     -------
  2589802                     6 files
```