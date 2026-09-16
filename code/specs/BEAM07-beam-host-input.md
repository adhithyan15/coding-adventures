# BEAM07 — BEAM host input (`INPUT`/`READ-ITEM` EOF peek)

**Status:** Delivered — this slice.
**Depends on:** `BEAM06-string-array-representation.md` (the `str`
representation — an ordinary Erlang character list — this spec's
`input_str`/`string:trim` path reuses unmodified) and the `save_live_
across_imported_call!`/`restore_live_across_imported_call!` Y-register
liveness machinery already used by `str_concat`/`str_slice`/`call_closure`.
**Selected by:** the user, from `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s
post-BEAM06 reprioritization note — VM-060b was the larger of the two
remaining non-ALGOL BEAM gaps (`RND`/VM-018 being the other), unblocking 5
Dartmouth BASIC `INPUT` rows and 4 FlowMatic `INPUT`/`READ-ITEM`/EOF rows at
once — the largest remaining non-ALGOL BEAM gap in the backlog.

## 1. Problem

Two frontends already lower host-input reads to `call_builtin` names this
BEAM backend had never implemented:

- `dartmouth-basic-iir-compiler`'s `INPUT X` → `call_builtin "input_i64"`
  (parse a line as an integer); `INPUT A$` → `call_builtin "input_str"`
  (the whole line as a runtime string).
- `flow-matic-iir-compiler`'s `READ-ITEM` → `call_builtin "input_more"`
  (peek: does a record remain?) followed by `call_builtin "input_i64"` per
  field.

`iir-to-beam`'s validator (Check 6b) rejected all three outright:

```
UnsupportedOp: function "main", call_builtin Some(Var("input_i64")) is not
in the BEAM builtin set (pair?/equal?/not/print_i64/putchar)
```

Separately, even if the IIR-level lowering existed, `lang_matrix.rs`'s
`run_beam` test runner never piped a program's declared stdin to the
spawned `erl` process at all (`Command::new("erl")....output()`, stdin
inherited-but-unused) — every OTHER backend's runner already does this via
`output_with_stdin`. Both gaps had to close together for any of the 9
target rows to run.

## 2. Research method: confirm the real-`erl` I/O contract before writing any lowering

Every prior BEAM slice's own discipline ("test on real `erl` before
assuming a semantic contract") applied here directly, because host input
crosses a boundary none of the prior slices touched: unlike WASM/VM/JIT
(which share an in-process host that can hand a value across a Rust/wasm
boundary), BEAM's `erl` is a genuine, separate OS process — any "read a
line from stdin" primitive has to be **real Erlang I/O**, not a shared
buffer.

### 2.1 `io:get_line/1` — the line-read primitive

Probed directly (`erlc`-compiled module, real `erl -noshell`, actual piped
stdin — not `-eval` string literals, which cannot carry embedded newlines
cleanly):

```erlang
main() ->
    L1 = io:get_line(''),   % "hello\n"
    L2 = io:get_line(''),   % "world"   (no trailing \n — last line lacked one)
    L3 = io:get_line('').   % eof
```

Piped stdin `"hello\nworld"` (deliberately no trailing newline on the last
line) produced exactly `L1 = "hello\n"`, `L2 = "world"`, `L3 = eof`. This
is the load-bearing contract: a line keeps its trailing `"\n"` when one
was present, keeps none when the source had none, and the **atom** `eof`
(not an empty list, not a special integer) signals end of input. `''` (the
empty atom) is a valid zero-length "no prompt" argument for the `Prompt`
parameter — confirmed directly rather than assumed from the `io:get_line/2`
form's documentation.

### 2.2 `string:to_integer/1` — BASIC `INPUT X`'s parse step

C's `sscanf`-based `__twig_input_i64` (the native/LLVM runtime helper every
other backend's `input_i64` traces back to) has a specific, already-shipped
contract: parse a leading integer from the line, permissively return `0` on
any parse failure (including EOF). The real-`erl` equivalent, probed
directly:

```erlang
string:to_integer("42\n")  -> {42, "\n"}
string:to_integer("-3")    -> {-3, []}
string:to_integer("abc\n") -> {error, no_integer}
string:to_integer("")      -> {error, no_integer}
string:to_integer(eof)     -> {error, badarg}   % does NOT crash
```

This matches the `sscanf` contract exactly (leading-integer parse,
trailing garbage ignored, permissive on failure) and — importantly for the
lowering shape chosen in §3 — never raises for a malformed or wrong-typed
argument; it always returns a 2-tuple, success or failure alike, so the
lowering only needs to test whether element 1 is an integer, not catch an
exception.

### 2.3 `string:trim/3` — BASIC string `INPUT A$`'s delimiter strip

`drain_stdin_line` (the VM/JIT in-process implementation every other
backend's `input_str` is proven consistent with) reads up to but not
including the line's `\n` — the delimiter is consumed, never returned as
part of the string. `io:get_line` returns the delimiter INCLUDED, so it
must be stripped for `input_str` specifically (not for `input_i64`, where
`string:to_integer` already ignores trailing non-digit characters
regardless). Probed directly:

```erlang
string:trim("OK\n", trailing, "\n") -> "OK"
string:trim("OK",   trailing, "\n") -> "OK"   % no-op when absent
string:trim("",     trailing, "\n") -> []     % BEAM nil
```

Exactly the "strip one trailing delimiter if present, no-op otherwise"
contract needed, expressed as a single call — no manual list-walking
required.

### 2.4 All three are real function calls (`call_ext`), never guard BIFs

Disassembling a small probe module with `erlc -S` (the same method BEAM03
used to distinguish `math:pow/2`'s `call_ext` shape from `erlang:float/1`'s
`gc_bif1` shape) confirms all three of `io:get_line/1`, `string:
to_integer/1`, `string:trim/3` compile to `call_ext` against their real
module:function/arity — never `gc_bif1`/`gc_bif2`. This matters for
liveness: every one of these calls clobbers all x-registers, so any IIR
variable that must survive one needs the existing `save_live_across_
imported_call!`/`restore_live_across_imported_call!` Y-register spill
(already proven for `str_concat`/`str_slice`/`call_closure`).

The SAME probe, applied to `erlang:get/1` and `erlang:put/2` (already used
by the existing `global_store`/`global_load` lowering and reused here for
the `input_more` lookahead cache — see §3.2), surfaced a critical, and
previously latent, asymmetry:

```erlang
main(A) ->
    B = A + 1,
    erlang:put(k, v),      %% compiles to call_ext erlang:put/2 — NOT gc_bif2
    C = erlang:get(k),     %% compiles to a plain zero-GC `bif` — NOT gc_bif1 either
    {A, B, C}.
```

`erlang:get/1` never allocates (a pure dictionary lookup), so it needs no
GC-safety machinery at all — the existing `gc_bif1` lowering is a strictly
safe superset of what real `erlc` does, and every real-`erl` test in this
slice confirms it behaves correctly. `erlang:put/2` **can** allocate
(growing the dictionary's hash table) and real `erlc` always compiles it
as a genuine `call_ext`, clobbering every x-register — never `gc_bif2`.
§3.3 covers why this distinction was not academic.

## 3. Decision

### 3.1 `input_i64`/`input_str` — a "consuming read"

Both share a prefix: read the line (from a one-line lookahead cache if
`input_more` populated one — see §3.2 — otherwise a fresh `io:get_line`),
then branch on the result:

```
Result = io:get_line('')            % (or drained from the cache)
if Result == eof:
    input_i64 -> 0                  % input_str -> "" (nil)
else:
    input_i64 -> element(1, string:to_integer(Result)) if integer, else 0
    input_str -> string:trim(Result, trailing, "\n")
```

Draining the cache is expressed as one atomic `erlang:put(key, undefined)`
— exploiting `put/2`'s "returns the OLD value" contract so the check
("was anything cached?") and the clear ("consume it exactly once") happen
in a single call, with no separate get-then-put race.

`get_tuple_element`/`is_integer` (two BEAM opcodes never previously needed
by this backend) implement the "extract element 1, test whether it's an
integer" step for `input_i64`'s `string:to_integer` result — their raw
opcode numbers (45 and 66) were confirmed via `beam_opcodes:opcode(is_
integer, 2)`/`beam_opcodes:opcode(get_tuple_element, 3)` on this host's
real `erl` (OTP 29, erts-17.0.5), the same method every existing opcode
constant in `lower.rs` is already documented against.

### 3.2 `input_more` — a one-line lookahead cache in the process dictionary

FlowMatic's `READ-ITEM` calls `input_more` to peek whether a record
remains before reading its fields — real `erl` has no byte-level peek
primitive (unlike C's `ungetc`, which `__twig_input_more` already uses).
The chosen emulation: cache one line's worth of lookahead in the process
dictionary under a private key (`'$lang_vm_input_peek'`) that no BASIC or
FlowMatic source identifier can ever collide with (both frontends only
ever emit user-chosen, unprefixed names). `input_more` reads-and-caches
(via `erlang:get/1`, non-destructive) without consuming; the very next
`input_i64`/`input_str` call atomically reads-and-clears it (§3.1), so a
peeked line is delivered exactly once, and a program that never calls
`input_more` (every BASIC `INPUT`) simply always misses the cache and
reads a fresh line directly — the same code path either way.

Proven non-destructive and idempotent under repeated peeks: `test_112_
real_erl_input_more_double_peek_no_double_consume` (two `input_more` calls
in a row observe the same line, and the following `input_i64` still
consumes it — not an already-drained one), mirroring the existing native/
LLVM `portable_text_stdout_input_more_peek` double-peek proof for
`__twig_input_more`.

### 3.3 Two confirmed pre-existing framework bugs, found via real-`erl` access violations

Both were found the same way: this slice's first cut of `input_more`/
`input_i64`/`input_str` passed every SINGLE-read test immediately, but a
program combining TWO sequential reads whose results both survive (BASIC's
`INPUT A\nINPUT B\nPRINT A+B`, FlowMatic's `READ-ITEM` loop) either printed
silently-corrupted garbage or crashed the `erl` process outright with a
Windows access violation (`ExitStatus(3221225477)` = `0xC0000005`) — never
a clean Erlang-level error. Per this backlog's standing discipline, neither
was assumed away; both were bisected to a root cause with disposable probe
tests (a fresh `str_const` + `str_concat`/`global_store`-only control,
using ZERO of this slice's new code, reproduced the SAME access violation —
conclusive proof the bug was pre-existing, not introduced by the new
lowering) before being fixed.

**Bug 1 — `erlang:put/2` lowered as `gc_bif2` corrupts co-live heap data.**
The existing `global_store` lowering (and this slice's own first cut, by
direct precedent) staged `erlang:put/2` through the `gc_bif2` opcode. This
"works" as long as nothing else needing GC protection is live across the
call, because `gc_bif2`'s `Live` parameter is the ONLY mechanism protecting
OTHER x-registers from a GC the instruction might trigger — and `put/2` is
not a guard-safe BIF (§2.4), so lowering it as `gc_bif2` violates the
opcode's own contract regardless of what `Live` says. No existing op
combined "an `erlang:put/2` call" with "a separate REAL heap value (e.g. a
`str_const` list) also live across it" until this slice's two-sequential-
reads shape — the first control test that did (`str_const` a list, then an
UNRELATED `global_store` of a plain integer, then read the list back)
reproduced the identical access violation using only pre-existing,
unmodified code. Fix: `input_more`/`input_i64`/`input_str`'s own `put/2`
calls now go through `call_ext` (matching real `erlc` exactly), protected
by the same `save_live_across_imported_call!`/`restore_live_across_
imported_call!` machinery already used for `str_concat`/`str_slice`.
`global_store`'s identical `gc_bif2` usage carries the same latent bug —
confirmed but explicitly **not fixed in this slice** (§4).

**Bug 2 — `allocate`'s Y-register slots are never zero-initialized.**
`lower_iir_to_beam` emits `{allocate, StackNeed, Live}` whenever a function
needs cross-call Y-register spilling, but (unlike real `erlc`, which always
follows it with `init_yregs` to zero every freshly-reserved slot) left
each slot's initial content as whatever garbage was already on the stack.
A GC that runs before a slot's first write scans that garbage as if it
were a live term. Latent for the identical reason as Bug 1: no prior op
put two INDEPENDENT call-sites, each introducing a NEW live variable into
a NEW Y-slot, in one straight-line function — every existing multi-variable
BASIC/COBOL/Twig program either spills the SAME variable repeatedly or
never has more than one Y-slot in play before its first write. Fixed
generally, not just for the new ops: a `move {a,0} {y,N}` (BEAM nil, an
immediate — always GC-safe, unlike leaving the slot untouched) for every
allocated slot immediately after `allocate`.

Both fixes were verified independently significant: reverting either one
in isolation (while keeping the other) still reproduced a failure on the
two-sequential-reads test — Bug 1's `call_ext` conversion was the one that
actually resolved the observed corruption; Bug 2's zero-init is verified
correct and beneficial by re-running the full existing `iir-to-beam` and
`lang_matrix` suites (zero regressions), but is a genuine correctness fix
in its own right regardless of whether it alone would have masked Bug 1's
symptom in this exact scenario.

### Alternatives considered and rejected

- **A dedicated "peek" NIF or port program.** Rejected: would require
  linking a NIF into the `erl` runtime the test harness spawns, or a
  second co-process — far more machinery than a process-dictionary cache,
  for a peek need that is purely a one-line lookahead, never more.
- **Byte-level peek via `file:read`/raw fd operations.** Rejected:
  `io:get_line`'s line-oriented contract already matches what both
  frontends need (a whole line per read); a byte-level reimplementation
  would have to reconstruct line-splitting semantics `io:get_line` already
  provides correctly (§2.1's trailing-newline behavior).
- **Fixing `global_store`'s identical `gc_bif2`-for-`put/2` bug in this
  slice.** Rejected — see §4: `global_store` is used pervasively by every
  BASIC/COBOL/Twig program with a module-level variable; converting it to
  `call_ext` requires ALSO adding it to the `live_across` liveness filter
  (currently absent) and re-verifying the entire existing BEAM corpus,
  which is a properly-scoped follow-up, not a drive-by fix bundled into an
  unrelated feature slice.

## 4. Explicitly out of scope for this slice

- **`RND` (VM-018)** — still blocked on its own module-global design
  question, unrelated to host input.
- **`global_store`/`global_load`'s pre-existing `gc_bif2`-for-`put/2`
  bug** (§3.3, Bug 1) — confirmed real and reproducible with zero of this
  slice's own code, but fixing it safely requires adding `global_store` to
  the `live_across` liveness filter (it lowers to a real `call_ext` once
  converted, and isn't currently listed as one) and re-verifying the
  ENTIRE existing BEAM corpus — every program with a module-level variable
  goes through it. Flagged as a discovered gap for a dedicated follow-up
  slice (see the backlog's discovery section), not bundled here.
- **Binary I/O / non-line-oriented reads** — both frontends only ever read
  a whole line at a time; no BASIC/FlowMatic construct needs anything
  finer-grained.
- **Multi-field-per-line parsing** — FlowMatic's `READ-ITEM` already reads
  one field per `input_i64` call (one field = one line in this corpus);
  no promoted row needs splitting multiple values off a single line.

## 5. Validation

- `iir-to-beam` 0.15.0 → 0.16.0: see its own `CHANGELOG.md` entry for the
  full accounting — 11 new tests (105 → 116, all real-`erl` round-trips
  where execution matters), `cargo clippy --all-targets -- -D warnings`
  clean.
- `lang-aot`: `lang_matrix.rs`'s `run_beam` now pipes `program_stdin(p)`
  through to the spawned `erl` process via the same `output_with_stdin`
  helper every other subprocess backend (native/LLVM/JVM/CLR) already
  uses — before this slice `run_beam` never wired stdin at all. Two new
  dedicated tests, each executed against real `erl` before promotion, per
  this backlog's "probe before declaring, promote only proven cells"
  discipline:
  - `portable_text_stdout_dartmouth_basic_beam_input` — the 5 Dartmouth
    BASIC `INPUT` rows (numeric `INPUT X`; two sequential `INPUT`s summed;
    a branch-selected string chosen by a runtime `INPUT N`; string `INPUT
    A$`; two string `INPUT`s concatenated).
  - `portable_text_stdout_flow_matic_beam_read_item_and_eof` — the 4
    FlowMatic `READ-ITEM`/EOF rows (a loop over two records then EOF; the
    same loop over a genuinely empty file; a two-field-per-record loop;
    two sequential bare `READ-ITEM`s with no EOF check).
  - `feature_coverage_doc_counts_match_programs_source` updated: Dartmouth
    BASIC tuple `(51, 402)` → `(51, 407)` (5 rows each gaining one new
    `Beam` cell), FLOW-MATIC tuple `(8, 60)` → `(8, 64)` (the 4
    `READ-ITEM`/EOF rows each gaining one new `Beam` cell, going from seven
    declared backends to eight). `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth
    BASIC and FLOW-MATIC rows and grand-total prose (1667 → 1676) updated
    to match.

**Dartmouth BASIC now declares all 51/51 rows except `RND` (50/51 — `RND`
is VM-018, a separate unscoped design question) on `Beam`.** FLOW-MATIC now
declares all rows it has on `Beam` (fully complete). Combined with Twig
(49/49, VM-041 follow-up) and COBOL-60 (58/58), this closes every
non-ALGOL BEAM gap in `LANG-VM-NON-ALGOL-BACKLOG.md` except `RND` — the
one remaining item is a genuinely separate, unscoped product/architecture
question, not a probe-and-promote target.
