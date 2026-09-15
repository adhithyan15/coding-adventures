## 0.226.0 - 2026-08-13 (the BEAM column joins the matrix)

The cross-language matrix gains **BEAM** as an eighth backend — `Backend::Beam`,
a `run_beam` arm compiling to `.beam` bytecode and executing it under a real
`erl`, gated on `erl` being present like the other external-toolchain columns.
The enum's doc comment used to describe itself as "a non-BEAM backend"; that is
no longer true.

Scoped deliberately to the Lisp-shaped end. BEAM is the one target with **no raw
memory** — no linear address space, every term immutable — so it takes the
cons/immutable half of the IIR natively and refuses the rest at validation
rather than approximating it. That refusal is the point: a program needing
`box`/`unbox` or `str_len` is rejected before emission, never miscompiled.

Of the 47 Twig programs, **19 carry `Beam`** in their `backends` list. Each was
established by EXECUTING the emitted module under `erl`, not by compiling it —
all 19 that compiled also ran, and the matrix then confirmed their values. The
remaining 28 are refused and do not list the backend: 12 on `str_len`, 7 on
other string ops, 9 on `box`.

McCarthy Lisp was already covered — `run_beam` is backend 6 of 9 in the
conformance suite and green — so this closes the gap on the cross-language side.

Five defects in the new column came out of security review, all in the
harness's core "never silently skip / never silently pass" property:

* **No `& 0xFF` on the result.** The first version masked BEAM's printed value
  to a byte to mimic `exit()` truncation. That can only ever turn a WRONG value
  into a passing one — and `SYMBOL_ID_BASE` is `1 << 29`, exactly 0 mod 256, so
  a leaked symbol tag would mask straight onto an expected 0 or 42. BEAM is the
  one column that observes the full-width value; it now fails loudly outside
  `0..=255` instead of throwing that away.
* **The result is sentinel-delimited.** `main()` runs before the `io:format`
  and shares stdout, so a program printing "5" and returning 42 produced "542",
  which parsed cleanly and was confidently wrong.
* **A non-zero `erl` exit is a failure** even when stdout parses. OTP writes
  `=ERROR REPORT` to stdout, not stderr, so the status is the reliable signal.
* **`Beam` is registered with the single-cell env var** (`LANG_MATRIX_ONLY_CELL`)
  and with the round-trip guard test that exists to catch exactly this. Without
  it, re-verifying a real BEAM failure spawned a child that died on an env-parse
  panic, and the parent reported THAT instead of the actual defect.
* **`erl_ok()` joins the skip invariant and gains a per-column floor**, so a host
  without Erlang no longer reports its legitimate skips as a runner opting out,
  and a transient spawn failure cannot silently turn the column off.

Also collapses **102 copies** of the per-test `toolchain_available` match into
one `toolchain_available(backend)` function. Every test block carried its own
identical copy, so adding a `Backend` variant meant editing them all — and any
block landing on main while a backend branch was open broke that branch's build
on merge, which happened three times while this one was open. A new variant now
needs exactly one edit.

The executed matrix stays at 4 confirmed failures (ALGOL strings on
NativeAot/Clr, COBOL COMPUTE on Clr/Jvm); the new column adds none.
