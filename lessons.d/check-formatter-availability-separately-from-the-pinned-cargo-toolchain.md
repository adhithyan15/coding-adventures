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
