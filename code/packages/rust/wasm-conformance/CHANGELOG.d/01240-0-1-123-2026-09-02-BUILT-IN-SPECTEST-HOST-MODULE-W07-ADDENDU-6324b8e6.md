## 0.1.123 — 2026-09-02 — built-in `spectest` host module (W07 addendum 2 item 4)

Added a small, always-available `spectest` host-module stub directly to
`RegistryHost` (this crate's `HostInterface` implementation), closing the
gap `code/specs/W07-wasm-post-mvp-epics.md`'s "Addendum 2" item 4
predicted was likely: "check first whether this is actually a missing,
cheap, one-time host-stub addition to `wasm-conformance`'s own test
harness (likely) rather than an interpreter capability gap at all." It
was exactly that — no changes needed in `wasm-wast-parser`,
`wasm-module-parser`, `wasm-validator`, `wasm-execution`, or
`wasm-runtime`.

**Background**: the official upstream `WebAssembly/spec` testsuite has an
informal convention (documented in that repo's own interpreter test
harness, `interpreter/host/spectest.ml`) where many `.wast` files assume
a host module literally named `"spectest"` is available to import from —
a fixed set of no-op print functions, constant globals, a funcref table,
and a linear memory, used purely as test fixtures. The real reference
interpreter registers this module once, unconditionally, before running
any script. This crate's `RegistryHost` never had that registration —
before this change, EVERY `spectest` import genuinely failed to link,
correctly graded `NotYetSupported` (an honest capability gap), not
`Fail`.

**Census** (`grep -oh '(import "spectest" "[a-zA-Z0-9_]*"'
tests/fixtures/testsuite/*.wast`, confirmed live via a throwaway probe
against `wasm_conformance::run_wast_source` before implementing anything):
23 corpus files reference `spectest`, using exactly 13 real exports —
`memory` (63x), `table` (33x), `global_i32` (31x), `print_i32` (24x),
`print` (4x), `print_i32_f32` (3x), `print_i64`/`print_f64`/`global_i64`
(2x each), `table64`/`print_f64_f64`/`print_f32`/`global_f64`/`global_f32`
(1x each) — i.e. the ENTIRE real upstream `spectest` module, not just
`global_i32` as the addendum's own headline case named. A 14th name,
`"unknown"` (5x), is used ONLY inside deliberate `assert_unlinkable`
cases whose whole point is that `"unknown"` is NOT a real `spectest`
export — implementing it would have made those 5 directives regress from
`Pass` to `Fail`, so it is deliberately absent from the stub.

**Implementation** (`src/lib.rs`): a new `SpectestModule` struct holds
the fixed exports, backed by real `wasm-execution` primitives
(`LinearMemory::new(1, Some(2))`, `Table::new(10, Some(20))` /
`Table::new_with_is64(10, Some(20), true)` for `table64`, and
`Rc<RefCell<GlobalStorage>>` cells for the four globals) — never a real
parsed WASM module or a full `WasmInstance`. Values are taken verbatim
from the real upstream `spectest.ml` source (fetched and read directly,
not guessed): `global_i32`/`global_i64` = `666`, `global_f32`/`global_f64`
= `666.6`, `table`/`table64` = min 10 / max 20 funcref, `memory` = min 1 /
max 2 pages. `print*` functions are no-ops (`Ok(Vec::new())`) — no corpus
directive ever asserts on printed output, only on the import resolving
and the call succeeding. `Executor` constructs exactly ONE
`SpectestModule` for its whole run; every `RegistryHost` it builds clones
it cheaply (`LinearMemory`/`Table` already share storage via an internal
`Rc`, and each global cell already IS an `Rc<RefCell<..>>`), so
`spectest.table`/`spectest.memory` behave like a real, persistently-
registered module shared across an entire script's multiple `(module
...)` directives — matching the real upstream interpreter's own
"register once, before the script runs" behavior. Registry lookups
(`register`ed sibling modules) are tried FIRST in every `resolve_*`
method, with the `spectest` stub as a fallback only when `module_name ==
"spectest"` — no corpus file actually shadows the name, but this keeps
the precedence honest relative to how the real interpreter shares one
namespace between host- and script-registered modules.

**Diffed programmatically against the pre-fix baseline across all 257
files** (Python, comparing the `files` dict keyed by filename) — 18
files improved, summing to **272 `not_yet_supported` directives closed**
(1552 → 1280 corpus-wide), zero regressions (no file's `fail`/`trap`
counts changed):

| File | pass before → after | NYS before → after |
|---|---|---|
| `global.wast` | 53 → 117 | 71 → 7 |
| `imports.wast` | 143 → 194 | 75 → 24 |
| `return_call_indirect.wast` | 18 → 69 | 63 → 12 |
| `elem.wast` | 104 → 119 | 47 → 32 |
| `return_call.wast` | 14 → 49 | 35 → 0 |
| `data.wast` | 48 → 64 | 17 → 1 |
| `token.wast` | 46 → 47 | 15 → 14 |
| `table.wast` | 34 → 35 | 12 → 11 |
| `imports2.wast` | 9 → 19 | 11 → 1 |
| `annotations.wast` | 67 → 69 | 7 → 5 |
| `imports4.wast` | 10 → 16 | 6 → 0 |
| `data0.wast` | 1 → 5 | 6 → 2 |
| `linking.wast` | 157 → 159 | 6 → 4 |
| `start.wast` | 14 → 17 | 6 → 3 |
| `func_ptrs.wast` | 31 → 36 | 5 → 0 |
| `binary-leb128.wast` | 88 → 91 | 3 → 0 |
| `names.wast` | 484 → 486 | 2 → 0 |
| `table64.wast` | 13 → 14 | 1 → 0 |

`binary-leb128.wast` was NOT in the text-grep census (its `spectest`
import is binary-encoded LEB128 bytes, not a quoted `"spectest"` string
literal) — caught only by the full baseline diff, confirming the value
of verifying against the real corpus rather than trusting a single grep.

New tests directly exercise the stub: resolving `global_i32`/`global_i64`/
`global_f32`/`global_f64` to their real upstream values, calling
`print`/`print_i32_f32` as no-ops, resolving `memory`/`table`/`table64`
with the right limits and confirming they're genuinely usable
(`memory.size`/`table.size`), and confirming `spectest.unknown` still
correctly fails to link. The one pre-existing test that documented the
OLD "no `spectest` support" behavior
(`module_with_unresolved_import_marks_invoke_not_yet_supported`) now uses
a genuinely-unknown module name instead, preserving its original intent
(testing the `NotYetSupported`-cascade mechanism) without asserting a
now-false claim about `spectest` itself.

