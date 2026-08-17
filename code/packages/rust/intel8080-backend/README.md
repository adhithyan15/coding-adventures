# intel8080-backend

Intel 8080 backend for `jit-core` / `aot-core`. Third lane of the
9-architecture expansion. Minimal-viable port — covers `const_*` +
`ret_*` only, mirroring `intel8008-backend`'s shape (the 8080 is the
8008's direct architectural successor, sharing the same
`MVI A, n` / `HLT` return convention).
