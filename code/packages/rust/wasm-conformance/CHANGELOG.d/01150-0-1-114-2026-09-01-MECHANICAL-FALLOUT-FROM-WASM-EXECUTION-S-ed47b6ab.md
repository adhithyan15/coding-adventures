## 0.1.114 — 2026-09-01 — mechanical fallout from `wasm-execution`'s `GlobalStorage` (W35 third slice)

Slice 3 of 4 for `code/specs/W35-wasm-cross-instance-function-identity.md`
(see `wasm-execution`/`wasm-runtime`'s own CHANGELOGs for the actual new
machinery — this crate's own changes are PURELY the mechanical fallout
compiling against it requires, per this slice's own explicit scope
boundary: "ModuleRegistry wiring changes beyond whatever MECHANICAL
fallout compiling ... requires" — real cross-module `CrossModuleFunction::
identity()` propagation and the actual registry wiring slice 4 needs are
explicitly NOT touched here).

- **`RegistryHost::resolve_global`**: return type follows `wasm-execution`'s
  `HostInterface::resolve_global` signature change
  (`Rc<RefCell<GlobalStorage>>`, was `Rc<RefCell<WasmValue>>`) — the
  function body itself (`instance.globals.get(index)?.clone()`) needed no
  logic change at all, since `instance.globals`'s own element type already
  follows `wasm-runtime`'s `WasmInstance::globals` change.
- **`Executor::run_action`'s `Action::Get` handler** (the `(get "name")`
  script action, used by `assert_return`/`assert_trap` against a global
  export): reads `GlobalStorage::value`, not the whole `GlobalStorage` —
  a real funcref-typed global's `value` here is the reserved
  `WasmValue::Ref(Some(0))` sentinel (see `GlobalStorage`'s own doc
  comment in `wasm-execution`); its real identity lives in `func_ref`,
  unused by this action. This is intentionally the SAME "mechanical
  fallout only" scope as `RegistryHost`'s own fix above — real funcref-
  equality/cross-module-propagation semantics for this action are not
  addressed here.

### Verification

- `cargo build -p wasm-conformance`: clean.
- `cargo test -p wasm-conformance`: 60 lib tests + `testsuite_conformance`
  integration test (the full 257-file corpus), all passing, unchanged.
- `cargo run --release --bin wasm_conformance_report -- --write-baseline`:
  regenerated baseline byte-for-byte identical to the pre-slice-3
  committed one.

