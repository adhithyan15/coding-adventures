## 0.6.0 — 2026-06-01 (LLVM04 — `--emit=llvm-ir` + iir-to-llvm wiring)

### Added — `--emit=llvm-ir` flag and `compile_file_to_llvm_ir` API

Wires `iir-to-llvm` (v0.4.0) into the lang-aot driver.  Source files for
every supported language (Twig, Nib, Brainfuck, BASIC, Oct) can now be
lowered to textual LLVM IR (`.ll`) via:

```text
lang-aot path/to/input.bas --emit=llvm-ir [-o out.ll]
```

When `-o` is omitted the default output is the input with the extension
replaced by `.ll` (matching what downstream `llc` / `opt` expect).
Accepted aliases for the value: `llvm-ir` (canonical), `llvm`, `ll`.

#### Why cross-platform (no host gating)

The native-executable pipelines (`compile_file_to_{linux,windows,macos}_executable`)
are `cfg`-gated because they invoke the host linker.  LLVM IR emission
is **pure string output** — `compile_file_to_llvm_ir` is therefore
cross-platform and runs on any host.  Downstream `llc` / `opt`
invocations are the caller's job.

#### Public API surface added

* `pub fn compile_file_to_llvm_ir(src: &Path, out: &Path, language: Language)
   -> Result<(), LangAotError>`
* `LangAotError::LlvmBackendError(String)` — wraps human-readable errors
  surfaced by `iir-to-llvm`'s lowerer.

#### Tests added (27 total, was 25)

* `end_to_end_twig_emits_llvm_ir_via_lang_aot`
* `end_to_end_basic_print_emits_llvm_ir_with_print_extern`

Both cross-platform.  Tolerate unsupported-op / unsupported-type errors
from `iir-to-llvm` as "expected gaps" (a future LLVM05+ will broaden
coverage).  The BASIC test asserts on the `@__print_i64` extern shape.

