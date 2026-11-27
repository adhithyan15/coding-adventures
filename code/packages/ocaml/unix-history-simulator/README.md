# OCaml Unix History Simulator

A reusable, in-memory OCaml port of the HOS00 First Edition Unix teaching
model. The library exposes filesystem, descriptor-table, and kernel
components. It has no command-line program and does not load repository
fixtures at runtime.

## Use

```ocaml
let filesystem = Unix_history.Filesystem.create_demo ()
let table = Unix_history.Descriptor_table.create ~capacity:4
let kernel = Unix_history.Kernel.create ~filesystem ~descriptors:table
let trace = Unix_history.Kernel.run_command kernel "cat /hello"
```

The `trace` contains output and ordered typed events. Its caller decides how to
display or serialize them. A negative descriptor capacity raises
`Invalid_argument`; zero capacity is valid and allocates no descriptors.

The package BUILD recipe runs component checks and replays the nine shared
traces with an external test-only adapter. The adapter and fixture JSON are
outside the package API.
