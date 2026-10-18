---
category: Rust
---

# Check formatter availability separately from the pinned Cargo toolchain

The pinned1.99.0 toolchain has Cargo/rustc but no rustfmt component; the explicit
formatter invocation failed before source changes. The existing default stable
formatter is available. Check its actual version and format the new inventory
module/tests with that installed binary, while retaining1.99.0 for compilation
and tests. Do not treat formatter availability as a compiler/test result or
download another toolchain merely to format reversible local source edits.

CLOC41 repeated the explicit1.99.0 formatter attempt before checking this
existing lesson; it again failed without modifying source. The installed
stable formatter1.9.0 then formatted only the new CLI file. Read this shard
before invoking formatting, and keep formatter and execution toolchains
separately recorded rather than assuming Cargo installation includes rustfmt.
