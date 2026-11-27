# Unix History Simulator (C#)

This is a reusable .NET 9 library for the HOS00 in-memory teaching model. It
provides `FileSystem`, `DescriptorTable`, `Kernel`, and a pure
`Shell.RunCommand` function. The package contains no executable entry point and
does not read host files or write host output. The demo file image is in
memory.

```csharp
using UnixHistorySimulator;

var files = FileSystem.DemoImage();
var descriptors = new DescriptorTable(4);
var kernel = new Kernel(files, descriptors);
var opened = kernel.Open("/hello");
var result = Shell.RunCommand("cat /hello");
```

Run the external build-only component checks and all nine canonical fixture
cases with `python3 code/specs/fixtures/historical-unix-v1/build_dotnet.py
csharp` from the repository root. That adapter is not shipped in this package.
The model is a semantic teaching aid, not an historical ABI, binary, or host
filesystem implementation.
