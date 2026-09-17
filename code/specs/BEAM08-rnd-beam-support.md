# BEAM08 — `RND` on BEAM, and issue #15332's `global_store` fix (VM-018 closed)

**Status:** Delivered — this slice.
**Depends on:** BEAM07's VM-D036 discovery (`erlang:put/2` must lower to
`call_ext`, never `gc_bif2`) and the `save_live_across_imported_call!`/
`restore_live_across_imported_call!` Y-register liveness machinery BEAM07
already applied to its own new `put/2` usage.
**Selected by:** the user, from `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s
post-BEAM07 reprioritization note — `RND` (VM-018) was the ONLY remaining
non-ALGOL BEAM gap in the entire backlog once BEAM07 closed host input.

## 1. Problem

Dartmouth BASIC's `RND` compiles cleanly through `iir-to-beam` validation,
then traps at runtime on real `erl`:

```
{badarith,[{erlang,'*',[undefined,48271],...}]}
```

This is `state * multiplier` inside the frontend's compiled `__basic_rnd`
helper (`dartmouth-basic-iir-compiler`'s `rnd_helper_function`), where
`state` comes from a `global_load` of `BASIC_RND_STATE` — meaning the very
first read of the module-global Park–Miller seed returns the atom
`undefined` instead of the `1` `main` is supposed to have stored before
running any source line (`emit_rnd_state_init`, always the first thing
emitted when `uses_rnd` is set).

This trap was first observed at VM-LOOP-24, then re-confirmed unresolved
at BEAM04, BEAM06, and BEAM07 — every prior slice explicitly declined to
force a fix, framing it as VM-018's still-open "RND's full DEF-FN-and-
module-global chain" design question. This slice's job was to find out
whether that framing was actually correct, or whether — like BEAM06's
"does the existing `:ets` substrate already work for strings?" finding —
the real gap was smaller and already had a proven fix sitting nearby.

## 2. Research: is this a new design question, or a known bug?

### 2.1 What IIR ops does `RND` actually use?

Direct inspection of `dartmouth-basic-iir-compiler/src/lib.rs`
(`emit_rnd_state_init`, `rnd_helper_function`) shows `RND` uses **no
special substrate of its own**. It is a completely ordinary consumer of
the SAME module-global mechanism every other BASIC/COBOL/Twig module-level
variable already uses:

- `main` seeds the state with one `global_store(Str("__basic_rnd_state"),
  Var(seed=1))` before executing any source line.
- The `__basic_rnd` helper reads it back with `global_load`, computes the
  Park–Miller step (`state * 48271 mod 2147483647`, all `i64`), and writes
  the new state back with another `global_store` — reseed and repeat paths
  do the same.

There is no VM-018-specific IIR op, no new type, no `DEF FN`-specific
lowering distinct from any other function call. "RND's full DEF-FN-and-
module-global chain" was, on inspection, just "a function that happens to
read and write a module global" — exactly what `global_store`/`global_load`
already exist to support.

### 2.2 Does `iir-to-beam`'s existing `global_store`/`global_load` lowering
have a known bug matching this exact trap shape?

Yes — and it was already found and documented, just never connected to
`RND` explicitly. BEAM07's own VM-D036 discovery (`LANG-VM-NON-ALGOL-
BACKLOG.md`'s BEAM07 section; issue #15332) is:

> `iir-to-beam`'s existing `global_store` lowering stages `erlang:put/2`
> through the `gc_bif2` opcode. Real `erlc -S` disassembly confirms
> `erlang:put/2` always compiles to a genuine `call_ext`, never `gc_bif2`
> — growing the process dictionary's hash table can allocate, and `put/2`
> does not honor a guard BIF's "only touch the declared Dest" contract, so
> `gc_bif2`'s `Live` count cannot correctly protect any OTHER live
> x-register across the call.

BEAM07 fixed this for its OWN new `put/2` usage (`input_more`'s lookahead
cache) but deliberately left `global_store`'s pre-existing, identical bug
unfixed, filing it as issue #15332 with an explicit note: "a correctly-
scoped follow-up, not a drive-by fix bundled into this feature slice."

`RND`'s trap is exactly the symptom this bug predicts. `emit_rnd_state_init`
in `main` runs `global_store(state_key, 1)` as the very first module-global
write. Under `gc_bif2`, `erlang:put/2`'s "only touch the declared Dest"
contract is violated (§2.3 confirms `put/2` is not guard-safe), so the
store does not reliably land — the RND helper's subsequent `global_load`
reads back `undefined`, exactly the observed trap. `global_load` itself is
NOT implicated (`erlang:get/1` is a genuine zero-GC guard BIF, confirmed
safe both by BEAM07's own `erlc -S` disassembly and by every passing
existing `global_load` test).

### 2.3 Confirming issue #15332's root cause is a genuine PREREQUISITE, not a coincidence

Per this task's own instructions, the question was checked concretely
rather than assumed: does fixing #15332 alone (with ZERO changes to
`dartmouth-basic-iir-compiler`'s existing RND lowering) make the exact
promoted-row Park–Miller sequence produce the correct output on real
`erl`?

Answer: **yes.** After converting `global_store` to `call_ext` (§3) with
no other change, the exact `lang_matrix.rs` RND row —

```basic
10 DEF FNR(X) = RND(X)
20 PRINT INT(FNR(-1) * 1000000)
30 PRINT INT(RND(1) * 1000000)
40 PRINT INT(FNR(0) * 1000000)
50 PRINT INT(RND(1) * 1000000)
60 END
```

— compiled through the unmodified frontend and executed on real `erl`
(OTP 29, erts-17.0.5, this host) prints exactly `22`, `85032`, `85032`,
`601352` — byte-for-byte the same sequence every other standard backend
(NativeAOT, LLVM, WASM, JVM, CLR, VM, JIT) already proves. This confirms
issue #15332 is a genuine, necessary prerequisite for `RND` on BEAM — not
a coincidentally-adjacent bug — and that fixing it is also SUFFICIENT: no
further RND-specific lowering work was needed.

## 3. Fix: apply issue #15332's already-scoped fix

Converted `global_store`'s lowering (`iir-to-beam/src/lower.rs`) from
`gc_bif2` to `call_ext`, exactly mirroring BEAM07's own fix for its
`input_more`/`input_i64`/`input_str` `put/2` calls:

```
IIR:  global_store Str("name"), Var("%v")
BEAM: move {x,rv} {x,s_val}          ; stage the value away first —
                                      ; parallel-move hazard: rv may
                                      ; already be x0 or x1
      move {a,atom("name")} {x,0}
      move {x,s_val} {x,1}
      call_ext {u,2} {u,import_put}
```

`s_val` is one scratch register above `meta.next_reg`, following the same
"stage every source into scratch BEFORE any call" discipline `array_set`'s
`:ets` path already uses. The call's return value (the process
dictionary's OLD value for that key) lands in `x0` and is discarded —
`global_store` has no destination.

Per issue #15332's own scope note, this required two changes, both made:

1. **The `gc_bif2` → `call_ext` conversion itself** (above).
2. **Adding `"global_store"` to `iir-to-beam`'s `live_across` liveness
   filter** (`lower.rs`'s call-classification list, used to compute which
   IIR variables need a Y-register spill across an imported call) — it was
   absent the whole time `gc_bif2` was assumed non-clobbering. Without
   this, `global_store` would emit a genuine `call_ext` (clobbering every
   x-register) with no compensating Y-spill for any OTHER live variable —
   the exact "wrong VALUE, not a crash" failure mode `lower.rs`'s own
   comment on this list already warns about for the six `:atomics` ops.

`global_load`'s lowering is unchanged (`erlang:get/1` via `gc_bif1` remains
correct — confirmed safe in BEAM07 and re-confirmed here).

### 3.1 Why this is a prerequisite fix, not a drive-by

Per this task's explicit instruction to check this before touching
#15332: `RND`'s `main` → `__basic_rnd` handoff is a `global_store` (the
seed) immediately followed by a `global_load` (the helper's read) — the
smallest possible instance of the exact bug class #15332 describes. There
is no alternate lowering path RND could use instead; fixing `RND` on BEAM
without touching `global_store` is not possible, because `RND`'s frontend
lowering (§2.1) has no substrate of its own to redirect. This slice
therefore fixes #15332 as a genuine dependency of its own stated task, not
as an unrelated drive-by — and closes the issue referencing this PR.

### Re-verification of the wide blast radius

Issue #15332 flagged that `global_store` is used pervasively — every
BASIC/COBOL/Twig program with a module-level variable goes through it —
and required "re-verifying the ENTIRE existing BEAM corpus" before
landing. Done: the full `iir-to-beam` suite (118 tests, up from 116) and
the full `lang_matrix.rs` BEAM test group (34 tests spanning Twig, Nib,
Oct, Brainfuck, COBOL-60, FlowMatic, and Dartmouth BASIC — every one of
which exercises `global_store` somewhere in its module-level state) all
pass unchanged after the fix. No regression.

### Alternatives considered and rejected

- **Give `RND` its own private, non-`global_store` state substrate** (e.g.
  a dedicated `:ets` cell or a bespoke opcode), avoiding `global_store`
  entirely. Rejected: this would special-case `RND` for no principled
  reason — the frontend already treats it as an ordinary module global
  (§2.1), and inventing a parallel mechanism just to route around a bug
  with an already-scoped fix would be strictly worse engineering than
  fixing the shared mechanism once.
- **Leave `RND` blocked and re-file it as still gated on #15332**, treating
  the two as separate future work. Rejected once §2.3's concrete probe
  confirmed fixing #15332 is both necessary AND sufficient — at that point
  deferring would mean shipping nothing while a fully-scoped, fully-tested
  fix sat unapplied.

## 4. Explicitly out of scope for this slice

- Any other consumer of `global_store` gaining new behavior — this fix is
  a pure bug fix to an existing, already-validated lowering; no frontend
  changed.
- `Brainfuck`'s remaining 3/6 BEAM rows — unrelated (real stdin-as-tape
  host support), a separate, already-flagged item.

## 5. Validation

- `iir-to-beam` 0.16.0 → 0.17.0:
  - `global_store` converted from `gc_bif2` to `call_ext`; added to the
    `live_across` liveness filter.
  - `test_114_global_store_emits_call_ext_not_gc_bif2` — instruction-shape
    proof (exactly one `call_ext`, zero `gc_bif2`), mirroring
    `test_86_f64_pow_emits_call_ext_not_gc_bif2`'s style for the identical
    bug class.
  - `test_115_real_erl_global_store_survives_live_across_call` — the
    disposable control test from issue #15332's own reproduction, made
    permanent: a `str_const` heap value (`"OK"`) stays intact across a
    `global_store` call that used to either corrupt it or crash `erl`
    outright with a Windows access violation, AND the stored global reads
    back correctly afterward (`OK42`).
  - 118 tests total (up from 116), all green; `cargo clippy --all-targets
    -- -D warnings` clean.
- `lang-aot` 0.345.0 → 0.346.0:
  - `portable_text_stdout_dartmouth_basic_beam_rnd` — the exact promoted
    `RND` row executed against real `erl` before promotion, per this
    backlog's "probe before declaring, promote only proven cells"
    discipline. Confirms stdout `22\n85032\n85032\n601352`, matching every
    other standard backend exactly.
  - `RND` row's `backends` list gained `Beam` (the 8th).
  - `feature_coverage_doc_counts_match_programs_source` updated: Dartmouth
    BASIC tuple `(51, 407)` → `(51, 408)` (one row gaining one new `Beam`
    cell; row count unchanged at 51 since `RND` was already a corpus row).
    `LANG-VM-FEATURE-COVERAGE.md`'s Dartmouth BASIC row and grand-total
    prose updated to match.
  - Full `iir-to-beam` suite and the full `lang_matrix.rs` BEAM test group
    (34 tests) re-run clean — see "Re-verification" above.

**Dartmouth BASIC now declares all 51/51 rows on `Beam`.** Combined with
Twig (49/49), Nib (26/26), Oct (12/12), COBOL-60 (58/58), FLOW-MATIC
(8/8), and Brainfuck (3/6, intentional — real stdin-as-tape host support
is a separate, unscoped item), **this closes every non-ALGOL BEAM gap in
`LANG-VM-NON-ALGOL-BACKLOG.md`** — the multi-month non-ALGOL BEAM
completion track this backlog has been driving since BEAM01.

## 6. Issue #15332

Closed by this PR. Both required changes from the issue's own scope note
were applied (§3) and the full re-verification the issue asked for (the
"Re-verification of the wide blast radius" section above) was executed
with zero regressions.
