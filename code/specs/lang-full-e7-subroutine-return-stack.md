# LANG-FULL E7 — Subroutine / return-stack (`GOSUB` / `RETURN`)

**Status:** design spec, pending user sign-off (no implementation yet).
**Depends on:** E5 (arrays — COMPLETE, runs on all 7 backends) and the existing
comparison + `jmp`/`jmp_if_false` control-flow ops (the AL5 computed-goto chain,
COMPLETE).
**Unblocks:** BASIC **BA1** (`GOSUB` / `RETURN`), and serves as the reference
model for any future unstructured-return construct (e.g. an `ON … GOSUB`).

---

## 1. What this is, and the question the roadmap left open

The roadmap entry for E7 reads:

> **E7 — Subroutine / return-stack.** `GOSUB`/`RETURN` and procedure call/return —
> likely expressible with existing `call`/`ret`; confirm and add if needed.

This spec **confirms** that question, and the answer has two halves:

1. **Structured procedure call/return is already done.** A call to a named
   function that returns to its single static call site is the IIR `call` / `ret`
   pair, and it already runs on every backend: ALGOL value procedures (AL3),
   BASIC `DEF FN` (BA5), and the BA2 recursive print helpers all use it. Nothing
   to add there.

2. **BASIC `GOSUB` / `RETURN` is *not* structured call/return, and `call`/`ret`
   cannot express it.** This is the part E7 actually has to solve — and it solves
   it by **reusing existing IIR ops** (an E5 array as the return stack + the AL5
   computed-goto chain), so it needs **zero new backend ops**.

The rest of this spec explains why (2) is genuinely different, and specifies the
lowering.

---

## 2. Why `call` / `ret` cannot express `GOSUB` / `RETURN`

A Dartmouth BASIC program is **one flat sequence of line-numbered statements in a
single `main` function**. `GOSUB`/`RETURN` is *unstructured*:

```basic
10 GOSUB 100
20 PRINT 1
30 GOSUB 100
40 END
100 PRINT 9
110 RETURN
```

- `GOSUB 100` jumps to line 100 **and remembers where to come back to** (the
  statement after this `GOSUB`).
- `RETURN` resumes at **the saved return point of the most recent `GOSUB`** —
  line 20 the first time, line 40 the second time. The target is **dynamic**: the
  *same* `RETURN` at line 110 returns to two different places depending on the
  call site.

This breaks every assumption the IIR `call`/`ret` model makes:

| `call` / `ret` (structured) | `GOSUB` / `RETURN` (unstructured) |
| --- | --- |
| The subroutine is a **separate `IIRFunction`** with its own register namespace. | The "subroutine" is just a **range of lines in the same `main`** — it shares all of `main`'s variables and labels, and can be entered by `GOSUB`, by `GOTO`, or by falling through from the line above. |
| A function has **one entry** and returns a value to the call expression. | A `GOSUB` block has no defined boundary; `RETURN` can be reached by fall-through or after a `GOTO` into the middle of it. |
| The callee's locals are **fresh per call** (a real call frame). | A "called" line block reads and writes the program's shared variables in place — there is no frame to allocate. |
| `ret` goes back to the single static call site. | `RETURN`'s target is the **dynamically most-recent** `GOSUB`, i.e. a runtime stack discipline. |

You *could* try to hoist each `GOSUB`-target line range into its own
`IIRFunction` and turn `GOSUB n` into `call`. It does not work: the block's
extent is undefined (where does the "subroutine" end — at the next `RETURN`? what
about fall-through, or a `GOTO` out of it?), and the block freely reads/writes
`main`'s variables, which the code-gen backends do not share across functions
without enabler E6's globals. So `GOSUB`/`RETURN` must stay **inside `main`** and
be modelled as what it actually is: **a jump plus a runtime return-address
stack**.

---

## 3. The design: an E5-array return stack + an AL5 computed-goto, reusing existing ops

The whole construct is expressible with primitives that **already run on all 7
backends**, in exactly the substrate-reuse spirit of BA3 (arrays), BA6
(`READ`/`DATA` over an array pool), and BA2 (digit printing over `putchar`):

- **The return stack is an E5 `array<i64>`** (`alloc_array` / `array_get` /
  `array_set`), plus an `i64` register `__basic_gosub_sp` as the stack pointer —
  the same shape as BA6's `DATA` pool + `__basic_data_ptr`. Materialised once at
  the top of `main`. A fixed capacity (e.g. 64 frames — Dartmouth BASIC programs
  nest only a handful deep) is allocated; pushing past it traps via the
  bounds-checked `array_set` (the natural "GOSUB nesting too deep" error), and
  `RETURN` with an empty stack is a clean compile-or-runtime error ("RETURN
  without GOSUB").

- **Each `GOSUB` site gets a small integer id** `r` (0, 1, 2, …), assigned by a
  pre-pass over the program (one per `gosub_stmt`), and a synthetic label
  `gosub_ret_r` placed at the statement immediately after the `GOSUB`.

### 3.1 `GOSUB n` lowers to

```text
  ; push this site's return id onto the stack
  array_set  __basic_gosub_stack, __basic_gosub_sp, <const r>
  __basic_gosub_sp := __basic_gosub_sp + 1
  jmp  line_n
  label gosub_ret_r          ; execution resumes here on the matching RETURN
```

### 3.2 `RETURN` lowers to

```text
  ; pop the most-recent return id
  __basic_gosub_sp := __basic_gosub_sp - 1
  r := array_get __basic_gosub_stack, __basic_gosub_sp
  ; computed goto over every GOSUB return site — the AL5 switch chain,
  ; which is just cmp + jmp_if_false + jmp (runs on every backend):
  if r == 0 : jmp gosub_ret_0
  if r == 1 : jmp gosub_ret_1
  …
  if r == K : jmp gosub_ret_K
```

That `r == k ? jmp gosub_ret_k` chain is **identical in shape** to what
`algol-iir-compiler::emit_simple_desig_jump` already emits for `goto s[i]` (AL5)
— a proven, cross-backend construct. RETURN is "a computed goto whose index came
off a stack instead of out of an expression."

### 3.3 Worked example

The program in §2 lowers (sketch) to:

```text
main:
  __basic_gosub_stack := alloc_array 64
  __basic_gosub_sp := 0
line_10:
  array_set __basic_gosub_stack, __basic_gosub_sp, 0   ; site 0
  __basic_gosub_sp := __basic_gosub_sp + 1
  jmp line_100
  label gosub_ret_0
line_20:
  call __basic_print_int(1)            ; PRINT 1   (BA2)
line_30:
  array_set __basic_gosub_stack, __basic_gosub_sp, 1   ; site 1
  __basic_gosub_sp := __basic_gosub_sp + 1
  jmp line_100
  label gosub_ret_1
line_40:
  const 0 -> r; ret r                  ; END
line_100:
  call __basic_print_int(9)            ; PRINT 9
line_110:
  __basic_gosub_sp := __basic_gosub_sp - 1
  r := array_get __basic_gosub_stack, __basic_gosub_sp
  if r == 0 : jmp gosub_ret_0
  if r == 1 : jmp gosub_ret_1
```

Run: `GOSUB 100` (push 0) → prints 9 → `RETURN` (pop 0) → `gosub_ret_0` → prints
1 → `GOSUB 100` (push 1) → prints 9 → `RETURN` (pop 1) → `gosub_ret_1` → `END`.
Output `9`, `1`, `9`. Correct unstructured semantics, on every backend, with no
new IIR op.

---

## 4. Why this needs no new backend ops

Every op in §3 is already executed cross-backend:

| Op | Already proven by |
| --- | --- |
| `alloc_array` / `array_get` / `array_set` (the stack) | E5 (BA3 arrays, BA6 DATA pool) — all 7 backends |
| `add` / `sub` (stack-pointer arithmetic) | every integer language |
| `const`, `mov` | every language |
| `cmp_eq` + `jmp_if_false` + `jmp` (the computed-goto chain) | AL5 ALGOL switch — all 7 backends |
| in-function `label` / `jmp` (the `gosub_ret_r` labels, `jmp line_n`) | BASIC `GOTO`/`IF`, FOR/NEXT |

So E7 is a **pure frontend lowering in `dartmouth-basic-iir-compiler`** — the
same blast radius as BA6. No `interpreter-ir`, `vm-core`, or backend-crate change.

---

## 5. Implementation plan (one PR)

E7 is small enough to land in a single PR (it is one frontend feature, mirroring
BA6's size):

1. **Pre-pass** (mirrors BA5's `register_def_names` and BA6's DATA gather): walk
   the program, assign each `gosub_stmt` a sequential id `r`, and record whether
   the program uses `GOSUB` at all (so the stack array + `RETURN` dispatch are
   only materialised when needed — like BA2's lazy print helpers and BA6's pool).
2. **Stack init**: if any `GOSUB` exists, emit the `alloc_array` + `__basic_gosub_sp := 0`
   at the top of `main` (next to the BA6 DATA-pool init).
3. **`emit_gosub`**: push id, bump sp, `jmp line_n`, emit `gosub_ret_r` label.
4. **`emit_return`**: dec sp, `array_get` the id, emit the computed-goto chain
   over all recorded return sites. `RETURN` in a program with no `GOSUB` is a
   clean `Unsupported`/`Malformed` error ("RETURN without GOSUB").
5. **Capacity + safety**: fixed depth (64). Over-push traps via the bounds-checked
   `array_set`; document it as the "GOSUB too deep" runtime error. Guard the
   stack-pointer arithmetic so a malformed program can't drive a negative index
   (an empty-stack `RETURN` is rejected at compile time when statically provable,
   else bounds-checked at run time).
6. **Matrix proof** (`lang-aot/tests/lang_matrix.rs`): the §2 program (or a
   compact variant) asserting captured stdout (`919`, or with separators a
   distinct string) on **all 7 backends** — verified by RUNNING, per the
   campaign's anti-smoke-test rule. Nested `GOSUB` (a subroutine that itself
   `GOSUB`s) is the key second cell — it proves the stack discipline, not just a
   single level.
7. **Docs**: `dartmouth-basic-iir-compiler` CHANGELOG + README, bump versions,
   flip roadmap **BA1** ☐→✅ and mark **E7** confirmed/COMPLETE.

### Test matrix

| Program | Expected stdout | Proves |
| --- | --- | --- |
| `10 GOSUB 100 / 20 PRINT 1 / 30 GOSUB 100 / 40 END / 100 PRINT 9 / 110 RETURN` | `919` | same `RETURN` returns to two different sites (the stack, not a fixed label) |
| nested: a subroutine that `GOSUB`s a second subroutine before `RETURN` | a distinct string | LIFO stack discipline across nesting depth > 1 |

---

## 6. Relationship to enabler E6 (globals) — and why E7 does *not* need it

One might ask: if a `GOSUB` block reads `main`'s variables, isn't that the
cross-function global access E6 is about? No — and that is the whole point of
keeping `GOSUB`/`RETURN` **inside `main`**. Because the return stack + computed
goto live in `main`, every variable a "subroutine" touches is just a `main`
register, reachable directly. E7 deliberately avoids the function boundary
precisely so it does **not** depend on E6. (Structured `DEF FN` already crossed
that boundary and is limited to its parameter until E6; `GOSUB` sidesteps it.)

---

## 7. Open questions for sign-off

1. **Fixed vs growable stack.** This spec fixes the depth at 64 and traps on
   overflow (simple, deterministic, matches BASIC's tiny programs). A growable
   stack would need a `realloc`-style array op the backends don't have. Fix at 64
   (revisit only if a real program needs more)?
2. **`RETURN` without `GOSUB`.** Reject at compile time when the program contains
   no `GOSUB` at all (proposed); rely on the bounds-checked pop for the harder
   "dynamically empty at this point" case. Acceptable?
3. **`ON x GOSUB a, b, c`** (computed GOSUB) is **out of scope** for E7/BA1 — it
   is a direct extension (push id, then the AL5 switch on `x` to pick the target
   line) and can be a small follow-up once the base mechanism lands. Confirm
   deferral.
