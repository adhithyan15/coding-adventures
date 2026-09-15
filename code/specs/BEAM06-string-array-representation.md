# BEAM06 — string-array representation for the BEAM backend

**Status:** Delivered — this slice.
**Depends on:** `BEAM04-float-array-representation.md` (the `:ets`-backed
array substrate this spec reuses unmodified) and the scalar string
representation already shipped for `str_const`/`str_concat`/`str_slice`
(BEAM E4 slices; see `iir-to-beam/src/lower.rs`'s `"str_const"` arm).
**Selected by:** `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s trailing note
after the VM-041 follow-up (Twig closed to 49/49): the only remaining
non-ALGOL BEAM gap is Dartmouth BASIC, split three ways — 9 `INPUT` rows
(VM-060b, unscoped), `RND` (VM-018, unscoped), and 2 string-array/mixed-
`DATA` rows blocked on `iir-to-beam` having **no `str`-typed array element
representation at all** (`UnsupportedType` at validation, independent of
BEAM04's float-storage fix). This spec closes the third gap.

## 1. Problem

Dartmouth BASIC's `DIM A$(n)` allocates an `array<str>` (`dartmouth-basic-
iir-compiler`'s BA7/E4d-BA-arr lowering): `alloc_array` with `type_hint ==
"array<str>"`, `array_set`/`array_get` with `type_hint == "str"` for each
element. `iir-to-beam`'s validator (`src/validate.rs`, Check 4) rejects
`type_hint == "str"` outright unless the op is in a short allow-list
(`str_const`/`str_concat`/`str_slice`/`call`/`ret`/`mov`) — `array_set` and
`array_get` are not in it, so any program with a string-typed array element
fails validation before lowering is even attempted:

```
UnsupportedType: function "main", op "array_set" has type_hint "str"; only
the ASCII string subset is supported in this BEAM backend
```

This blocks 2 Dartmouth BASIC corpus rows: `DIM A$(2)` (plain string array)
and the mixed numeric/string `DATA`/`READ`/`RESTORE` row (VM-017's parallel
kind/numeric/string pools, where the string pool is itself an `array<str>`).

## 2. Research method: test the reuse hypothesis directly, on real `erl`

Per this backlog's "probe before declaring" discipline, and per this task's
own framing (`neg`(f64) needed zero new code in an earlier slice — check
whether the same holds here before assuming new design work), the first
question was not "what new representation do we need" but **"does the
existing substrate already work, unmodified, for strings?"**

### 2.1 What does `iir-to-beam` already use for a scalar `str` value?

`lower.rs`'s `"str_const"` arm (the only place a `str` value is *created*)
answers this directly: a v1 BEAM string is an **ordinary Erlang character
list** — `[byte, ...]`, built right-to-left from BEAM nil (`{a,0}`) with
`put_list`. It is not a handle, not a boxed reference, not an index into a
separate table — the register holds the actual list term. `str_concat`
(`erlang:'++'/2`) and `str_slice` (`lists:sublist/3`) both operate on this
value directly, with no unwrap/wrap step. This matters for §2.2: whatever
holds a `str` array element only needs to hold an **arbitrary Erlang term**,
because that is exactly what a `str` value already is.

### 2.2 Does BEAM04's `:ets` substrate hold arbitrary terms, including character lists?

Yes — this is `:ets`'s whole point (unlike `:atomics`, which is fixed-width
64-bit integers only). Confirmed directly on real `erl` (OTP 17.0.5, this
host) with a standalone probe mirroring `iir-to-beam`'s *exact* `array_set`/
`array_get` lowering shape byte-for-byte — `erlang:list_to_tuple([Idx,
Val])` + `ets:insert/2` to write, `ets:lookup_element/3` to read:

```erlang
main() ->
    Tab = ets:new(farray, []),
    Tup0 = erlang:list_to_tuple([0, "O"]),
    ets:insert(Tab, Tup0),
    Tup1 = erlang:list_to_tuple([1, "K"]),
    ets:insert(Tab, Tup1),
    V0 = ets:lookup_element(Tab, 0, 2),
    V1 = ets:lookup_element(Tab, 1, 2),
    Combined = V0 ++ V1,
    io:format("~s~n", [Combined]),        % prints "OK"
    true = (V0 =:= "O"),
    true = (V1 =:= "K"),
    true = (Combined =:= "OK"),
    % overwrite with a longer string, and the empty-string edge case:
    ets:insert(Tab, erlang:list_to_tuple([0, "hello world"])),
    true = (ets:lookup_element(Tab, 0, 2) =:= "hello world"),
    ets:insert(Tab, erlang:list_to_tuple([2, ""])),
    true = (ets:lookup_element(Tab, 2, 2) =:= "").
```

Compiled with `erlc` and run with `erl -noshell -s probe_str_array main -s
init stop`: **`ALL PROBES PASSED`.** Round-trip, overwrite, concatenation of
two round-tripped values, and the empty-string edge case (BEAM nil, the
same immediate used for the array-nil sentinel elsewhere in this backend)
all behave exactly as they do for any other Erlang list term — because a
`str` value already *is* an Erlang list term, `:ets` does not need to know
or care that it happens to represent a "string."

### 2.3 Conclusion: zero new representation work

The hypothesis holds completely. There is no new BEAM opcode, no new
runtime helper, no tagged/variant wrapper, and no change to `ir-to-beam`
(the encoder). The **only** thing standing between a `str`-typed array and
a working BEAM lowering is that `alloc_array`/`array_set`/`array_get`'s
existing `:ets`-dispatch condition checks `type_hint == "array<f64>"`/
`"f64"` and nothing else — it needs to also recognize `"array<str>"`/
`"str"` and route to the exact same code path. The mixed numeric/string
`DATA` row's "heterogeneity" (BASIC's VM-017 kind/numeric/string parallel
pools) is a frontend concern already solved by `dartmouth-basic-iir-
compiler`: it materializes THREE separate arrays (`array<i64>` kind tags on
`:atomics`, `array<f64>` values on `:ets`, `array<str>` values on `:ets`),
not one heterogeneous array — so this backend never needs a tagged/variant
element representation at all. (Traced directly in
`dartmouth-basic-iir-compiler/src/lib.rs`'s `emit_data_pool_init`/
`emit_data_value`: every `READ` checks the runtime kind tag then reads from
the *statically*-chosen typed pool array — the heterogeneity is resolved by
which array is read, not by what a single array's elements can hold.)

This means the risk this task's own scope-boundary section warned about
("if research confirms this is bounded... proceed to implement") is
resolved in the bounded direction: no whole-new dynamic-value/tagged-union
representation is needed anywhere.

## 3. Decision: widen the existing `:ets` dispatch condition, unmodified otherwise

`alloc_array` (already dispatches `type_hint == "array<f64>"` to `:ets`) now
also dispatches `type_hint == "array<str>"`. `array_set`/`array_get`
(already dispatch `type_hint == "f64"` to `:ets`) now also dispatch
`type_hint == "str"`. All three continue to run the IDENTICAL instruction
sequence BEAM04 already validated (`ets:new/2` at alloc; stage → `put_list`
`[Idx,Val]` → `erlang:list_to_tuple/1` → `ets:insert/2` at set;
`ets:lookup_element/3` at get) — the lowering code does not need to
branch on *which* element type triggered the `:ets` path, because nothing
about the emitted BEAM instructions depends on it. Every other array/tape
use (Brainfuck's byte tape, the GOSUB return-address `array<i64>` stack,
the BASIC `DATA` pool's `array<i64>` kind array) is untouched — those stay
on `:atomics` exactly as before.

`validate.rs`'s Check 4 (`type_hint == "str"` rejected unless the op is in
a short allow-list) is widened to add `"array_set"` and `"array_get"` to
that list, mirroring the existing `str_const`/`str_concat`/`str_slice`/
`call`/`ret`/`mov` entries. `"array<str>"` (the `alloc_array` type_hint)
was never rejected by Check 4 in the first place — it only pattern-matches
the exact string `"str"`, not `"array<str>"` — so no validator change is
needed for `alloc_array` itself.

### Alternatives considered and rejected

- **A tagged/variant element wrapper** (e.g. `{str, List}` / `{f64, Val}`
  tuples) for a hypothetical single heterogeneous array. Rejected: §2.3
  traced the actual frontend and found no single array is ever
  heterogeneous — BASIC's mixed `DATA` already uses three separate
  parallel typed arrays. Building a tagged-union representation nobody
  needs would be exactly the over-engineering this backlog's prior slices
  (BEAM04 §3, VM-041 follow-up §"Fix chosen") have repeatedly avoided.
- **A new binary-based string representation for arrays specifically**
  (e.g. storing array-element strings as Erlang binaries instead of
  character lists, to save space). Rejected: it would require a codec at
  every array_set/array_get boundary to convert between the scalar
  representation (character list, used by `str_const`/`str_concat`/
  `str_slice`) and a binary, with zero benefit — `:ets` already stores a
  character list natively, and consistency with the scalar representation
  means no other op (`str_concat` on `PRINT A$(0) + A$(1)`, the corpus
  row's actual proof point) needs any special-casing for "did this string
  come from an array."

## 4. Explicitly out of scope for this slice

- **`RND`** — still blocked on VM-018's module-global design question,
  unaffected by array representation.
- **VM-060b (BEAM host input)** — the `INPUT` rows remain blocked on the
  unscoped BEAM host-input design, unrelated to arrays.
- **The zero-initialization gap** BEAM04 §4 documented (`ets:new` does not
  pre-populate cells; reading a never-`array_set` index traps `badarg`
  instead of returning a zero value) is unchanged and applies identically
  to string arrays now. Neither promoted row in this slice exercises it —
  confirmed by re-reading each row's source: `DIM A$(2)` writes indices 0
  and 1 before reading them (index 2 is never touched); the mixed-`DATA`
  row's `READ`s always land on the kind-matched pool index they were just
  populated at during `emit_data_pool_init` (traced in §2.3).
- **Binary/UTF-8 strings** — this backend's `str` representation (ASCII
  character lists) is unchanged; array elements inherit whatever the
  scalar representation already is, not a new one.

## 5. Validation

- `iir-to-beam` 0.14.0 → 0.15.0:
  - `validate.rs`: Check 4's `"str"`-type_hint allow-list gains
    `"array_set"`/`"array_get"`.
  - `lower.rs`: the `alloc_array`/`array_set`/`array_get` `:ets`-dispatch
    conditions each widen from a single `==` check to also match
    `"array<str>"`/`"str"`.
  - New unit test `test_100_str_array_ops_use_ets_not_atomics`
    (instruction-shape): confirms an all-`str`-array module's emitted
    `call_ext`s target `ets:new/2`/`ets:insert/2`/`ets:lookup_element/3`/
    `erlang:list_to_tuple/1` and none target `atomics:*`.
  - New real-`erl` test `test_101_real_erl_string_array_set_get_roundtrip`:
    the exact shape of the promoted `DIM A$(2)` corpus row — `alloc_array`,
    two `str`-typed `array_set`s, two `array_get`s, `str_concat` — executed
    on real Erlang, expecting `"OK"`.
  - New real-`erl` test `test_102_real_erl_string_array_overwrite`: writes
    index 0 twice (`"lo"` then `"hi"`) and confirms the read-back is
    `"hi"`, proving `ets:insert` overwrites for string values exactly as
    BEAM04 already proved for float values.
  - Clippy with warnings denied is clean.
- `lang-aot` 0.343.0 → 0.344.0: promotes the 2 Dartmouth BASIC rows VM-
  LOOP-24/BEAM04 both left deferred on this exact gap — `DIM A$(2)` and the
  mixed numeric/string `DATA` row — via a new dedicated
  `portable_text_stdout_dartmouth_basic_beam_string_arrays` test (2
  programs) executed against real `erl` before promotion, per this
  backlog's "probe before declaring" gate.
  `feature_coverage_doc_counts_match_programs_source` updated (Dartmouth
  BASIC tuple `(51, 400)` → `(51, 402)`); `LANG-VM-FEATURE-COVERAGE.md`'s
  Dartmouth BASIC row and grand-total prose updated to match.

Dartmouth BASIC now declares 45/51 rows on `Beam` — up from 43/51. Only 6
rows remain undeclared: the 5 `INPUT` rows (VM-060b) and `RND` (VM-018) —
both genuinely separate, unscoped design questions, not probe-and-promote
items. With Twig at 49/49, COBOL-60 at 58/58, and Nib/Oct/Brainfuck/
FLOW-MATIC each fully proven per their own rows, this closes out every
non-ALGOL BEAM gap in this backlog that does not require a new,
out-of-scope architectural design decision.
