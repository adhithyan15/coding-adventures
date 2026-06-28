# LANG-FULL E4 — Strings (design spec)

**Status:** IR + reference VM slice implemented. BASIC BA4 literal/scalar
strings, reassignment, equality/inequality, lexical ordering, copied-slot
equality, literal and variable-backed concat, expression concat in `PRINT`/`IF`,
and multi-item string `PRINT` with `;` and `,` run on all seven backends,
including `PRINT A$ + B$` over two scalar string slots. ALGOL AL4 literal output,
`output`, multi-argument `output`, scalar variables, scalar copies, copy
snapshots, and literal-backed string equality/ordering predicates run on all
seven backends. Twig literal, immutable top-level, lexical-local, and direct
top-level-function string-op proofs run on all seven backends, including
`str_concat` and `str_slice` feeding `str_index`, lexical ordering predicates,
and a function-wrapped `string-length` direct call. Captured/dynamic strings,
arrays/input/parameters, and fuller backend byte-string representations remain.
**Enabler:** E4 in [`LANG-FULL-IMPLEMENTATION.md`](LANG-FULL-IMPLEMENTATION.md).
**Unlocks:** Dartmouth BASIC strings + string `PRINT` (BA4), ALGOL 60 strings +
`print`/`output` I/O (AL4), Twig strings on the code-gen backends (TW4), and any
future language with text values.

---

## 1. Goal

A **representation-agnostic string primitive** in the shared
`interpreter_ir::IIRModule`, lowered to *every* backend (vm-core, jit-core,
iir-to-llvm, iir-to-wasm, iir-to-jvm-class-file, iir-to-cil-bytecode,
x86_64-backend, aarch64-backend), and **verified by RUNNING** a real program on
each that produces observable text output.

### 1.1 What a "string" is in v1 (the scope decision)

A v1 string is an **immutable, length-counted sequence of bytes** (UTF-8 /
ASCII). Concretely:

- **Bytes, not codepoints.** Indexing returns the *byte* at a position; length is
  the *byte* count. No Unicode-codepoint, grapheme, or normalisation semantics —
  those are an explicit follow-up. This matches what the source languages of this
  era actually had (ALGOL/BASIC strings are byte/character sequences) and keeps
  every backend's representation a flat byte buffer.
- **Immutable.** String operations (`str_concat`, …) produce *new* strings;
  there is no in-place mutation. Immutability removes aliasing hazards, lets the
  managed backends use their native interned `String` types directly, and lets
  the static backends keep literals in read-only memory.
- **A string is morally an `array<u8>` + identity.** It reuses the E5 array
  substrate (length-prefixed flat buffer on the static backends; a managed array
  / native `String` on the managed backends). E4 is therefore **not** a brand-new
  allocator — it is the E5 byte-aggregate plus a literal pool, a few text ops, and
  a print primitive.

### 1.2 The dual-mode requirement (the design driver)

As with E3 (`f64`) and E5 (arrays), the IIR must **not** bake in a representation,
because the toolchain serves both target families:

- **Static-allocation / unmanaged** (C, ALGOL, BASIC, the native + native-AOT
  backends, LLVM, wasm-linear): a string is a **length-prefixed byte buffer** —
  literals live in read-only data; runtime-built strings live in heap memory the
  program owns. This is exactly E5's unmanaged array header (`[i64 len][bytes…]`).
- **Garbage-collected / managed** (the JVM, the CLR, WasmGC): a string is a
  **native managed object** — `java/lang/String`, `System.String`, or a WasmGC
  `(array i8)` — that carries its own length and is reclaimed by the runtime.

The IIR expresses string *operations* abstractly; each backend picks the
representation that is natural and safe for its target.

---

## 2. IIR surface

Eight string ops. A string value rides the `type_hint` `str`; it flows as a `Var`
(a managed reference or a fat handle), exactly like an `array<T>` value.

| Op | Form | Result | Semantics |
|---|---|---|---|
| `str_const` | `dest <- "literal"` | `str` | Materialise a compile-time string constant. The bytes ride as an `Operand::Str` **value** (see §2.1). `dest` is the string value. |
| `str_len` | `dest <- s` | `i64` | The **byte** length of `s`. |
| `str_concat` | `dest <- a, b` | `str` | A new string = bytes of `a` followed by bytes of `b`. Neither input is mutated. |
| `str_index` | `dest <- s, idx` | `i64` | **Bounds-checked** byte load: if `idx < 0 \|\| idx >= str_len(s)` → **trap**; else `dest` = the unsigned byte value `s[idx]` (0–255). |
| `str_slice` | `dest <- s, start, end` | `str` | **Bounds-checked** byte slice `[start,end)`: invalid ranges trap; valid ranges produce a new immutable string. |
| `str_eq` | `dest <- a, b` | `i64` (bool) | `1` if `a` and `b` have identical bytes, else `0`. |
| `str_cmp` | `dest <- a, b` | `i64` | Lexicographic byte ordering: `-1` if `a < b`, `0` if equal, `1` if `a > b`. |
| `print_str` | `s` | — | Write the bytes of `s` to stdout (no implicit newline). The text-I/O primitive — the string sibling of `call_builtin "print_i64"`. |

Broader dynamic allocation, captured/reassigned strings, and richer frontend
string APIs remain follow-ups beyond this literal/known-value foothold.

### 2.1 Type model

- New `type_hint` `str` (a scalar-form hint string, like `f64` or `ref<…>`).
  Helpers `is_str_type` live beside the existing `ref<…>`/`array<…>` helpers in
  `interpreter-ir/src/opcodes.rs`.
- **No new `Operand` variant.** A string *literal* reuses the existing
  `Operand::Str(String)` — but as a **value** carried by `str_const`, distinct
  from its existing use as a compile-time *name* in `global_load`/`global_store`.
  The two are disambiguated **by opcode** (exactly as the spec note on
  `Operand::Str` already anticipates: "used by the global ops and by any future
  string-value opcode"). A string *value* in flight is a `Var`.
- **Encoding:** the literal bytes are the source bytes after escape processing
  (`\n`, `\t`, `\"`, `\\`, `\0`). The IIR stores raw bytes; it does not interpret
  them as codepoints.

### 2.2 Bounds-check + trap convention

`str_index` is **bounds-checked by definition** (mirroring E5 `array_get`): the
check is `0 <= idx < str_len(s)`; on violation the program **traps**, reusing each
backend's existing hard-trap path (the same table as E5 — `ud2`/`udf`/`unreachable`
/ native exception / `VMError`). On the managed backends the native
`charAt`/indexer bounds check is reused (free); the static backends emit an
explicit `cmp idx,len` + branch-to-trap.

---

## 3. Per-backend representation

| Backend | Family | Representation | `str_const` | `str_len` | `str_eq` | `str_cmp` | `str_index` | `str_concat` | `str_slice` | `print_str` |
|---|---|---|---|---|---|---|---|---|---|---|
| **vm-core** | (interp) | `Value::Str(Vec<u8>)` (new value variant) or a handle into `memory` | intern the literal bytes | `.len()` | byte equality | byte ordering | range-checked byte index | allocate a new buffer | allocate a sliced buffer | write bytes to the host stdout sink |
| **jit-core** | (interp) | same as vm-core (CIR mirrors it) | — | — | — | — | — | — | — | — |
| **JVM** | GC | `java.lang.String` for the landed literal-output/metadata/index slice; byte-string representation still under E4-managed | `ldc` string CP entry ✅ for ASCII literals | `String.length()` ✅ for ASCII literals | `String.equals(Object)` ✅ for ASCII literals | `String.compareTo` + `Integer.signum` ✅ | `String.charAt(I)` ✅ for ASCII literals | `String.concat(String)` ✅ for ASCII literals | `String.substring(II)` ✅ for ASCII literals | `PrintStream.print(String)` ✅ |
| **CLR** | GC | `System.String` for the landed literal-output/metadata/index slice; byte-string representation still under E4-managed | `ldstr "…"` ✅ for ASCII literals | `String.Length` ✅ for ASCII literals | `String.Equals(string,string)` ✅ for ASCII literals | `String.CompareOrdinal` + `Math.Sign` ✅ | `String.get_Chars(int32)` ✅ for ASCII literals | `String.Concat(string,string)` ✅ for ASCII literals | `String.Substring(int32,int32)` ✅ for ASCII literals | `Console.Write(string)` ✅ |
| **WASM** | linear-memory foothold now; WasmGC later | `i32` pointer into a data segment for the landed literal-output/metadata/index slice; richer byte-string representation still under E4-managed | data segment ✅ for ASCII literals | literal side-table length ✅ | literal side-table byte equality ✅ | literal side-table ordering ✅ | guarded `i32.load8_u` ✅ for ASCII literals | literal data entry ✅ | literal side-table slice ✅ | host import `env.__print_str(ptr,len)` ✅ |
| **LLVM** | static | length-prefixed buffer `[i64 len][bytes…]`; literals in a `private constant` global | private `{len,bytes}` global ✅ for ASCII literals | literal side-table length ✅ | literal side-table byte equality ✅ | literal side-table ordering ✅ | folded literal byte ✅ | literal metadata ✅ | literal metadata slice ✅ | `@__print_str(i8* base+8, i64 len)` C-runtime ✅ |
| **x86_64** | static | heap-byte literal-output foothold now; full length-prefixed rodata model later | `alloc_bytes` + `store_byte` ✅ for ASCII literals | folded literal length ✅ | folded literal equality ✅ | folded literal ordering ✅ | folded literal byte ✅ | folded literal concat ✅ | folded literal slice ✅ | `call __twig_print_string(ptr,len)` ✅ |
| **aarch64** | static | heap-byte literal-output foothold now; full length-prefixed rodata model later | `alloc_bytes` + `store_byte` ✅ for ASCII literals | folded literal length ✅ | folded literal equality ✅ | folded literal ordering ✅ | folded literal byte ✅ | folded literal concat ✅ | folded literal slice ✅ | `bl __twig_print_string(ptr,len)` ✅ |

**Unmanaged header layout** (LLVM / x86_64 / aarch64): identical to E5's array
header — word 0 is the byte count, bytes start at offset 8. String literals are
emitted once into read-only data with that header. The landed LLVM literal-only
slice folds direct-literal `str_concat` into a derived read-only constant so
`print_str` can consume the concat result; richer dynamic concat allocation
remains future E4-managed/static work. **No new allocator** — E4 reuses E5's.

**Managed backends** (JVM/CLR/WASM): JVM/CLR currently use native `String`
constant loads (`ldc`/`ldstr`) for the literal-output/metadata/index slice;
because the landed proof is printable ASCII, managed character length/indexing
equals E4 byte length/indexing. WASM currently uses a linear-memory data segment,
side-table metadata, and guarded byte loads; a richer managed `(array i8)`/WasmGC
representation remains the follow-up for non-literal byte-string ops.

**The print runtime** (`print_str`): the static backends share one
`__print_str(const char* base_plus_8, long len)` C runtime (the string sibling of
the existing `__print_i64`), compiled into the LLVM/native matrix harness exactly
as `PRINT_RUNTIME_C` is today. WASM imports `env.__print_str(ptr,len)` (the sibling
of the existing `env.__print_i64`). The landed JVM/CLR literal-output footholds
write through `PrintStream.print(String)` / `Console.Write(string)` directly; a
broader `printStr` host surface remains a representation decision for the rest of
E4. This is the one genuinely new piece of host surface E4 adds beyond E5.

---

## 4. Frontend drivers

- **Dartmouth BASIC (BA4)** — `PRINT "HELLO"` already *parses* (the grammar has a
  `STRING` `print_item`); the frontend now lowers a `STRING` print-item to
  `str_const` + `print_str` on all seven backends. `$` string variables now
  tokenize/parse as `NAME`s, and `LET A$ = "HI"; PRINT A$` lowers to a safe E4
  string slot plus `print_str`; `LET A$ = "NO"; LET A$ = "OK"; PRINT A$`
  proves literal reassignment through that same slot, and
  `LET A$ = "O" + "K"; PRINT A$` proves literal `str_concat`. `LET A$ = "OK";
  LET B$ = A$; PRINT B$` proves scalar string copy by lowering through
  `str_concat` with an empty suffix, and `IF B$ = A$ THEN ...` proves copied-slot
  equality through `str_eq`. `PRINT A$; B$` proves ordered tight multi-item string
  output without concat or numeric formatting helpers, while `PRINT A$, B$`
  proves BA2's comma separator (`putchar(' ')`) composes with the same ordered
  `print_str` calls. `LET A$ = "O"; PRINT A$ + "K"` proves `PRINT` can consume a
  temporary E4 string-expression result, and `LET B$ = "K"; PRINT A$ + B$`
  proves both concat operands can be scalar string slots in that direct-print path.
  `IF A$ + "K" = "OK" THEN ...` proves the same temporary expression path before
  `str_eq`, while `IF A$ + B$ = "OK" THEN ...` and
  `IF A$ + B$ <> "NO" THEN ...` prove variable-variable concat on the `=` and
  `<>` branch paths through `str_eq` plus `jmp_if_true` / `jmp_if_false`.
  `LET B$ = A$ + "K"; PRINT B$` proves variable-backed concat
  assignment into another scalar string slot. String compares in `IF A$ = "Y"` and
  `IF A$ <> "Y"` lower to `str_eq` (the latter branches with `jmp_if_false`),
  while `IF A$ < "B"` / `IF "B" > A$` lower through `str_cmp` plus typed zero
  comparisons. These paths now drive line-control branching on all seven
  backends; string arrays, string `INPUT`, captured/dynamic string storage, and
  broader runtime byte-string operations remain follow-ups.
- **ALGOL 60 (AL4)** — `string` is already a `type` keyword; `algol-parser`
  produces string literals. The current foothold recognises undeclared
  statement-position `print`/`output` calls and lowers string literal actuals to
  `str_const` + `print_str`. Literal-backed scalar string variables now lower
  `s := 'HI'` to a direct `str_const` slot consumed by `print(s)`, and
  literal-backed scalar copies lower `t := s` through `str_concat` with an empty
  suffix. Reassigning the source after a copy leaves the target printable as a
  snapshot, matching the immutable E4 value model. `output(s, t)` over two
  literal-backed scalar strings preserves actual order through two E4
  `print_str` calls. Literal-backed scalar string predicates lower through E4
  too: `s = 'OK'` / `s != 'NO'` use `str_eq` plus typed zero comparisons, while
  `s < 'BETA'` / `'BETA' > s` use `str_cmp` plus typed zero comparisons before
  the normal ALGOL conditional branch. Captured/`own` strings, arrays,
  parameters, and broader dynamic strings remain follow-ups.
- **Twig (TW4)** — Twig string literals + `print` lower to `str_const` +
  `print_str`; `++`/`string-append` to `str_concat`, `string=?` to `str_eq`,
  `string<?`/`string>?` to `str_cmp` plus typed comparisons, and `string-ref`
  to `str_index`. Direct literals, immutable top-level values, and
  lexical `let`/`let*` locals can now feed `str_len`, `str_index`, `str_eq`, `str_cmp`, and
  `str_concat` on all seven backends; local `substring` lowers to `str_slice`,
  and local `string-append` results can also feed
  `string-ref` directly through `str_concat` followed by `str_index`, and
  local `string-length` results can compute `string-ref` indexes through typed
  arithmetic. Direct top-level functions whose body ends in one of those typed
  E4 string-op results now preserve the concrete return type through a later
  direct `call`; `(define (strlen) (string-length "HELLO")) (strlen)` returns `5`
  on all seven backends. The dynamic-`any`, captured, reassigned, and
  parameter-derived Twig string paths still need broader E6/dynamic representation
  work; the *typed* string slice here is the statically-typed subset that clears
  the code-gen validators, mirroring how E5/E6 carved a typed slice out of Twig.

The frontends emit `str` values and the shared E4 string ops; no backend learns anything
language-specific.

---

## 5. Verification (the matrix proof)

Unlike the integer/array proofs (observable via exit code), strings are observable
via **stdout** — the same channel the Dartmouth BASIC `PRINT 42` and Oct `out`
cells already use in `lang-aot/tests/lang_matrix.rs` (`Expect::Stdout`). The first
proof:

```basic
10 PRINT "HELLO"
```

⇒ stdout `HELLO` on every backend the toolchain is present for. The landed Twig
footholds also prove direct literal `str_len`, `str_index`, `str_eq`, `str_cmp`, and
`str_concat`-feeding-`str_len` via exit codes `5`/`66`/`1`/`42`/`5`. The named-value
proofs exercise immutable top-level string values with `str_concat` + `str_len`,
`str_eq` driving an `if`, and `str_index` via exit codes `5`/`42`/`67` on every
backend. Lexical string locals now run through indexing, `let*` length, equality
branches, and concat: `(let ((s "ABC") (i 2)) (string-ref s i))` returns `67`,
`(let* ((s "HELLO")) (string-length s))` returns `5`,
`(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))` returns `42`, and
`(let ((a "AB") (b "CDE")) (string-length (string-append a b)))` returns `5`
everywhere; `(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))`
returns `68`, proving a concat temporary can feed byte indexing, and
`(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))` returns `69`,
proving `str_len` can compute a byte-index operand.
`(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))` returns `67`,
proving `str_slice` can feed byte indexing. `(if (string<? "ALPHA"
"BETA") (if (string>? "BETA" "ALPHA") 42 0) 0)` returns `42`, proving lexical
ordering through `str_cmp`. `(define (strlen) (string-length "HELLO")) (strlen)`
returns `5`, proving a direct top-level function can wrap a typed E4 string op
and propagate its `i64` return through the caller's `call`. The matrix also
covers the **bounds-trap** case: `(string-ref "ABC"
3)` must fail closed on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
Dartmouth BASIC proves source-language string variables, reassignment, scalar
copy, copied-slot equality, literal/variable-backed concat, concat expressions in
`PRINT`/`IF` including `PRINT A$ + B$`, equality/inequality branches, and multi-item string `PRINT` with
both `;` and `,` on all seven backends. ALGOL proves literal output, the
`output` alias, multi-argument `output`, scalar string variables, scalar copies,
copy snapshots, and literal-backed scalar string predicates on the same
all-seven E4 path:

```algol
begin string s; s := 'ALPHA';
  if (s = 'ALPHA' and s != 'OMEGA') and
     (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD')
end
```

The program writes `OK` through native-AOT + LLVM + WASM + JVM + CLR + VM +
JIT.
Follow-up proofs now focus on string arrays/input/parameters, captured or
reassigned dynamic strings, and runtime byte-string operations beyond the current
immutable scalar/local subset.

`run_native` runs the host arch, so `NativeAot` exercises aarch64 locally and
x86_64 on CI (as for E3/E5). The `x86-simulator` harness can additionally run the
x86_64 string output locally.

---

## 6. PR breakdown (incremental — one concern per PR)

E4 is large, so it ships as a sequence, each a `feat(lang-full): …` PR babysat to
merge before the next:

0. **E4-spec** (this document) — committed specs-first, for design sign-off.
1. ✅ **E4-ir + vm** — define the string ops + the `str` type helper in
   `interpreter-ir`; implement them in `vm-core`: a string value model,
   `str_len`/`str_index` (bounds-checked)/`str_concat`/`str_eq`, and `print_str`
   to the host sink. Unit tests incl. an out-of-bounds trap. *No matrix Prog yet
   (needs a frontend), but a direct IIR unit test proves it runs.* The generic CIR
   JIT remains i64-only and cold-interprets/declines string-shaped functions
   until a string-capable tier is added.
2. ✅ **E4-basic-frontend (all-7 literal-output proof)** — `dartmouth-basic-iir-compiler` lowers
   `PRINT "…"` to `str_const` + `print_str`; matrix `Prog` (`PRINT "HELLO"` ⇒
   stdout `HELLO`) runs on native-AOT + VM + JIT + LLVM + WASM + JVM + CLR. The
   native/LLVM/WASM/JVM/CLR slices are deliberately literal-output footholds
   (native heap-byte `alloc_bytes` + `store_byte` + `print_string`, LLVM private `{len,bytes}` global,
   WASM data segment +
   `env.__print_str(ptr,len)`, JVM/CLR `ldc`/`ldstr` + `PrintStream.print`/
   `Console.Write(string)`).
2a. ✅ **BA4-string-variable proof** — `coding-adventures-dartmouth-basic-lexer`
   tokenizes `$`-suffixed names as one `NAME`, the parser accepts `STRING` as a
   primary expression, and `dartmouth-basic-iir-compiler` lowers `LET A$ = "HI"`
   into a safe typed string slot consumed by `PRINT A$`. Matrix `Prog` returns
   stdout `HI` on native-AOT + VM + JIT + LLVM + WASM + JVM + CLR. Literal
   reassignment, literal concat assignment, scalar copy, variable-backed concat
   assignment, concat expressions in `PRINT`/`IF` including `PRINT A$ + B$`,
   expression-backed equality branches,
   expression-backed inequality branches,
   lexical string ordering branches,
   copied-slot equality, tight multi-item string `PRINT` (`PRINT A$; B$`), and
   comma-separated string `PRINT` (`PRINT A$, B$` => `O K`) all now run on the
   same seven backends. String arrays, string `INPUT`, captured/dynamic storage,
   and broader runtime byte-string operations remain follow-ups.
2b. ✅ **AL4-literal-output proof** — `algol-iir-compiler` recognises
   undeclared statement-position `print`/`output` calls and lowers string literal
   actuals to E4 `str_const` + `print_str`. Matrix `Prog`
   `begin print('HI') end` returns stdout `HI` on native-AOT + VM + JIT + LLVM +
   WASM + JVM + CLR. Scalar string variables are the follow-up proof below;
   broader dynamic strings remain follow-up work.
2c. ✅ **AL4-string-variable proof** — `algol-iir-compiler` accepts scalar
   `string` declarations when assigned from a literal, materialising the variable
   slot with E4 `str_const`; `print(s)` is accepted only for literal-backed
   slots. Matrix `Prog` `begin string s; s := 'HI'; print(s) end` returns stdout
   `HI` on native-AOT + VM + JIT + LLVM + WASM + JVM + CLR, and the sibling
   `output(s)` matrix proof returns `OK` through the same E4 path. The
   multi-argument proof `begin string s, t; s := 'O'; t := 'K'; output(s, t) end`
   also returns `OK` through ordered `print_str` calls. The scalar
   copy proof `begin string s, t; s := 'OK'; t := s; print(t) end` and the copy
   snapshot proof `begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end`
   now return stdout `OK` on the same seven backends. Captured/`own` strings,
   string arrays, string parameters, and broader dynamic strings remain
   follow-ups.
3. ✅ **E4-literal-metadata/index proofs** — Twig lowers literal
   `(string-length "HELLO")`, `(string-ref "ABC" 1)`,
   `(string=? "HELLO" "HELLO")`, and
   `(string-length (string-append "AB" "CDE"))` to shared `str_const` +
   `str_len`/`str_index`/`str_eq`/`str_concat`; matrix `Prog`s return `5`, `66`,
   `1`, and `5` on native-AOT + VM + JIT + LLVM + WASM + JVM + CLR. This is
   deliberately still a direct-literal foothold: native and LLVM fold to integer
   consts, WASM uses literal data plus a guarded byte load, and JVM/CLR use
   managed `String` metadata/index/equality/concat for printable ASCII.
3a. ✅ **E4-named-value ops proofs** — immutable top-level Twig string value
   defines stay in `main` as typed `str_const` registers, so named values can
   feed `str_concat`+`str_len`, `str_eq` in an `if`, and `str_index`; matrix
   `Prog`s return `5`, `42`, and `67` on native-AOT + VM + JIT + LLVM + WASM +
   JVM + CLR.
3b. ✅ **E4-lexical-local proof** — Twig `let`/`let*` string literal bindings
   materialise directly as typed `str_const` registers, and known local string
   and integer registers can feed E4 ops. Matrix `Prog`
   `(let ((s "ABC") (i 2)) (string-ref s i))` returns `67`, and
   `(let* ((s "HELLO")) (string-length s))` plus
   `(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))` return `5`/`42`, while
   `(let ((a "AB") (b "CDE")) (string-length (string-append a b)))` returns `5`,
   and `(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))`
   returns `68`, and
   `(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))` returns `69`,
   `(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))` returns `67`,
   and `(define (strlen) (string-length "HELLO")) (strlen)` returns `5`
   through a typed direct `call [i64]`, on native-AOT + VM + JIT + LLVM + WASM +
   JVM + CLR. Captured/reassigned/parameter-derived strings still wait for the
   broader dynamic representation.
4. **E4-managed-backends** — richer WASM/JVM/CLR byte-string ops once their
   representations own UTF-8 byte semantics. (May be one PR per backend if they
   diverge.)
5. **E4-static-backends** — full native x86_64 + aarch64 byte-string ops
   (length-prefixed rodata literals + heap `str_concat` + explicit `str_index`
   guard). Native literal output is already proven through the heap-byte foothold;
   this item is now the richer ops/representation slice.
6. ✅ **E4-ops-proofs** — named-value `str_concat`+`str_len`, `str_eq` driving a
   branch, named/local `str_index`, local `str_concat` feeding `str_index`, and
   local `str_len` computing a `str_index` operand, `str_slice` feeding
   `str_index`, `str_cmp` driving lexical predicates, direct top-level
   function-call return typing for E4 string ops, plus the `str_index`
   out-of-bounds **trap** proof now run across every backend.
7. **Follow-ups beyond v1** captured/reassigned dynamic strings,
   string arrays/input/parameters in each
   frontend, runtime byte-string allocation beyond the current immutable scalar
   foothold, Unicode codepoint/grapheme semantics, the dynamic-`any` Twig string
   path (needs broader E6), and string interpolation.

Ordering rationale mirrors E5: get the IIR + reference interpreter right (1),
prove it end-to-end through the simplest frontend (2), extend source-language
frontends while staying inside the immutable typed slice (2a-3b), and only then
take on the wider managed/static byte-string representations and dynamic
front-end features. Front-loads the cheap high-confidence wins.

---

## 7. Open questions / decisions for review

1. **Bytes vs. codepoints in v1** — proposal: **bytes** (UTF-8/ASCII; `str_index`
   returns a byte 0–255, `str_len` is the byte count). Unicode codepoint/grapheme
   semantics are a follow-up. *OK?*
2. **Immutability** — proposal: strings are **immutable**; `str_concat` allocates a
   new string. This lets the managed backends use interned `String` directly and
   keeps literals in read-only memory. A mutable `StringBuilder`-style buffer is a
   separate later primitive. *OK?*
3. **`Operand::Str` reuse for literals** — proposal: carry a string *literal* in
   the existing `Operand::Str`, disambiguated from its name use by opcode
   (`str_const` value vs `global_load` name). Alternative: a new `Operand::Bytes`
   variant. Reuse is simpler and the operand already documents this intent.
   *Reuse preferred — confirm?*
4. **The `print_str` host runtime** — proposal: add a `__print_str(base,len)` C
   runtime (static backends), an `env.__print_str` import (WASM), and a
   `BasicRuntime.printStr` (JVM/CLR) — the string siblings of the existing
   `__print_i64` surface. This is the one new host primitive. *OK?*
5. **First frontend driver** — proposal: Dartmouth BASIC `PRINT "…"` (BA4), since
   its grammar already produces the `STRING` print-item and the frontend already
   has a clean error site to replace. ALGOL `string`/`output` (AL4) and Twig (TW4)
   follow. *OK, or prefer ALGOL first?*
6. **Trap vs. recoverable error on OOB `str_index`** — proposal: a hard **trap**
   (matches E5 arrays + the managed runtimes' native behaviour). *OK?*
7. **Concat growth on the static backends** — proposal: `str_concat` always
   allocates a fresh `8 + len_a + len_b` block (no small-string optimisation, no
   in-place growth, since strings are immutable). *OK?*

---

*This spec is the E4 contract; each ☐ in the PR breakdown becomes a
`feat(lang-full): …` PR, and divergences are called out in the implementing PR's
commit message and folded back here. It deliberately mirrors
[`lang-full-e5-arrays.md`](lang-full-e5-arrays.md): strings are the byte-aggregate
sibling of arrays, reusing the same dual-mode allocator, header, trap, and matrix
methodology.*
