# LANG85 — One dynamic-value representation across boundaries on the tagged-word columns

**Status:** Draft, 2026-10-27. Design only; no code lands with this spec.
LANG85 sits under LANG77 (the tagged-word ABI and its tag table) and LANG83
(the operation registry, trap kinds and the "language rules are
compiler-emitted IIR" principle). It replaces the incremental call-boundary
boxing work tracked in #18451 (decision comment of 2026-10-27), and it is
the umbrella for #18425, #18427, #18480 and parts of #18441, #18451 and
#18487.

> **One sentence.** On the native (x86-64 / AArch64, via `twig-aot`) and
> LLVM (via `lang-aot` → `iir-to-llvm`) columns, every dynamic value is a
> LANG77 tagged word everywhere it can be observed by code that did not
> produce it, and a raw machine value exists only inside a region whose
> type the compiler *declared*, never one it guessed from how the value is
> used.

---

## 1. Why this exists

### 1.1 The tagged word, in one table

LANG77 §2 is the source of truth for the encoding; it is repeated here only
so this spec can be read alone. A dynamic value is one 64-bit word whose
low three bits say what it is:

| Low 3 bits | Kind | Word for an example value | Decode |
|---|---|---|---|
| `000` | integer `n` | `5` → `0x28` (40) | arithmetic `>> 3` |
| `001` | nil | the whole word `1` | `x == 1` |
| `010` | symbol | `(id << 32) \| 2` | `x >> 32` |
| `011` | `#f` | the whole word `3` | `x == 3` |
| `101` | `#t` | the whole word `5` | `x == 5` |
| `111` | heap object (pair, closure chain) | `ptr \| 7` | `x & ~7` |
| `100`, `110` | reserved | — | — |

The integer range is the 61-bit *fixnum* range `[-2^60, 2^60 - 1]`.

### 1.2 What goes wrong when one value has two spellings

Every dynamic value has exactly one tagged spelling. Integers also have a
second, *raw* spelling: the machine integer `n` itself. Pointers do not:
`ptr | 7` is the only spelling a pointer has, so "raw" and "tagged" are the
same word for a pair.

Today the passes in `iir-builtin-lowering` decide, value by value, which
spelling to use. When a writer and a reader disagree, the reader decodes
the word with the wrong rule. This table shows what a *tagged* reader
concludes from a *raw* integer:

| Raw integer written | Its low 3 bits | A tagged reader sees | Consequence on main |
|---|---|---|---|
| 0 | `000` | the integer 0 | right by accident |
| 1 | `001` | **nil** | `(null? x)` is true (#18480 row 4b) |
| 3 | `011` | **`#f`** | a truthy value branches as false |
| 5 | `101` | **`#t`** | a raw 5 branches and prints as true |
| 7 | `111` | **a pair at address 0** | `(pair? 7)` is true (#18480 row 4a); `car` passes the tag check and reads address 0 |
| 8 | `000` | the integer 1 | every raw integer reads as an eighth of itself (#18451's cell and forward-global cases) |
| 15 | `111` | **a pair at address 8** | dead-branch `(car 15)` dereferences |

And a *raw* reader of a *tagged* integer simply sees `8n`:
`(define (h y) (+ y 1)) (h 1)` boxes the caller's `1` to `8`, adds a raw
`1`, and exits 9.

Two mismatches can also cancel. A value written raw and read raw is right,
even though the convention for that slot is "tagged". That is why main
passes most of its tests: most programs make an even number of mistakes.
Section 9 shows how this defeated nine rounds of incremental fixes.

### 1.3 How bad main is today

These numbers come from the differential fuzz runs of the ninth review (VM
as reference, `ref.py` for programs the VM refuses; main is main plus the
#18483 signed-compare fix, PR #18489):

| Corpus | Programs judged | main agrees with the reference | main crashes (SIGSEGV / abort) |
|---|---:|---:|---:|
| gen5, 7,000 programs | 6,974 | 3,125 (45%) | 1,078 |
| gen3, seed 880808 | 5,942 | 2,113 (36%) | 973 |
| gen4, seed 990911 | 2,839 | 1,454 (51%) | 256 |
| hand probes | 148 | 86 (58%) | 20 |

The generators are biased toward the risky shapes (values crossing calls,
joins and cells), so these are not the pass rates of ordinary programs.
They are the pass rates of exactly the programs a representation bug
breaks, and they are far from 100%.

### 1.4 The decision

In the owner's words on #18451: "a correct fix has to make the
representation uniform. Every dynamic value crossing a call boundary, a
join, a cell, a closure call or a compare must be in one representation.
Unboxing happens only inside regions the compiler has proven typed."

LANG85 turns that rule into a definition (§2), a proof obligation for
typed regions (§3), an owner and a file-by-file change list (§4), the
arithmetic and comparison semantics on tagged words (§5), the per-column
impact (§6), a validator, a fuzz gate and a PR sequence (§7–§8), and the
history that made it necessary (§9).

---

## 2. The invariant

### 2.1 Two representations, decided by declared type

Every IIR register, parameter, return value, global slot and heap field on
a tagged-word column has exactly one **representation** for its whole
lifetime:

- **`Dyn`** — the value is a LANG77 tagged word.
- **`Raw(T)`** — the value is a machine value of type `T`: `i64`, `bool`
  (0 or 1), `f64`, `str` (a heap handle without a tag), and so on.

The representation comes from **declared types only**: the type hints the
frontend wrote on parameters, return types and instruction results. It
never comes from what the value is later used for. This is the single
most important rule in the spec. Every rejected draft in §9 broke it by
inferring a representation from a use ("this parameter is only read by
`+`, so pass it raw").

| Declared type hint | Representation |
|---|---|
| `dyn` (the LANG83 PR7f-1 operand type) | `Dyn` |
| `ref<any>`, `ref<LispyPair>`, any other `ref<…>` | `Dyn` |
| `symbol` | `Dyn` (an interned symbol is already a finished tagged word) |
| `closure` (before `lower_closures_to_heap`) | `Dyn` (a closure becomes a cons chain, tag `111`) |
| `i64`, `i32`, `u8`, … | `Raw(that type)` |
| `bool` | `Raw(bool)` |
| `f64`, `str`, arrays, byte tapes | `Raw(that type)`; no `Dyn` form yet (§2.5) |
| bare `any`, `polymorphic` | `Raw(i64)`, the meaning `twig-aot`'s `normalize_params_to_i64` already gives it (§4.4) |

The last row is deliberate. Bare `any` today means two different things:
"a tagged value" to McCarthy Lisp, "an unresolved machine value" to Twig,
Nib, the CAS frontends and others. A pass cannot tell them apart, and
guessing is what failed. So a frontend that means "tagged" must say `dyn`
or `ref<any>` (McCarthy already says `ref<any>`), and Twig changes to say
`dyn` (PR4 in §8). Bare `any` keeps its machine meaning.

### 2.2 Where the invariant applies

A value is **observable** at a point if code other than the instruction
that produced it can read it there. At every observable point below, a
dynamic value is `Dyn`:

| # | Boundary | Rule |
|---|---|---|
| B1 | Direct call, argument | The argument's representation equals the callee's *declared* parameter representation. For a callee with a `dyn` parameter, the argument is `Dyn`. |
| B2 | Direct call, result | The result register's representation equals the callee's declared return representation. |
| B3 | `ret` | The returned register's representation equals the function's declared return representation. |
| B4 | Closure call (`call_closure`, lowered to `__dyn_call_closure`) | Arguments and captures are stored in the cons chain as `Dyn`. The dispatcher converts each one to the representation the *body's own declared parameter* needs (§4.2.6). The result is `Dyn`. |
| B5 | Join (`if`, `and`, `or`, `match`, `cond`) | All writers of the join register produce one representation: the consensus of their declared types if they all agree on a machine type, otherwise `Dyn`. A writer whose value is in another representation is converted on its edge. |
| B6 | Cell / field store and load | Every `dyn_pair_new` operand, every record/union `field_store` value and every `field_load` / `dyn_pair_first` / `dyn_pair_rest` result is `Dyn`. |
| B7 | Global | A global slot is `Dyn` if any access to it is declared dynamic. Every `global_store` into it stores `Dyn` and every `global_load` from it yields `Dyn`. |
| B8 | Closure capture | Captured values are `Dyn`. |
| B9 | Dynamic builtins | Every operand of `pair?`, `null?`, `equal?`, `eq?`, `not` (on a dynamic value), `=`, `<`, `<=`, `>`, `>=`, `+`, `-`, `*`, `/`, `quotient`, `remainder`, `modulo`, `length`, `list-ref`, `append`, `reverse`, `assoc`, `string-length` and every other `call_builtin` whose signature row (§4.3) says `Dyn` is `Dyn` at the point the builtin's lowering reads it. |
| B10 | Program exit | The entry function's dynamic result is coerced to the exit code by the runtime tag switch (`__lisp_exit_code`), or by a cheaper coercion when the representation proves the kind (§4.2.1). |

"Converted on its edge" always means one of the explicit conversions in
§3.3. There is no other way to change representation.

### 2.3 What the invariant buys

- **A tag check is always truthful.** An integer is never a word with tag
  `111`, `001`, `011` or `101`. So `pair?`, `null?`, truthiness and the
  C runtime's `car` check (#18441 item 4) answer correctly by construction.
- **Equality is word equality.** The encoding is injective (§5.1), so
  `eq?`, `=` on integers and the `__lisp_equal` atom case compare one word.
- **The GC can trust tags.** A slot that may hold a pointer holds `ptr | 7`
  and nothing else with tag `111`. A raw integer 15 in a reference slot
  today has a pointer's tag and could be taken for one by the precise
  collector; under the invariant it cannot exist.
- **No pass needs to know who called a function.** A callee reads its
  declared signature; a caller reads the same signature. Neither looks at
  the other's body. Both sides read one fact (the lesson
  `a-call-boundary-needs-one-representation-that-caller-and-callee-both-read`).

### 2.4 What the invariant does not change

- What the VM, WASM, JVM, CLR and BEAM columns compute (§6). They do not
  use tagged words.
- The tag table, the runtime ABI (`__dyn_*`), symbol interning, or the
  Lisp rules in `lisp_semantics.rs`.
- Frontend-typed code. `(< (string-length s) 3)` is all `Raw` today and
  stays all `Raw`.

### 2.5 Values with no dynamic form

`str`, `f64`, arrays and byte tapes have no tagged spelling: the two free
tags (`100`, `110`) are unassigned, and a raw string handle has tag `000`,
which would read as an integer. A value of such a type that reaches a
`Dyn` position is a **compile-time refusal** on the tagged columns
(`ReprError::NoDynamicForm { ty, function, instr }`), never a silent
conversion. LANG83's rule applies: a refusal is a compile error, never a
miscompile. Giving strings a dynamic form is #18428's follow-up (§10).

---

## 3. Typed regions

### 3.1 What a typed region is

A **typed region** is the set of registers whose representation is
`Raw(T)`. A raw value may exist only there. The region is not a block of
code. It is a set of registers, and a register belongs to it only if the
proof below holds for every instruction that writes it.

### 3.2 How a register is proven typed

The classifier runs per function after the conversions that earlier passes
emit (closure lowering, heap renames, symbol interning) and before any
rewrite of its own.

A register `r` is **boundary-dynamic** if it is any of:

1. a parameter whose declared type is dynamic (§2.1 table);
2. the destination of a `call` to a function whose declared return type is
   dynamic, or of the closure dispatcher;
3. the destination of a `dyn_*` builtin whose signature row (§4.3) returns
   `Dyn`;
4. the destination of a `global_load` from a `Dyn` global;
5. the destination of an instruction whose declared type hint is dynamic.

A boundary-dynamic register is `Dyn`. For every other register, the
classifier computes a candidate type by a **greatest fixpoint**:

```text
candidate(r) := declared type of r's first writer

repeat until nothing changes:
    for each instruction I that writes r:
        let t := the type I writes
                 (its result type hint; for `mov r, s`: the representation of s;
                  for `const k`: the literal's own type)
        if t ≠ candidate(r) or t is dynamic:
            candidate(r) := Dyn          (demote; never promoted back)

Raw(T) for r  ⇔  candidate(r) = T at the fixpoint
```

Demotion only moves down, from `Raw(T)` to `Dyn`, so the loop terminates
after at most one demotion per register.

In inference-rule form:

| Rule | Premises | Conclusion |
|---|---|---|
| R-Const | `I = const k`, `k : T` | `I` writes `Raw(T)` |
| R-Op | `I` is a typed op (`add`, `cmp_lt`, `str_len`, `i64_add_ovf`, …) with result hint `T` | `I` writes `Raw(T)` |
| R-Mov | `I = mov r, s` | `I` writes `rep(s)` |
| R-Unbox | `I = unbox s` (an explicit conversion, §3.3) | `I` writes `Raw(i64)` |
| R-Box | `I = box s` | `I` writes `Dyn` |
| R-Dyn | `r` is boundary-dynamic | `rep(r) = Dyn` |
| R-Join | every writer `I` of `r` writes `Raw(T)` | `rep(r) = Raw(T)` |
| R-Demote | some writer of `r` writes `Dyn`, or two writers disagree | `rep(r) = Dyn` |

Two properties follow, and the drafts in §9 lacked both:

- **Register reuse is harmless.** Twig's `let` reuses a parameter's name as
  its register (#18439). Under R-Dyn the parameter register is `Dyn` for
  its whole life, so a `let` that writes a typed value into it gets a box
  on that edge. No fact is assumed about "the argument still being there".
- **No fact depends on a later rewrite.** The classification reads declared
  types. The rewrite inserts conversions and never changes a declared type,
  so the facts cannot go stale (draft 2's failure).

### 3.3 The edges: where box and unbox happen

A conversion is needed exactly where an operand's representation differs
from what the reading position requires. The required representation comes
from the reader: the signature row for a builtin (§4.3), the callee's
declared parameter for a call, the function's return type for `ret`, the
join register's representation for a `mov`, `Raw(bool)` for a branch
condition.

| From → to | Conversion | Notes |
|---|---|---|
| `Raw(i64)` → `Dyn` | `repr_box_int` | `n << 3`; traps `Trap.Overflow` when `n` is outside the fixnum range (§5.4, from PR7) |
| `Raw(bool)` → `Dyn` | `repr_box_bool` | `3 + 2·b`: 0 → `#f` (3), 1 → `#t` (5) |
| `Dyn` → `Raw(i64)` | `repr_unbox_int` | arithmetic `>> 3`; a non-integer word yields some integer and never a pointer |
| `Dyn` → `Raw(bool)` | `repr_truthy` | TW00 truthiness: false iff the word is `#f` (3) or nil (1) |
| `Raw(i64)` → `Raw(bool)` | none | not a conversion. A typed op that needs `bool` from `i64` is the frontend's business. |
| `Raw(str/f64/…)` ↔ `Dyn` | none: refusal | §2.5 |

The `repr_*` names are **abstract ops internal to the pass** (§4.2). They
never survive the pass's last step. The table at §4.2.5 gives each one's
expansion.

Two directions are deliberately missing:

- **No conversion is ever inserted on a `Dyn` value to make it `Dyn`.**
  Boxing an already-tagged pointer produces `p << 3`, a forged word the
  collector cannot see (draft 3). The validator rule V8 rejects it.
- **No conversion makes a `Dyn` value `Raw` for a position that may
  dereference it.** Only `Dyn` operands feed `dyn_pair_first` / `rest` /
  `field_load`. A `repr_unbox_int` result only feeds arithmetic,
  comparisons and machine builtins, so it cannot forge a pointer.

### 3.4 Optimisations are local rewrites with a lemma

Every optimisation is a rewrite of a small pattern into an equivalent one.
Each comes with a lemma that says why the two are equal on every input,
including the error cases (traps), and the validator runs after all of
them. An optimisation that cannot state its lemma does not land.

| # | Rewrite | Valid when | Lemma |
|---|---|---|---|
| O1a | `repr_truthy(repr_box_bool(b))` → `b` | always | `box_bool` yields only 3 or 5; `truthy(3) = 0`, `truthy(5) = 1` |
| O1b | `repr_unbox_int(repr_box_int(x))` → `x` | `x` is proven in the fixnum range: a literal in range, a `repr_unbox_int` result, or the result of a fixnum-checked op | `box` traps iff `x` is out of range (§5.4); in range, `(x << 3) >> 3 = x` |
| O1c | `repr_box_int(repr_unbox_int(d))` → `d` | `d` is proven an integer word (tag `000`): produced by `repr_box_int` or by tag-aware arithmetic | for `d = n << 3`: `(d >> 3) << 3 = d`. **Not valid** for an unknown `d`: a pair would become an integer. |
| O2 | `repr_box_int(const k)` → `const (k << 3) : dyn` | `k` in range | constant folding of O1b's box |
| O2′ | `repr_box_int(const k)` → `trap Trap.Overflow` at that point | `k` out of range | the box would trap there. The trap stays at the point of use, so a program with the literal in dead code still runs, as on the VM |
| O3 | `repr_lt(d, repr_box_int(const k))` → `cmp_lt [i64] d, (k << 3)` (and `=`, `<=`, …) | always | §5.1 and §5.2 |
| O4 | a dynamic arithmetic builtin whose operands are both `Raw(i64)` → a raw checked op with a `Raw(i64)` result | both operands proven `Raw(i64)` | the reference lowering unboxes nothing, does the op, and boxes the result. The checked op traps on the same inputs (§5.4). |
| O5 | `jmp_if_false (repr_truthy (dyn_is_pair x))` → `jmp_if_false (cmp_eq [i64] (and x, 7), 7)` | always | `dyn_is_pair` is `tag == 7`; truthy of its 5/3 result is the same bit |
| O6 | an unboxed calling convention (worker/wrapper) | future, its own PR (§10) | see below |

**O6 is the only way a raw value may cross a call**, and it is not part of
the initial migration. When it lands, it **clones** a function into a raw
*worker* plus the original as a `Dyn` *wrapper*. The public symbol, the
closure table and every unknown caller keep the `Dyn` ABI. Only direct call
sites that the pass proves pass `Raw` values may call the worker. A
function's own declared signature is never rewritten in place. Drafts 1, 8
and 9 rewrote it in place.

---

## 4. Ownership and the change list

### 4.1 The owner

The invariant has **one owner on the tagged columns**: a new pass,
`iir_builtin_lowering::lower_uniform_repr`, in a new module
`iir-builtin-lowering/src/uniform_repr/`. It replaces, on those columns
only, the representation decisions now spread across `lower_dynamic_arith`,
`lower_dyn_repr` and `lower_box_unbox_to_runtime_calls`. No other pass on a
tagged column may insert, remove or reinterpret a `box`, an `unbox` or a
`dyn_box_int` / `dyn_unbox_int` call. Earlier passes may emit explicit
`box` / `unbox` (closure lowering, list helpers); the owner treats those
as conversions and checks them (V8).

Callers after PR5:

```text
twig-aot::prepare_module_for_aot            lang-aot::compile_iir_to_llvm
  lower_global_io                             lower_global_io
  lower_closures_to_heap  (Uniform mode)      lower_closures_to_heap (Uniform mode)
  lower_heap_builtins_runtime                 lower_heap_builtins_runtime
  intern_symbols                              intern_symbols
  lower_uniform_repr  ◄── owner               lower_uniform_repr  ◄── owner
  (string literals, pre_lower, …)             concretize_scalar_any_for_llvm

lang-aot::jit_lisp::run_mccarthy_on_jit
  lower_heap_builtins_runtime → intern_symbols → lower_uniform_repr
```

`lower_dynamic_arith`, `lower_dyn_repr` and
`lower_box_unbox_to_runtime_calls` are no longer called on these three
pipelines. `lower_dynamic_arith` keeps running on the WASM, JVM, CLR and
BEAM pipelines and on the CAS JIT path, which this spec does not change.

### 4.2 The new module

```text
iir-builtin-lowering/src/uniform_repr/
  mod.rs          lower_uniform_repr(module) -> Result<(), ReprError>
  signatures.rs   the operand-representation table (§4.3)
  classify.rs     §3.2: Rep, boundary-dynamic seeds, the demotion fixpoint, globals
  convert.rs      §3.3: edge conversions; dynamic builtins → abstract repr ops
  optimise.rs     §3.4: O1–O5, each a function with its lemma in its doc comment
  validate.rs     §7.1: V1–V12; also exported as validate_uniform_repr
  expand.rs       abstract repr ops → word-level IIR and dyn_* builtins
```

`lower_uniform_repr` runs these steps in order:

1. `lisp_semantics::check_reserved_names` (unchanged from `lower_dyn_repr`).
2. Set aside `lisp_semantics::is_compiler_helper` functions, as
   `lower_dyn_repr_marked` does today.
3. `classify` every function, and every global across the module.
4. `convert`: insert edge conversions, lower dynamic builtins to abstract
   ops, mark Lisp rules as `call __lisp_*`. Refuse anything without a
   signature row.
5. `optimise` (from PR8; a no-op before).
6. `validate`. Any violation is a compiler bug and fails the compile with
   `ReprError::Violation(list)`. It never emits code.
7. `expand` to word-level IIR. Then `lisp_semantics::expand_inline_rules`
   and `add_used_helpers`, as now.

#### 4.2.1 The exit boundary

The entry function's declared return is dynamic. Its `ret` gets a coercion
chosen from the returned register's *provenance*, not its use:

| The returned value is proven to be | Coercion |
|---|---|
| a `repr_box_int` result or tag-aware arithmetic result | `dyn_unbox_int` (cancels with the box under O1b) |
| a `repr_box_bool` result or a predicate result (`dyn_is_pair`, `dyn_is_nil`, `__lisp_not`, `__lisp_equal`) | `__lisp_truthy` |
| a symbol constant | the word itself |
| anything else (a parameter, a call result, a cell read, a join) | `__lisp_exit_code`, the runtime tag switch |

This is today's `insert_unbox_before_lisp_rets`, with its three sets
(`boxed_regs`, `call_result_regs`, `bool_regs`) replaced by provenance.

#### 4.2.2 Lowering of dynamic builtins (the `convert` step)

| Frontend form | After `convert` (abstract) |
|---|---|
| `call_builtin "+" a b` (also `-`, `*`, `/`, `quotient`, `remainder`, `modulo`, unary `-`) | `repr_add a′ b′` (`repr_sub`, …), `a′` and `b′` converted to `Dyn`; result `Dyn`. Under O4, both operands `Raw(i64)` → raw checked op, result `Raw(i64)`. |
| `call_builtin "=" a b` (also `<`, `<=`, `>`, `>=`) | `repr_eq a′ b′` (…) with `Dyn` operands; result `Raw(bool)` |
| `call_builtin "not" x`, `x` is `Dyn` | `call __lisp_not x` (result `Dyn`) |
| `call_builtin "not" x`, `x` is `Raw(bool)` | unchanged: numeric `not` |
| `call_builtin "equal?" a b` | `call __lisp_equal a′ b′` (result `Dyn`) |
| `dyn_pair_new`, `dyn_pair_first`, `dyn_pair_rest`, `dyn_is_pair`, `dyn_is_nil`, `dyn_tag` | unchanged; operands converted per §4.3 |
| `alloc ref<LispyPair>` + `field_store` (record/union constructor) | from PR6: `dyn_pair_new` chain, operands `Dyn` |
| `field_load c, i` | from PR6: `dyn_pair_rest`ⁱ then `dyn_pair_first`; this read is checked (#18441) |
| `jmp_if_false c` / `jmp_if_true c`, `c` is `Dyn` | `repr_truthy c` then the jump |
| explicit `box s` / `unbox s` from an earlier pass | `repr_box_int s` / `repr_unbox_int s`, after V8 checks `s`'s representation |

#### 4.2.3 Arithmetic and comparison expansion

The two expansion forms for each abstract op are defined in §5. PR5 uses
the **reference** form: untag with `dyn_unbox_int`, do the typed op, retag
with `dyn_box_int`. That is the shape main already emits, so PR5 changes
only *which* values are converted, not how. PR8 switches to the
**tag-aware** form.

#### 4.2.4 Types after expansion (GC roots)

After `expand`, every register whose representation is `Dyn` carries the
hint `ref<any>`, and only those registers do. `ref<any>` is a GC root on
both native backends (`is_gc_root_ty` is a deny-list of machine types). It
is an `i64`-width word on LLVM (`llvm_type_for("ref<any>")`). Word-level
compares and arithmetic on `Dyn` words are typed `i64`, because a
comparison's hint is its operand type (lesson
`a-comparison-s-type-hint-is-its-operand-type-so-a-ref`).

This also closes a latent rooting gap. Today `normalize_params_to_i64`
retypes Twig's `any` parameters to `i64`. That drops them from the
precise stack map even when they hold a pair. A `dyn` parameter becomes
`ref<any>`, which `normalize_params_to_i64` does not touch.

#### 4.2.5 Abstract ops and their expansions

| Abstract op | Reference expansion (PR5) | Tag-aware expansion (PR8) |
|---|---|---|
| `repr_box_int x` | `call_builtin dyn_box_int x` | `i64_mul_ovf x, 8` (traps exactly outside the fixnum range, §5.4) |
| `repr_unbox_int d` | `call_builtin dyn_unbox_int d` | an inline arithmetic right shift by 3. LLVM needs `shr` lowered as `ashr` first: `iir-to-llvm` lists no `shl`/`shr` today. |
| `repr_box_bool b` | `mul b, 2` ; `add _, 3` (typed `i64`) | same |
| `repr_truthy d` | `call __lisp_truthy d` (inline rule) | same |
| `repr_add a b` | unbox, unbox, `add`, box | `and a, -8` ; `and b, -8` ; `i64_add_ovf` |
| `repr_sub a b` | unbox, unbox, `sub`, box | as `repr_add` with `i64_sub_ovf` |
| `repr_mul a b` | unbox, unbox, `i64_mul_ovf`, box | `unbox a` ; `and b, -8` ; `i64_mul_ovf` |
| `repr_div` / `repr_mod` | unbox, unbox, `div` / `mod`, box | unchanged (division needs the untagged values) |
| `repr_eq a b` | `cmp_eq [i64] a, b` | same (no untag; §5.1) |
| `repr_lt a b` (and `le`, `gt`, `ge`) | unbox, unbox, `cmp_lt [i64]` | `cmp_lt [i64] a, b` (no untag; §5.2) |

In PR5 the arithmetic ops wrap at 2^64 exactly as main does. PR7 makes
`dyn_box_int` checking and switches `mul` to `i64_mul_ovf` (§5.4).

#### 4.2.6 Closures in Uniform mode

`lower_closures_to_heap` gains a mode parameter:
`lower_closures_to_heap_with(module, ClosureRepr::Uniform | ClosureRepr::Legacy)`.
The old name stays as `Legacy` for the managed pipelines. In Uniform mode:

- `store` never boxes. Captures and arguments are `Dyn` by B4/B8 and go
  into the chain as they are. The convert step boxes a `Raw` capture on its
  edge like any other.
- The dispatcher's `extract` converts per body **and per parameter**, from
  that body's declared parameter type: a `dyn` parameter gets the chain
  word, a `Raw(i64)` parameter gets `unbox`. Today's module-wide
  `params_are_boxed = dispatch.iter().any(…)` is the heuristic this
  replaces.
- The `may_hold_a_pointer` assert is unnecessary in Uniform mode, since no
  unknown value is ever boxed. It stays for Legacy.

### 4.3 The operand-representation table (`signatures.rs`)

Every builtin that can reach a tagged column has one row. The row says
what each operand position requires and what the result is. A
`call_builtin` with no row, given a `Dyn` operand, is refused
(`ReprError::NoSignature { builtin }`). Failing closed here means the next
builtin someone adds cannot silently read a tagged word as raw.

| Builtin / helper | Operands | Result |
|---|---|---|
| `dyn_pair_new` (`cons`) | `Dyn`, `Dyn` | `Dyn` |
| `dyn_pair_first`, `dyn_pair_rest` (`car`, `cdr`) | `Dyn` | `Dyn` |
| `dyn_is_pair`, `dyn_is_nil` (`pair?`, `null?`) | `Dyn` | `Dyn` (the words 5 / 3) |
| `dyn_tag` | `Dyn` | `Raw(i64)` |
| `dyn_box_int` | `Raw(i64)` | `Dyn` |
| `dyn_unbox_int` | `Dyn` | `Raw(i64)` |
| `dyn_nil`, `dyn_symbol` | — | `Dyn` |
| `__lisp_equal`, `__lisp_not` | `Dyn`… | `Dyn` |
| `__lisp_truthy`, `__lisp_exit_code` | `Dyn` | `Raw(i64)` |
| dynamic arithmetic (`+ - * / quotient remainder modulo`) | `Dyn` (or both `Raw(i64)` under O4) | `Dyn` (or `Raw(i64)`) |
| dynamic comparison (`= < <= > >=`) | `Dyn`, `Dyn` | `Raw(bool)` |
| `print_i64`, `write_i64`, `putchar`, `io_out` | `Raw(i64)` | — |
| `str_len`, string ops, `alloc_bytes`, byte ops | `Raw(str)` / `Raw(i64)` | `Raw(…)` |
| `string-length` with a `Dyn` operand | refused (#18428) | — |
| `global_store` / `global_load` | the slot's representation (B7) | the slot's representation |

`print_i64` of a `Dyn` value prints the unboxed integer, which matches
main. Printing a non-integer dynamic value correctly needs a print rule
(§10).

### 4.4 File-by-file, function-by-function

#### `iir-builtin-lowering/src/dyn_repr.rs`

| Item | Change |
|---|---|
| `lower_dyn_repr` | Kept as a compatibility wrapper around `lower_uniform_repr` until PR9, then deleted. Its callers (`twig-aot`, `lang-aot` LLVM, `jit_lisp`) switch in PR5. |
| `lower_dyn_repr_marked` | Deleted in PR9. The helper set-aside moves into `uniform_repr::mod`. |
| `lower_dyn_repr_function` step 0: `rename_lisp_equal` | Moves into `convert` unchanged. |
| step 0: `rename_lisp_not` (`dyn_results` heuristic) | Replaced: `not` becomes `__lisp_not` iff its operand's representation is `Dyn` (§4.2.2). |
| step 1: `lisp_arg_regs` | **Deleted.** It decided boxing from a use (a register that feeds a lisp call). |
| step 2: `boxed_regs` seeding and the **bidirectional `mov` fixpoint** | **Deleted.** The bidirectional spread was the mechanism behind draft 6's failure. It is replaced by the one-directional demotion fixpoint of §3.2. |
| step 3: const boxing, `INT_MAX_BOXABLE` / `INT_MIN_BOXABLE` "left raw" | Replaced by O2 / O2′. An out-of-range literal in a `Dyn` position traps, and is never left raw. |
| step 4: `wrap_tagged_conditions`, including the `boxed_bool` repointing | **Deleted.** The general rule `Dyn` condition → `repr_truthy`, plus O1a, covers both cases. |
| step 5: `insert_unbox_before_lisp_rets` | Rewritten as §4.2.1, reading provenance. |
| step 5b: `box_raw_scalar_return` | **Deleted** (the canonical call-boundary heuristic). A `Raw` returned value is boxed by B3's edge rule, and a returned `Dyn` parameter needs nothing. |
| `is_lisp_function_call`, `lisp_builtin_name`, `LISP_BUILTINS`, `LISP_VALUE_HELPERS`, `LISP_ARG_ONLY_BUILTINS`, `is_lisp_arg_only_builtin` | **Deleted.** Replaced by the signature table (§4.3) and declared callee signatures (B1/B2). |

#### `iir-builtin-lowering/src/dynamic_arith.rs`

| Item | Change |
|---|---|
| `lower_dynamic_arith`, `lower_dynamic_arith_function`, `unbox_operand`, `typed_op` | No longer called on the tagged pipelines (PR5). Unchanged for WASM / JVM / CLR / BEAM / the CAS JIT. |
| The call-result relabelling fixpoint in `lower_dynamic_arith` (`dynamic_callees`, `returns_dynamic`, rewriting `func.return_type` to `ref<any>`) | Never runs on tagged columns again. This is the "ABI inferred from a body rewrite" pattern: a function's signature changed because of what this pass emitted in its body. On the managed columns it stays until their own spec (§10). |
| `producer_types` (last-writer-wins; #18425 comment) | Irrelevant on the tagged columns after PR5. The managed fix is consensus, as in §3.2 (§10). |
| `is_comparison` and the `box` of a comparison's `bool` result | On the tagged columns, comparisons yield `Raw(bool)`, and the edge rule uses `repr_box_bool` (words 3 / 5). Today the result is `dyn_box_int(0 or 1)`, the integers 0 and 8. Within one function `wrap_tagged_conditions` repoints the branch to the raw `bool`; across a call nothing does, and truthiness reads both as **true** (#18480 row 2). |
| `lower_box_unbox_to_runtime_calls` | Absorbed into `expand` (PR5). Deleted in PR9 once no pipeline calls it. |

#### `iir-builtin-lowering/src/dyn_repr_structural.rs`

| Item | Change |
|---|---|
| `tagged_boundary_functions` (`pub(crate)`, used only by `dyn_repr`) | **Deleted** in PR9. The tagged ABI comes from declared signatures. |
| `lisp_functions`, `seeded_lisp_functions`, `has_lisp_param`, `lower_dyn_repr_structural` | Unchanged (managed columns). PR4 adds `dyn` wherever bare `any` is accepted, with the same meaning as `any` there. |

#### `iir-builtin-lowering/src/list_ops.rs`

The list helpers already follow the uniform rule. Each helper's parameters
and return are declared `ref<any>`, and each converts explicitly by
declared type. So they need almost no change:

| Item | Change |
|---|---|
| `build_length_helper`, `build_listref_helper`, `build_append_helper`, `build_reverse_helper` | Unchanged. Their explicit `box` / `unbox` become `repr_box_int` / `repr_unbox_int`, checked by V8. |
| `box_if_scalar`, `scalar_var_map`, `is_scalar_ty` | Unchanged and still correct: they box an operand whose *declared* type is a machine scalar. On the tagged columns they become redundant with the edge rule, and can be dropped from the runtime path in PR9. |
| `build_assoc_helper` (`unbox` both keys, then `cmp_eq [i64]`) | Optional follow-up: compare the key words with `repr_eq`. Word equality is `eq?` (§5.1), so symbol keys work too. It is not required for the invariant. |

#### `iir-builtin-lowering/src/closure_heap.rs`

| Item | Change |
|---|---|
| `lower_closures_to_heap` | Gains `ClosureRepr` (§4.2.6). Tagged pipelines pass `Uniform`. |
| `params_are_boxed` (module-wide `any`) | Replaced in Uniform mode by a per-body, per-parameter decision read from declared types. |
| `store`, `may_hold_a_pointer` | No boxing in Uniform mode. The assert stays for Legacy. |
| `build_dispatcher` / `extract` | Per-parameter conversion in Uniform mode. The index test (`call_builtin "="` on the boxed index and a `const k`) becomes `repr_eq` and folds to `cmp_eq [i64] idx, (k << 3)` (O3). |

#### `iir-builtin-lowering/src/immediates.rs`

`materialize_immediate_operands` runs only on JVM / CLR / WASM and is not
changed. The uniform pass handles immediates itself: an `Operand::Int` or
`Operand::Bool` in a position that requires `Dyn` is replaced by a
`const … : dyn` holding the tagged word (O2), or a `trap` (O2′). V3
checks that no immediate is left in a `Dyn` position.
`value_operand_indices` is a useful model for "which operands are values",
and `convert` should reuse it rather than re-derive it.

#### Other files in the crate

| File | Change |
|---|---|
| `heap.rs` (`lower_heap_builtins_runtime`, `lower_heap_function_runtime`, `RUNTIME_RENAMES`) | Unchanged. The `ref<any>` retype it already does is a declared type, which `classify` reads. |
| `symbol_intern.rs` (`intern_symbols`) | Unchanged. A `symbol` const is `Dyn` and never boxed (it is already the finished word). |
| `lisp_semantics.rs` | Unchanged. `__lisp_truthy`, `__lisp_not`, `__lisp_equal` and `__lisp_exit_code` are the Lisp rules the pass calls. |
| `global_io.rs` | Unchanged. `classify` computes global representations (B7) module-wide from the access sites. |
| `error.rs` | New `ReprError { ReservedName, NoDynamicForm, NoSignature, Violation(Vec<ReprViolation>) }`. `lower_dyn_repr`'s `ReservedNameError` folds in. |
| `lib.rs` | Exports `lower_uniform_repr`, `validate_uniform_repr`, `ReprError`, `ClosureRepr`. |
| `numeric.rs`, `lower.rs`, `closure.rs` | Unchanged. |

#### Outside `iir-builtin-lowering`

| Crate / function | Change |
|---|---|
| `twig-ir-compiler` (`compile_top_level_lambda`, `compile_anonymous_lambda`, record/union constructors and accessors, `compile_apply` result hints, `compile_if` / `and` / `or` / `match` join consensus, `global_get`) | Emit `dyn` wherever it now emits bare `any` for a dynamic value: parameters (unless annotated), return types, `call` / `call_closure` results, the dynamic `call_builtin` fallback, `global_get`, and join registers whose writers disagree. Typed fast paths (`add [i64]`, `cmp_lt [i64]`, `str` paths) are unchanged. |
| `twig-aot::prepare_module_for_aot` | Calls `lower_uniform_repr` in place of `lower_dynamic_arith` + `lower_dyn_repr` + `lower_box_unbox_to_runtime_calls`, and `lower_closures_to_heap` in Uniform mode. `normalize_params_to_i64`, `propagate_aot_types` and `default_any_to_i64` are unchanged; they never see `dyn`, because `expand` has already turned it into `ref<any>`. |
| `lang-aot::compile_iir_to_llvm`, `jit_lisp::run_mccarthy_on_jit` | The same switch. |
| `lang-aot` `is_narrowable_dynamic`, `concretize_scalar_any_for_{wasm,jvm,cil,beam,llvm}` | PR4: accept `dyn` as bare `any` is accepted today. |
| `vm-core`, `jit-core`, `interpreter-ir` type helpers, `iir-to-{wasm,jvm-class-file,cil-bytecode,beam}` validators | PR4: accept `dyn` as bare `any` is accepted today. |
| `iir-to-llvm` | PR6: no new op for records (they become `dyn_*` calls). PR8: lower `shl` / `shr` (as `shl` / `ashr`) for the inline untag, if the tag-aware form uses them. |
| `twig-aot/runtime/dynval_runtime.c` `__dyn_box_int`; `dynval-runtime` `box_int`; LANG77 §6 golden test | PR7: range-checked box. |
| `interpreter-ir` `op_registry` row for `dyn_box_int` | PR7: `may_trap: [Trap.Overflow]`. |

### 4.5 What gets deleted: the call-boundary heuristics

These are deleted, or never merged, and must not come back on a tagged
column. Each one decides a representation from something other than a
declared type:

1. `box_raw_scalar_return` in `dyn_repr.rs`, plus its parameter
   exemption (`param_is_tagged`).
2. `tagged_boundary_functions` used as the tagged-column ABI.
3. `lisp_arg_regs` and the bidirectional `mov` spread in `dyn_repr.rs`.
4. `lower_dynamic_arith`'s return-type relabelling, on the tagged columns.
5. `closure_heap`'s module-wide `params_are_boxed`, on the tagged columns.
6. The unmerged draft machinery: `CallBoundary::analyse`,
   `raw_only_params`, `tagged_at_every_site`, `is_tagged_copy`,
   `holds_provably_raw`, `tagged_only_params`, `box_returning_functions`.
   It is recorded in §9 so it is not reinvented.

---

## 5. Comparisons and arithmetic on tagged words

### 5.1 Equality is word equality

The encoding is **injective**: two different dynamic values never share a
word.

| Kinds compared | Why their words differ |
|---|---|
| int vs int | `n << 3` is injective on the fixnum range |
| int vs nil / `#f` / `#t` / symbol / pair | the tags differ (`000` vs `001` / `011` / `101` / `010` / `111`) |
| symbol vs symbol | interning gives distinct names distinct ids |
| pair vs pair | distinct cells have distinct addresses |

So `eq?` is `cmp_eq [i64]` on the words, and `=` on integers is the same
instruction. Deep `equal?` stays `__lisp_equal`, whose atom case is this
word compare.

`=` on two *different kinds* is the one place where word equality makes a
choice. `(= 0 #f)` compares 0 with 3, so it is false: the two are different
values, as TW00 says. The ninth review's comment on #18480 records both
natives disagreeing with the VM on `(= 0 #f)` through a parameter.
`vm-core`'s `Value::as_i64` turns `#f` into 0, so the VM may answer true.
Before PR5's gate counts such a program, the oracle's answer has to be
settled (§11, question 6).

### 5.2 Signed order is preserved

For fixnums `a` and `b`, `a << 3` is `8a`, and `8a` cannot overflow `i64`.
Multiplying by a positive constant preserves order, so:

```text
a < b   ⇔   8a < 8b   ⇔   (a << 3) <ₛ (b << 3)        (signed i64 compare)
```

| `a` | word of `a` (hex) | `b` | word of `b` | `a < b` | signed word compare | **unsigned** word compare |
|---:|---|---:|---|---|---|---|
| -1 | `FFFF_FFFF_FFFF_FFF8` | 0 | `0` | true | true | **false** |
| -8 | `FFFF_FFFF_FFFF_FFC0` | 100 | `320` | true | true | **false** |
| 3 | `18` | 5 | `28` | true | true | true |
| 2^60−1 | `7FFF_FFFF_FFFF_FFF8` | −2^60 | `8000_0000_0000_0000` | false | false | **true** |

The comparison must be **signed** (#18483, PR #18489). Word compares on
non-integers give some boolean and never touch memory. The reference
interpreter refuses such programs, so they are outside the fuzz gate.

### 5.3 Arithmetic must untag and retag, or use tag-aware forms

| Op | Identity on tagged words | Tag-aware form | Why it is right |
|---|---|---|---|
| `+` | `8a + 8b = 8(a+b)` | `(a & −8) + (b & −8)` | Ints already have tag `000`, so the mask changes nothing. For a non-int operand the mask forces the result's tag to `000`, so it can never forge a pointer. |
| `−` | `8a − 8b = 8(a−b)` | `(a & −8) − (b & −8)` | as `+` |
| `*` | `a · 8b = 8(ab)` | `(a >> 3) · (b & −8)` | exactly one operand must be untagged |
| `/`, `quotient`, `remainder`, `modulo` | no tagged identity | untag both, typed op, retag | the quotient of two words is not a word |
| unary `−` | `−8a = 8(−a)` | `0 − (a & −8)` | |

The masking matters for safety. An unmasked tagged add of a pair
(`p | 7`) and the integer 1 (`8`) yields `p + 15`, which has tag `111`: a
forged pointer. With the mask it yields `(p & −8) + 8`, tag `000`, which
is wrong but harmless. The reference form (untag, op, retag) is safe for
the same reason: the retag always produces tag `000`. Detecting the type
error and failing closed is a separate decision (§11, question 3).

### 5.4 Overflow at ±2^60: the decision

**Decision:** Twig integers on the tagged columns are 61-bit fixnums.
**Any operation whose exact mathematical result is outside
`[−2^60, 2^60 − 1]` traps with `Trap.Overflow` (LANG83 id 5).** Boxing an
out-of-range `Raw(i64)` traps the same way. This combines #18487's options
"trap" and "documented 61-bit range"; heap bignums are rejected.

How each operation achieves it, from PR7:

| Operation | Implementation | Traps exactly when |
|---|---|---|
| `repr_box_int x` | `i64_mul_ovf x, 8` inline, or the checked `__dyn_box_int` in the runtime | `8x` overflows i64 ⇔ `x ∉ [−2^60, 2^60−1]` |
| tagged `+` / `−` | `i64_add_ovf` / `i64_sub_ovf` on the masked words | `8(a±b)` overflows i64 ⇔ `a±b` out of range |
| tagged `*` | `i64_mul_ovf (a>>3), (b & −8)` | `8ab` overflows i64 ⇔ `ab` out of range |
| reference form (untag, op, retag) | `i64_*_ovf` on the untagged values, then checked box | the op overflows i64 (out of range a fortiori), or the checked box traps |
| `/` | untag, `div`, checked box | `−2^60 / −1 = 2^60`, out of range |
| out-of-range literal in a `Dyn` position | O2′: `trap Trap.Overflow` where it is used | when control reaches it |

Why this choice:

1. **It is fail-closed.** A wrapped value is a wrong answer, and in a
   representation pass a wrong answer is a memory-safety bug (lesson
   `a-head-only-wrong-answer-in-a-representation-pass-is-a-memory`). A
   documented range *without* a trap still wraps.
2. **Every piece already exists.** LANG77 documents the ±2^60 range.
   LANG83's `dyn_box_int` row says `|n| < 2^60`. `Trap.Overflow` and the
   `i64_*_ovf` family are lowered on every column (LANG83 PR4).
3. **It keeps the invariant cheap.** A bignum makes integers heap objects.
   Equality stops being word equality (§5.1), order stops being one
   compare (§5.2), every arithmetic op can allocate (a GC safepoint in
   every `+`), and the runtime grows a bignum kind, printing and
   exit-code rules. No other column has bignums either: the VM's `Value`
   is an `i64`. Bignums on native would *diverge* from the reference
   rather than match it.
4. **The cost is one overflow branch per op.** On x86-64 the tag-aware add
   is `and`, `and`, `add`, `jo`.

The divergence this leaves is that a program whose exact intermediate
leaves the fixnum range traps on native and LLVM and returns an `i64` on
the VM. The fuzz gate classifies that as expected when `ref.py` in fixnum
mode also says overflow (§7.3). Whether the other columns should adopt the
same range is a language decision for the owner (§11, question 1).

Frontend-typed code (`add [i64]` from Twig's typed fast path) keeps its
declared `i64` semantics. The pass never moves an operation between the
tagged domain and a typed region without the lemma of O4, so the
optimisation level never changes which programs trap.

### 5.5 Truthiness

TW00 says only `#f` and nil are false. So `repr_truthy` is `__lisp_truthy`:
`(x ≠ 3) · (x ≠ 1)` on the whole word.

| Word | Value | Truthy |
|---|---|---|
| 0 | integer 0 | **yes** |
| 1 | nil | no |
| 3 | `#f` | no |
| 5 | `#t` | yes |
| 8 | integer 1 | yes |
| `p \| 7` | a pair | yes |

The generic VM's `Value::is_truthy` treats `Int(0)` as false, which
disagrees with TW00. `ref.py` refuses non-boolean conditions, so the fuzz
gate cannot see this. It is a `vm-core` divergence, recorded in §6 and not
changed here.

---

## 6. Per-column impact

| Column | Value model for Twig | Uses tagged words? | Effect of LANG85 |
|---|---|---|---|
| **NativeAot** (x86-64 Linux / Windows, AArch64 macOS; `twig-aot`) | tagged words + `dynval_runtime.c` | **yes** | The target. `prepare_module_for_aot` switches to `lower_uniform_repr` (PR5). Wrong answers and crashes from mixed representations go away (§8). `dyn` parameters become GC roots. |
| **LLVM** (`lang-aot::compile_iir_to_llvm` → `iir-to-llvm`) | tagged words + `dynval_runtime.c` | **yes** | Same switch, same result. PR8 needs `shl` / `shr` lowering for the inline untag. |
| **JIT** (`lang_matrix` `Jit`: vm-core + `GenericCirJit`) | `vm_core::Value` after the structural heap pass | no for Twig; **yes for McCarthy Lisp** (`jit_lisp::run_mccarthy_on_jit` runs the tagged passes with `dynval-runtime` builtins) | Twig: must accept `dyn` (PR4). McCarthy-on-JIT switches to `lower_uniform_repr` in PR5. Its frontend already declares `ref<any>`, so nothing changes observably. |
| **VM** (vm-core generic VM; also `twig-vm`) | `vm_core::Value` / `LispyValue` inside the runtime | no (not in the IIR) | The reference oracle. Must accept `dyn` (PR4). Known divergences it keeps: `Int(0)` is falsy (§5.5); full `i64` range (§5.4). |
| **WASM** (`iir-to-wasm`) | uniform-anyref (`i31ref`) for "lisp functions", `i64` for scalar ones | no | Unchanged; accepts `dyn` as `any` (PR4). It has the same *kind* of problem (a partition by heuristic, `lisp_functions`), but a mismatch there is a validation error or a trap, not a silent wrong word. |
| **JVM** (`iir-to-jvm-class-file`) | `Integer` / `Object` boxing | no | Same as WASM. #18451 item 3 (record arithmetic) stays open. |
| **CLR** (`iir-to-cil-bytecode`) | boxed `int32` / `object` | no | Same as WASM. A CLR mismatch has crashed with `AccessViolationException` (see `is_tagged_boundary_param` in `lang-aot`), so the managed follow-up (§10) is not cosmetic. |
| **BEAM** (`iir-to-beam`) | native Erlang terms, no boxing | no | Unaffected: every term describes itself. Accepts `dyn` as `any` (PR4). |
| Retro CPUs (`lang-aot` riscv32, armv7, …) | raw words; no `prepare_module_for_aot` | no | Out of scope. A retro column that someday hosts dynamic Twig must run `lower_uniform_repr` or refuse `dyn` (#18487 item 2 covers their `bool` compares). |

---

## 7. Validation

### 7.1 The validator: `validate_uniform_repr`

The validator checks the module after `optimise` and before `expand`, so
it reads abstract ops with unambiguous meaning, not word-level IIR in
which a tag-aware compare and a mistaken raw compare look the same. It is
linear in instruction count apart from the classifier's fixpoint. It runs
in release builds, and a violation fails the compile.

| Rule | Check | It would have rejected |
|---|---|---|
| V1 | Every register has one representation: all writers agree (§3.2). | draft 2 (a reassigned "tagged" parameter returned raw); draft 6 (the representation spread across a `mov` join) |
| V2 | Every `call` argument's representation equals the callee's declared parameter representation; every `call` result equals the declared return. This includes calls inside `__dyn_call_closure`. | drafts 1, 8, 9 (callers passing raw to a parameter other callers pass tagged); draft 9's `(app k 1)` vs `(k 1)` |
| V3 | Every `ret` operand matches the declared return. No immediate is left in a `Dyn` position. | `box_raw_scalar_return`'s double box (`5 << 3 = 40`) |
| V4 | Every operand of a builtin matches its signature row (§4.3). A builtin without a row has no `Dyn` operand. | the `pair?` / `null?` collisions (#18480 rows 4a / 4b); draft 7 |
| V5 | Every typed op (`add`, `cmp_*`, `i64_*_ovf`, `str_len`, …) reads only `Raw` operands of its type. | `dynamic_arith` reading a tagged call result as raw (#18480 comment); draft 4 |
| V6 | Every `dyn_pair_new` operand, record field store, closure-chain entry and `global_store` into a `Dyn` slot is `Dyn`. | draft 5; #18427 (computed integers stored raw) |
| V7 | Every `jmp_if_false` / `jmp_if_true` condition is `Raw(bool)`. | #18480 row 2 (a boxed `#f` branched as true) |
| V8 | `repr_box_*` and `box` only ever read a `Raw` operand. `repr_unbox_int`, `repr_truthy` and `unbox` only ever read a `Dyn` operand. | draft 3 (`p << 3`); every double box |
| V9 | `mov` never crosses representations. Only the conversions in §3.3 do. | the silent cross-representation joins of #18425 |
| V10 | No `Raw(str / f64 / array)` value is in a `Dyn` position. | a string handle read as an integer |
| V11 | The entry function's `ret` is coerced (§4.2.1). | the exit reading `8n` |
| V12 | After `expand`, no `repr_*` op remains, and every `Dyn` register is typed `ref<any>`. | an abstract op reaching a backend; a pointer slot dropped from the stack map |

The validator is also exported, so tests can run it on hand-built IIR.
Each rule has at least one unit test that builds the violating shape and
asserts the rule's name in the error. As with the lesson
`an-expect-a-crash-test-cannot-tell-a-fail-closed-runtime-check`, the test
checks *which* rule fired, not just that something failed.

**Report mode.** `IIR_REPR_VALIDATE=report` makes the old pipeline (before
PR5) run `classify` + `validate` on its *output* and print violations
instead of failing. PR3 uses it to take a census of where main mixes
representations, without changing what main emits.

### 7.2 Optional run-time assertion for fuzzing

A fuzz-only build flag, `TWIG_AOT_REPR_ASSERT=1`, makes `expand` emit
`dyn_unbox_int_checked`, which aborts on a word whose tag is not `000`.
It is a runtime helper, compiled in only when the flag is set. A
well-typed generated program never unboxes a non-integer, so with the
flag on, any silently wrong integer becomes a crash the harness can
count. Release builds never set it.

### 7.3 The differential-fuzz gate

The harness, the generators and the reference interpreter from the nine
review rounds are kept. They become the regression corpus (PR1).

**Engines.** Every program runs on three engines:

1. the VM: `vm-core` after `lower_global_io`, `lower_closures_to_heap`,
   `lower_heap_builtins` and `intern_symbols_structural`. It never runs a
   tagged pass, so it is independent.
2. native built from `origin/main`;
3. native built from the PR head.

LLVM runs too when `clang` is available.

**Oracle.** The expected answer is the VM's. For programs the VM refuses
(`pair?` on an integer, `<` on a parameter: #18470), it is `ref.py`'s.
`ref.py` gains a `--fixnum` mode (§5.4), and programs where the VM and
`ref.py` disagree (`REF!=VM`, 14 of 17,904 in the last run) are triaged by
hand, never dropped.

**Corpora** (all must run; about 46,000 programs plus the fresh seeds):

| Corpus | Source | Programs |
|---|---|---:|
| hand probes: every review repro, `REPROS.txt`, `final.txt`, `final2.txt`, the #18480 table, the call-boxing PR table | `rev8-callbox/`, `rev9-callbox/` | ≈ 170 |
| oldprobes (rev3–rev5) | `rev8-callbox/oldprobes.txt` | 351 |
| `gen.py`, globals first ("all6 fixed") | all six seeds | 17,904 |
| `gen2.py` (dead-branch guards) | seed 880809, plus g2new | 5,600 |
| `gen3.py` (tag-colliding literals, dyn params) | seed 880808 | 6,000 |
| `gen4.py` (boxed / raw joins) | seeds 880810 and 990911 | 4,000 |
| `gen5.py` (call-boundary shapes) | g5big, t1k | 8,000 |
| rev5 `big.txt` | | 3,600 |
| **new per PR**: a fresh seed for every generator | recorded in the PR body | ≥ 5,000 |

**Pass criteria, for every PR from PR3 on:**

- **0 HEAD-only crashes**: HEAD crashes (signal, abort, timeout) where
  main does not. A controlled `Trap.Overflow` counts as a crash unless the
  fixnum oracle predicts it.
- **0 HEAD-only wrong answers**: main equals the oracle and HEAD does not.
- **0 HEAD-only refusals** where main equals the oracle. A `NoDynamicForm`
  refusal of a program main answered correctly is a regression.
- Each HEAD-only result is reduced with `reduce.py`, and before it is
  dismissed it is turned into a crash with the dead-branch guard
  `(if (= W(e) e) A (car 15))` (lesson
  `a-head-only-wrong-answer-in-a-representation-pass-is-a-memory`).

**Reported, not gating:** main's wrong answers HEAD fixes; main's crashes
HEAD fixes; "both wrong, different answers"; the residual list (HEAD ≠
oracle). From PR5 on, the PR must explain every residual class with an
issue link. The target after PR8 is an empty residual outside the classes
§10 lists.

**Generator hygiene** (lesson
`differentially-fuzz-a-representation-pass-against-a-reference-vm-and-the-old`):
globals first; spot-check that the exit value depends on the expression
under test; literals biased to tag collisions (1, 3, 5, 7, 15,
`1229782938247303439`) and to ±2^60 (from PR7).

**CI.** PR1 adds a CI job that runs 2,000 programs from fixed seeds on
every PR touching `iir-builtin-lowering`, `twig-aot`, `iir-to-llvm`,
`twig-ir-compiler` or `lang-aot`. It is informational until PR5 and
blocking from PR5. The full corpus runs nightly. Per-PR authors run the
full corpus locally and paste the summary table into the PR body.

### 7.4 Matrix rows

Every reduced repro above becomes a `lang_matrix` row on NativeAot + LLVM
plus whichever of VM / JIT / WASM / JVM / CLR / BEAM run it. The expected
value comes from the VM, or from `ref.py` when the VM refuses, with
`Expect::Trap` + stderr text for overflow (LANG83 `StdoutThenTrap`).

---

## 8. Migration plan

Each PR keeps main green on every column, runs the §7.3 gate, and lists
what it closes. None removes code while anything still calls it.

| PR | Scope | Gate specifics | Closes |
|---|---|---|---|
| **PR0** | This spec, plus a cross-link in LANG77. | doc checks | — |
| **PR1** | Bring the fuzz harness into the repo: a Rust harness crate under `code/programs/rust/` plus the generators, `reduce.py`, `ref.py` (with `--fixnum`) and `cmp8.py` under `code/scripts/`, each with BUILD / README / CHANGELOG. Add the CI job (informational). Record main's baseline numbers in the PR. | The harness reproduces the §1.3 numbers on main ± noise. | — |
| **PR2** | Prerequisites land, unchanged: #18489 (signed compares, #18483) and #18454 (join-type consensus, #18448). #18452 (fresh registers, #18439) is **recommended**: §3.2 does not need it for correctness, but it enlarges typed regions. | their own gates | #18483, #18448 (#18439) |
| **PR3** | `uniform_repr::{signatures, classify, validate}` in **report mode** only, plus unit tests for V1–V12. Nothing in any pipeline changes. Publish the violation census of main over the corpora. | Emitted objects are **byte-identical** to main (hash every binary in the corpus). | — |
| **PR4** | `twig-ir-compiler` emits `dyn` (§4.4). Every consumer that accepts bare `any` accepts `dyn` with the same meaning, including the *old* tagged passes. So this PR is a rename for every column. Touches `twig-ir-compiler` and possibly `interpreter-ir` and `vm-core`, which are in `twig-vm`'s graph: run `code/scripts/miri-twig-vm.sh`. | `lang_matrix` byte-identical on every column; native and LLVM binaries hash-identical to main across the corpus. | — |
| **PR5a** | `lower_uniform_repr` (classify, convert, validate in enforce mode, reference-form expand) wired into **LLVM** (`compile_iir_to_llvm`) and McCarthy-on-JIT. `ClosureRepr::Uniform`. No optimisations. Overflow behaviour stays as main's (wrapping). | Full gate on the LLVM column. Every `lang_matrix` row of every language on LLVM stays green. | — (staging) |
| **PR5b** | The same switch for **NativeAot** (`prepare_module_for_aot`). | full gate on native | #18425, #18427, #18451 items 1–2 and its closure-argument comment, #18480 (all rows and the ninth-review comment), the call-boxing table (`(h 1)` = 9, `(h (h 1))`, recursive `loop`, the identity-return SIGSEGV) |
| **PR6** | Records, unions and `match` through checked `dyn_pair_*` on both tagged columns (§4.2.2). The `field_load` on a `match` scrutinee is checked. | gate plus new rows: an integer, nil and a cons passed to a record accessor | #18441 item 1 and its `match` comment; item 4 becomes true by construction |
| **PR7** | Overflow (§5.4): checked `__dyn_box_int` (C and `dynval-runtime`, golden test updated), `dyn_box_int` registry `may_trap`, `i64_*_ovf` in the expansion, O2′. Rows on both sides of ±2^60. Touches `interpreter-ir` and `dynval-runtime`: run the Miri script. | gate with the fixnum oracle | #18487 item 1 |
| **PR8** | Optimisations O1–O5 and the tag-aware expansions (§4.2.5), each its own commit with lemma and unit tests; `shl` / `shr` on `iir-to-llvm` if used. | gate, plus §8.1 performance | — |
| **PR9** | Deletions (§4.5): `lower_dyn_repr`'s old body, `box_raw_scalar_return`, `wrap_tagged_conditions`, `tagged_boundary_functions`, `lower_box_unbox_to_runtime_calls` (if it has no callers), the Legacy-only branches on tagged paths. | binaries hash-identical to PR8 | — |

PR5 cannot be split further by *construct*, only by column. A column is
either uniform or it is not; making calls uniform but not cells is the
incremental approach of §9. Splitting LLVM (5a) from native (5b) keeps
each diff reviewable without making any column half-uniform.

### 8.1 Performance expectations

| Path | main | after PR5 | after PR8 |
|---|---|---|---|
| dynamic `(+ a b)`, both `Dyn` | 2 `__dyn_unbox_int` calls + `add` + 1 `__dyn_box_int` call | same shape | 2 `and` + `add` + `jo`, inline |
| dynamic `(< a b)` | 2 unbox calls + `cmp` | same | 1 `cmp` |
| literal into a cell or a `Dyn` parameter | compile-time `n << 3` | same (O2 is a fold, not an optimisation) | same |
| typed-region arithmetic (`string-length` results etc.) | raw | raw | raw |
| branch on a comparison | raw | `box_bool` + truthy | raw (O1a / O5) |
| closure call | per argument: `box` on store, `unbox` in the dispatcher | no conversion for `Dyn` arguments | same |
| GC | `any` parameters not in the stack map | `dyn` parameters are roots (more slots scanned) | same |

Expectations: PR5 within ±10% of main on three microbenchmarks
(`fib 25`, summing a 10,000-element list, a closure-heavy `map`), measured
on native and LLVM and reported in the PR. PR8 at least as fast as main,
and expected faster wherever main paid runtime calls for box and unbox.
The extra compile time is one classification fixpoint per function
(bounded by one demotion per register) plus one validator pass, and should
not be measurable on the `lang_matrix` suite. A PR outside these bounds
explains why or does not land.

---

## 9. Background: why the incremental approach failed

In October 2026, nine drafts of
`fix(iir-builtin-lowering): values cross tagged-word call boundaries…`
(planned 0.43.0) each passed their author's tests, and every one failed
security review. None was pushed.

| Draft | Change | How review broke it |
|---|---|---|
| 1 | Callees unbox every bare-`any` parameter; callers box every argument that looks raw. | Identity functions returned pointers that were then boxed; captured pointers were unboxed: wild reads. |
| 2 | Facts computed once, assuming parameter registers are never reassigned. | Twig's `let` reuses the parameter's register (#18439). A reassigned "tagged" parameter was returned raw: SIGSEGV in `pair?`. |
| 3 | Skip the closure dispatcher's `unbox` for cons-only lambdas. | The call site's `box` stayed, so a cell pointer arrived as `p << 3`: SIGSEGV in `car`, and the GC lost the referent. |
| 4 | Box call arguments for mixed-use parameters. | `(if (= x 15) x (car l))`: the boxed 15 compared unequal with a raw 15, so the dead `car` ran on an integer. |
| 5 | Box computed values into cons cells; read cell results as tagged. | Fixed #18427, but other writers still stored raw. A guarded generator turned the resulting wrong answers into 181 crashes. |
| 6 | Relabel call results `ref<any>` early in `lower_dynamic_arith`. | The label spread through the bidirectional `mov` fixpoint to a raw literal on the other arm, which was boxed after its reader had been lowered raw. `(+ (if (= 1 2) (f 1 #t) 5) 1)` gave 41; a boxed `#f` read as true (`REPROS.txt`). |
| 7 | Box provably raw values for `pair?` / `null?`. | `(pair? 7)` became correctly false, and exposed a raw cell read that main had answered right by accident. |
| 8 | Prove-only: `raw_only_params`, `tagged_at_every_site`, `is_reassigned`, computed once by `CallBoundary::analyse`. Fuzzed on 37,614 programs: one HEAD-only crash, caused by #18483. | It needed #18483 first: a now-correct `-1` met an unsigned `<`. |
| 9 | Draft 8 restacked on the #18483 fix (#18489). Fuzzed on 41,766 programs: clean. | The ninth review's fresh generator (`gen5.py`) found 20 HEAD-only crashes and 15 HEAD-only wrong answers in 7,000 programs. The reduced set (`final.txt`, `final2.txt`) is below. |

Draft 9's reduced failures. Every one is right on main:

| Program | VM / main | Draft 9 |
|---|---|---|
| `(define (k y) (+ y 1)) (define (h y) (if (= y (k 0)) 10 20)) (h 1)` | 10 | 20 |
| … with `(car 5)` in place of `20` | 10 | SIGSEGV |
| `(define (k y) (+ y 1)) (define (h y) (k 0) (if (= y nil) 10 20)) (h 1)` | 20 | 10 (raw 1 equals `TAG_NIL`) |
| `(define (k y) (+ y 1)) (define (app f v) (f v)) (if (= (k 1) (app k 1)) 1 0)` | 1 | 0 |
| `(define (k y) (+ y 1)) (define (f a b) (k 0) (if (= a b) a (car 5))) (f 13 13)` | 13 | SIGSEGV |
| `(define (r1 n acc) (if (< n 1) acc (r1 (- n 1) (* acc 2)))) (let ((v (< (r1 3 5) 42))) v)` | 1 | 0 |

**The pattern.** Every draft converted some values to the right
representation at some boundaries. Each conversion was locally correct.
But main is correct only where its errors cancel: a raw store read raw, a
tagged result compared with a tagged result. Fixing one side of a
cancelling pair exposes the other side. And because any wrong answer can
be guarded into a dereference, each exposure is a memory-safety
regression, not merely a wrong number. With the representations of calls,
joins, cells, globals, captures and comparisons all coupled, there was
no proper subset that could be made correct without breaking a
coincidence somewhere else.

**What the drafts lacked**, and LANG85 supplies:

1. One owner: drafts changed two passes that each decided representation.
2. Declared types as the only source of a representation (§2.1): drafts
   inferred it from uses.
3. All boundaries at once, per column (§8, PR5).
4. A machine-checked invariant (§7.1): drafts were checked only by fuzzing
   and review, which find counterexamples one at a time.
5. A fuzz gate set up before the first review (§7.3, PR1).

The lessons `a-call-boundary-needs-one-representation-that-caller-and-callee-both-read`,
`a-head-only-wrong-answer-in-a-representation-pass-is-a-memory` and
`differentially-fuzz-a-representation-pass-against-a-reference-vm-and-the-old`
record the same history from the implementer's side. They live on the
`call-boxing` branch and land with PR1.

---

## 10. Out of scope, and the follow-ups it leaves

| Item | Why it is not here | Tracking |
|---|---|---|
| A dynamic form for strings (`string-length` on an untyped value) | needs a heap kind and a tag decision (§11, question 4) | #18428 |
| The managed columns (WASM / JVM / CLR) reading declared types instead of `lisp_functions` | different value model; mismatches fail loudly there; needs its own spec | #18451 item 3, #18425 (`producer_types`) |
| O6, the unboxed worker/wrapper calling convention | an optimisation; needs PR9's clean base | new issue |
| Failing closed on `(+ pair 1)` and other dynamic type errors | needs a trap kind (§11, question 3) | new issue |
| Printing a non-integer dynamic value with `print` | a print rule, like `__lisp_exit_code` | new issue |
| `vm-core` truthiness of `Int(0)`, and its `i64` range | VM-side divergences | new issues |
| Retro-CPU columns | they do not run `prepare_module_for_aot` | #18487 item 2 |
| VM error text with heap addresses, core files in `lang_matrix` | unrelated hygiene | #18441 items 2–3 |

---

## 11. Open questions for the owner

1. **Should 61-bit fixnums be Twig's integer semantics on every column?**
   §5.4 makes native and LLVM trap outside ±2^60. The VM, JIT, WASM, JVM
   and CLR keep `i64`. Making all columns trap (frontend-emitted checks,
   per LANG83 PR7) gives full parity but changes the VM's answers for
   large values.
2. **Is `dyn` the right spelling for Twig's dynamic values, or `ref<any>`?**
   `ref<any>` would unify Twig with McCarthy Lisp, but it would also move
   Twig onto the managed columns' uniform-anyref model at the same time.
   `dyn` (already the registry's operand type) keeps PR4 a no-op outside
   the tagged columns.
3. **Should dynamic arithmetic fail closed on a non-integer operand?** It
   needs a trap kind. LANG83's closed table has none for a type error
   (`Trap.Type`, id 8, would be new). Without it, `(+ pair 1)` yields a
   harmless but wrong integer, as on main.
4. **Strings:** a reserved tag (`100` or `110`) for string handles, or
   tag `111` with a GC-header kind check in `pair?`?
5. **May the fuzz harness and generators live in the repo** (PR1)? They
   were kept out deliberately until now. The gate in §7.3 needs them to be
   reproducible by anyone reviewing PR3–PR9.
6. **What is `=` on two different kinds?** Word equality (§5.1) says
   `(= 0 #f)` is false. The VM may coerce `#f` to 0 and say true. One of
   the two is the language's answer, and the oracle (`ref.py` and the VM)
   should agree with it before PR5.
