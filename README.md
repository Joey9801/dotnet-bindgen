<div align="center">
  <h1><code>dotnet-bindgen</code></h1>
  <p>
    <strong>Generate dotnet bindings to Rust code</strong>
  </p>
</div>

## Example

```rust
use dotnet_bindgen::dotnet_bindgen

// Export a simple top-level function from Rust.
#[no_mangle]
#[dotnet_bindgen]
pub extern "C" fn demo_function(first_arg: i16, second_arg: i16) -> i16 {
    println!("Hello from the demo function: arg1 = {}", first_arg);
    second_arg * 2
}
```

Generates the following C# code:

```c#
using System;
using System.Runtime.InteropServices;

namespace TestLib
{
    public static class Bindings
    {
        [DllImport("libtest_lib.so", EntryPoint = "demo_function")]
        public static extern Int16 DemoFunction(Int16 firstArg, Int16 secondArg);
    }
}
```
