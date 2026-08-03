# AOT00-T1x — WASM linear-memory growth for strings/arrays

> Status: stage 1 of 2 complete (growth). Stage 2 (reclamation) is a
> separate, larger follow-up — see §4.

Twig GC completion round, Part 3 of 3 (Part 1: `vm-core`'s shared `FlatHeap`
reroute, AOT00-T1v; Part 2: `iir-to-llvm`'s `gc_live_bytes` + `alloc_bytes`/
`alloc_array` fix). This part closes a gap distinct from both: `W04`'s
struct-heap collector already gave WASM's *cons/record/closure* values (a
`WasmValue::Ref` — a `Vec` index) real reclamation. Twig's *other* WASM heap
— `iir-to-wasm`'s bump-allocated linear-memory strings and E5 arrays — is a
different representation entirely (a plain `i32` byte offset, no runtime
type tag), and had no growth path at all.

## 1. The confirmed bug

Direct source reading (not doc comments, not assumption) found, before this
change:

- Every memory-using module declared `Limits { min: 1, max: Some(1) }`
  (`iir-to-wasm/src/lower.rs`, module assembly) — a single, hardcoded 64 KiB
  page, with no way to ask for more.
- `iir-to-wasm` never emitted a `memory.grow` (0x40) or `memory.size` (0x3F)
  instruction anywhere. The opcodes existed in the `wasm-opcodes` metadata
  table but had no encoder in `iir-to-wasm/src/codegen.rs`.
- The four bump-allocation call sites — `alloc_array`, `str_concat`'s
  runtime path, `str_slice`'s runtime path, and `call_builtin "input_str"` —
  all wrote directly at the current `__array_bump` offset with no capacity
  check.

The result: any Twig (or BASIC/ALGOL/etc.) program whose cumulative
string/array data crossed 64 KiB would hit `wasm-execution`'s bounds-checked
`LinearMemory` and trap on the very first out-of-page write. `wasm-execution`
side was already correct — `LinearMemory::grow` is implemented, tested, and
spec-compliant (resizes the backing `Vec`, enforces the 65536-page / 4 GiB
ceiling, returns `-1` on failure) — this was purely a codegen gap: nothing
ever asked it to grow.

## 2. Fix: a shared `$__ensure_capacity` helper, emitted in-module

Mirrors the shape this backend already uses for `$__str_eq`/`$__str_cmp`: a
self-contained WASM function, emitted once per module (gated by the same
`uses_memory` flag that already triggers the `__array_bump` global and the
memory section itself), appended after all IIR-defined functions.

```text
$__ensure_capacity(needed_end: i64) -> ()
  current_bytes = i64(memory.size()) * 65536
  if needed_end > current_bytes {
    delta_bytes = needed_end - current_bytes
    delta_pages = (delta_bytes + 65535) / 65536      ;; round up
    if i32(memory.grow(i32(delta_pages))) == -1 { unreachable }  ;; OOM
  }
```

`needed_end` is "one byte past the last byte this write is about to touch"
— computed at each of the four call sites from values already on hand
(the current bump offset plus the block's size), then passed to
`$__ensure_capacity` **before** any store/`memory.copy` touches that region:

- `alloc_array`: `handle + 8 (i64 length header) + count * elem_size`.
- `str_concat` (runtime path): `bump + 4 (i32 length header) + len(a) + len(b)`.
- `str_slice` (runtime path): `bump + 4 + (end - start)`.
- `call_builtin "input_str"`: `bump + 4 + INPUT_STR_MAX` (256).

The module's own declared `Limits.max` was raised from the hardcoded `1` to
a new `IIRWasmConfig::max_memory_pages` field (default `1024` pages = 64
MiB), clamped to `65536` (the WASM spec's absolute page ceiling) regardless
of what's configured. **Security review caught that unconditionally jumping
straight to the 4 GiB spec ceiling would be a real regression**: this
backend's allocator is bump-only and never frees (stage 2 below), so an
unbounded cap combined with, say, a simple `input_str`-in-a-loop program
would let any long-running or malformed module monotonically grow real
host-committed memory toward 4 GiB with no backstop — where before this fix
it hit a hard (if accidental) 64 KiB trap almost immediately. Making the cap
a real, clamped, caller-configured knob keeps `$__ensure_capacity`'s
`unreachable` reachable on a much smaller, sane bound by default, while
still letting a caller that genuinely needs more (or needs untrusted-input
hardening down to an even smaller bound) ask for it explicitly.

Two opcode encoders were added to `iir-to-wasm/src/codegen.rs`:
`encode_memory_size()` (0x3F 0x00) and `encode_memory_grow()` (0x40 0x00) —
the trailing `0x00` in both is the reserved memory-index byte (this backend
only ever declares one memory).

## 3. Verification

- `iir-to-wasm/tests/test_backend.rs`:
  `alloc_array_module_declares_growable_memory_not_capped_at_one_page`
  (asserts `Limits { min: 1, max: Some(65536) }`) and
  `alloc_array_emits_memory_grow_and_memory_size_opcodes` (asserts the
  appended `$__ensure_capacity` body actually contains both opcodes).
- `lang-aot/tests/wasm_memory_growth.rs` — genuine executed end-to-end
  proof, per this session's "verify by running, not just by reading" rule:
  - `algol_repeated_array_allocation_grows_past_one_page_and_keeps_running`
    — a real ALGOL 60 program (`allocone()` called 40 times in a loop,
    each call bump-allocating a fresh 2000-element `integer array`) reserves
    ~640 KB total (ten pages) and must still return the right value.
  - `repeated_runtime_str_concat_grows_past_one_page_and_keeps_running` —
    hand-built IIR (no ALGOL source could force this: every source-level
    string operation this backend can fold, ALGOL folds — confirmed
    empirically while writing this test, see the test's own doc comment),
    calling a function whose `str_concat` operates on its own parameters
    (never foldable) 8000 times, reserving ~72 KB.
  - **Both tests were confirmed to actually catch the regression**: with
    the `Limits.max` reverted to `1`, both fail with `TrapError: unreachable
    instruction executed` — the exact failure this fix closes.
- Full existing `iir-to-wasm`/`wasm-execution`/`lang-aot` suites, including
  `lang_matrix.rs`'s complete WASM column, stay green (the only failures
  present are the 5 pre-existing CLR/JVM/LLVM toolchain gaps this session's
  `babysit-pr` runs already treat as known/ignored, unrelated to WASM).
- `cargo clippy -p iir-to-wasm -p lang-aot --all-targets -- -D warnings`
  clean (also fixed one pre-existing, unrelated `doc_lazy_continuation`
  failure in `lang-aot/tests/e6d2b_dynamic_arith.rs`, hit only because it's
  in the same clippy invocation — not touched otherwise).
- `/security-review` (Rust-specialist sub-agent): found one MEDIUM
  (unconditional 4 GiB memory ceiling with no configurable bound and no
  reclamation — fixed via `IIRWasmConfig::max_memory_pages`, see above) and
  one LOW/INFO (non-overflow-checked `needed_end` arithmetic — confirmed
  pre-existing, not a regression introduced by this diff, and already
  covered by the same documented trust-boundary trade-off the rest of this
  backend's bump-allocation arithmetic accepts). Two new tests
  (`max_memory_pages_is_clamped_to_the_wasm_spec_ceiling`,
  `max_memory_pages_smaller_than_default_is_honored`) prove the clamp is
  real in both directions.

## 4. Explicitly out of scope here (stage 2, tracked as a follow-up)

The user's explicit direction for WASM's linear-memory strings/arrays was
**"build full reclamation now,"** not just raise the cap. This stage closes
the growth half of that (nothing could grow at all before this change —
fixing that first is a genuine, real, independently-shippable improvement:
programs that would previously always trap past 64 KiB now run correctly).
It deliberately does **not** yet include:

- A free-list allocator (reused blocks after logical "end of life") — today
  every allocation is still pure bump; nothing is ever freed. Memory only
  grows, never shrinks or reuses.
- A conservative mark-sweep collector over linear memory (treating in-scope
  `i32` locals/globals as candidate pointers into the arena, per the design
  sketched in the original plan for this round).

This is a real, load-bearing scope narrowing versus the original ask, and is
called out explicitly rather than silently dropped: stage 2 (free-list +
collector) is the necessary next piece of this same round, not a permanently
deferred item.
