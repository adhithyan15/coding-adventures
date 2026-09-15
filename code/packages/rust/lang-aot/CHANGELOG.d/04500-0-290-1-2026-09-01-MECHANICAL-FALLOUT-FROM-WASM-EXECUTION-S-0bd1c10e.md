## 0.290.1 - 2026-09-01 (mechanical fallout from `wasm-execution`'s `GlobalStorage`, W35 third slice)

Test-only mechanical fix: `tests/lang_matrix.rs`'s `PrintHost` and
`tests/wasm_emit.rs`'s `PrintStrHost` (both hand-built `HostInterface`
test doubles that `resolve_global` unconditionally to `None`) needed
their return type updated from `Rc<RefCell<wasm_execution::WasmValue>>`
to `Rc<RefCell<wasm_execution::GlobalStorage>>` to keep compiling against
`wasm-execution`'s new `GlobalStorage` type (`code/specs/
W35-wasm-cross-instance-function-identity.md`'s third slice — see that
crate's own CHANGELOG for the full rationale). No logic change; neither
double resolves a real global.

