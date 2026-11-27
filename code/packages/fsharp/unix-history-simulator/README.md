# Unix History Simulator (F#)

This is a reusable .NET 9 library for the HOS00 in-memory teaching model. It
provides `FileSystem`, `DescriptorTable`, `Kernel`, and the pure
`Shell.runCommand` function. The package contains no executable entry point
and does not read host files or write host output. The demo file image is in
memory.

```fsharp
open UnixHistorySimulator

let files = FileSystem.DemoImage()
let descriptors = DescriptorTable 4
let kernel = Kernel(files, descriptors)
let opened = kernel.Open "/hello"
let result = Shell.runCommand "cat /hello"
```

Run the external build-only component checks and all nine canonical fixture
cases with `python3 code/specs/fixtures/historical-unix-v1/build_dotnet.py
fsharp` from the repository root. That adapter is not shipped in this package.
The model is a semantic teaching aid, not an historical ABI, binary, or host
filesystem implementation.
