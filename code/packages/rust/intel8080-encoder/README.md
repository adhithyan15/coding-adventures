# intel8080-encoder

Pure-Rust Intel 8080 instruction encoder. Re-exports the `encode_*`
helpers and register/opcode constants `intel8080-backend` needs from
`intel8080-simulator::encoding` / `::opcodes`, so the backend depends on a
small, IR-agnostic surface without pulling in the full simulator crate's
decode/execute internals. Third lane of the 9-architecture expansion
(mirror of `mips-r2000-encoder` / `intel8008-encoder`).
