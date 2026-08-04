# AOT00 T7 — conformance-at-scale: generative differential harness (seed)

**Status:** Seed shipped — the first generative differential test lives in
`lang-aot/tests/lang_matrix.rs` (`t7_differential_random_u8_expressions_agree`).
**Track:** AOT00 **T7 — Conformance-at-scale** (see
`AOT00-native-aot-robustness-roadmap.md` §3 T7, §5.2). This is the "reference-
oracle differential harness + property/fuzz generation over the IIR" the roadmap
sequences to stand up early, in parallel with the runtime tracks — the safety net
that lets T1–T6 land against real coverage instead of hand-written cells.

## 1. Why

Every cell in `lang_matrix.rs` is **hand-authored**: a human picked a program and
a known result, and the campaign found each cross-backend disagreement (e.g. the
E6d-6b tagged-vs-boxed union bug — `unbox(42)=5` on native/LLVM) **one at a time**,
by writing the cell that happened to exercise it. That does not scale, and it only
covers what someone thought to write.

A **generative differential** harness inverts this: it *generates* well-formed
programs and asserts every available engine **agrees** on the result. No oracle of
"the right answer" is needed — the engines are each other's oracle (cross-backend
agreement is already the matrix's core invariant). A disagreement on any generated
program is a real bug, surfaced automatically. This directly satisfies the AOT00
robustness gate: it raises the **coverage** axis.

## 2. The seed slice (shipped)

- **Generator.** A deterministic zero-dep `xorshift64` PRNG (fixed seed → every
  failure replays) builds random `u8` expression trees over `+ & | ^` — total
  (no div-by-zero) and depth-capped (small source, no parser-depth blowup). Each
  is emitted as `fn main() -> u8 { return <expr>; }` (Nib), which runs on all
  seven engines.
- **Oracle + cross-check.** The in-process VM is the reference; every other engine
  present is cross-checked against it. The fast in-process engines (WASM, JIT) run
  on **every** generated program; the process-spawning toolchain engines
  (native/LLVM/CLR) run on a deterministic **sample** so the test stays quick
  (~13 s locally with clang + ilasm present). Absent toolchains skip.
- **Observable.** A `u8` program's observable is its **low byte**. Engines that
  report via a process exit code are already truncated to 8 bits by the OS
  (`300 & 0xFF = 44`); `run_clr` reports the launcher's printed `int32` (full
  width). All results are normalised to `& 0xFF` to compare the same observable.

## 3. What the seed already found

On its **first run** the harness flagged `vm=44` vs `clr=300` for
`fn main() -> u8 { return (200 + 100); }`. Root cause (not a low-byte disagreement
— the low byte agrees): a `u8`/`u4`/`u16` function's return value is **not narrowed
to its declared width before `ret`**. The four exit-code backends mask to 8 bits
via OS exit-code truncation; CLR's printed-`int32` channel exposes the un-narrowed
`300`. A latent conformance gap — the low byte is always correct, but the declared
width is only enforced by the reporting channel, not by the compiler. Tracked as a
follow-up (**T7-find-1**): narrow a `uN` return to its width before `ret` (CLR
`.method`, and ideally a shared frontend narrowing so the value model is uniform),
then the harness can compare full values, not just the low byte.

## 3b. Second slice (shipped) — full-value differential via BASIC `PRINT`

The u8 exit-code observable (§2) is only 8 bits, so it cannot see a disagreement
in the upper bits (a `u8 200+100` reads 44 on every engine merely because the OS
truncates the exit code). Dartmouth BASIC's `PRINT` reports the **full** integer
on stdout, so `t7_differential_random_basic_print_agree` generates
`10 PRINT <expr>` programs over `+ - *` (literals `0..=16`, depth ≤ 3 → an all-`*`
tree is ≤ `16^8 ≈ 4.3e9`, inside `i64`, never overflowing; no division, so total)
and compares whole `i64` values — negatives and large products included — across
every engine. **Strictly stronger** than the exit-code slice: a full-value
disagreement, not just a low-byte one, fails loudly. 368 full-value agreements
over 160 programs locally.

## 3c. Third slice (shipped) — control-flow differential via BASIC `IF … THEN`

The two arithmetic slices exercise only straight-line evaluation.
`t7_differential_random_basic_conditionals_agree` generates
`10 IF <a> <relop> <b> THEN 40 / 20 PRINT <c> / 30 GOTO 50 / 40 PRINT <d> / 50 END`
programs — `<a>`/`<b>`/`<c>`/`<d>` the §3b `+ - *` trees, `<relop>` over all six
(`= <> < > <= >=`). The printed value witnesses **both** the comparison result and
that the correct branch was taken, so it exercises the **comparison ops +
conditional branch + `GOTO`** codegen — the paths where cross-backend
disagreements are most likely (boolean representation, branch polarity: the class
of the E6d-6 boxed-bool `jmp_if_false` bug). 264 full-value agreements over 120
branch programs locally.

## 3d. Fourth slice (shipped) — loop differential via BASIC `FOR … NEXT`

The three prior slices cover straight-line and single-branch code but never a
**loop back-edge**. `t7_differential_random_basic_loops_agree` generates
`S := 0; FOR I = 1 TO n { S := S + <body(I)> }; PRINT S` accumulator programs —
`<body>` a `+ - *` tree whose leaves are the counter `I` or a small literal, trip
count `n ∈ 2..=6`, depth-capped so the sum stays inside `i64`. This exercises the
**loop header/latch, counter increment + bound test, and a mutated accumulator
across iterations** — a distinct codegen path and a classic divergence source
(off-by-one bounds, `NEXT` target, STEP). 264 full-value agreements over 120 loop
programs locally.

## 4. Growth path (future slices)

- Wider grammar still: `DEF FN`/`GOSUB` calls, `STEP`, nested loops, division
  (guarded) — each is a language the matrix already runs on all engines.
- Multiple frontends (ALGOL/BASIC/Oct/Twig) as generators.
- **Shrinking**: on a disagreement, minimise the program before reporting.
- **IIR-level generation** (the roadmap's end state): generate well-typed IIR
  directly, bypassing frontend grammar limits, for the widest op coverage.
- Promote to a **blocking** gate once stable (the roadmap's intent for T7).

## 5. Non-goals

- Not a *correctness* oracle against an external reference (that is a differential
  vs real SQLite/CPython-style oracle — a later T7 slice). This slice checks
  **inter-engine agreement**, which is the matrix's existing invariant at scale.
- Not fixing the T7-find-1 narrowing gap here — the seed's job is the mechanism;
  the finding is filed for its own change.
