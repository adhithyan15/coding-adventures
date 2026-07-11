# lang-aot

Multi-language AOT driver — compile **Twig, Nib, Brainfuck, Dartmouth
BASIC, Oct, and McCarthy Lisp** to native executables through the shared
LANG VM chain.

> **LANG-FULL E4 — BASIC, ALGOL, and Twig string footholds run on all 7 backends (v0.133.0):**
> `tests/lang_matrix.rs` now proves `10 PRINT "HELLO"` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
> It also proves `10 LET A$ = "HI"; 20 PRINT A$` produces stdout `HI` on
> the same seven backends, and `LET A$ = "NO"; LET A$ = "OK"; PRINT A$`
> produces `OK`, proving literal reassignment through the same E4 slot.
> BASIC literal string concatenation now runs too: `LET A$ = "O" + "K"; PRINT A$`
> reaches E4 `str_concat` and prints `OK` everywhere.
> Literal-backed scalar string copy also runs everywhere:
> `LET A$ = "OK"; LET B$ = A$; PRINT B$` lowers through `str_concat` with an
> empty suffix.
> Multi-item string `PRINT` runs too:
> `LET A$ = "O"; LET B$ = "K"; PRINT A$; B$` prints `OK` through ordered
> `print_str` calls.
> Comma-separated string `PRINT` composes BA2 separators with BA4 strings:
> `LET A$ = "O"; LET B$ = "K"; PRINT A$, B$` prints `O K` by placing
> `putchar(' ')` between the same ordered `print_str` calls.
> `PRINT A$ + "K"` now proves `PRINT` can consume a temporary E4 string
> expression result directly.
> `IF A$ + "K" = "OK" THEN ...` proves the same expression path can feed
> `str_eq` before line-control branching.
> `LET B$ = A$ + "K"; PRINT B$` proves a variable-backed concat can assign into
> another scalar string slot.
> BASIC string equality now drives control flow too:
> `IF A$ = "Y" THEN ...` routes to `PRINT "OK"` through E4 `str_eq` on every
> backend, and `IF A$ <> "Y" THEN ...` proves the inequality branch by reusing
> `str_eq` with `jmp_if_false`.
> Copied string slots now feed control flow as well:
> `LET A$ = "OK"; LET B$ = A$; IF B$ = A$ THEN ...` prints `OK` everywhere.
> ALGOL 60 now proves `begin print('HI') end`, lowering the implementation-defined
> `print`/`output` output procedures to the same E4 `str_const` + `print_str`
> path and producing stdout `HI` everywhere.
> The next ALGOL row proves literal-backed scalar string variables too:
> `begin string s; s := 'HI'; print(s) end` produces stdout `HI` everywhere,
> and `begin string s; s := 'OK'; output(s) end` proves the `output` alias;
> `begin string s, t; s := 'O'; t := 'K'; output(s, t) end` proves
> multi-argument output preserves ordered scalar string actuals;
> `begin string s, t; s := 'OK'; t := s; print(t) end` now proves
> literal-backed scalar string copy, and `s := 'NO'` after the copy leaves
> `print(t)` at `OK`; captured strings and broader dynamic string storage remain
> follow-up work.
> It also proves Twig `(string-length "HELLO")` returns exit code `5` and
> `(string-ref "ABC" 1)` returns exit code `66`,
> `(string=? "HELLO" "HELLO")` returns exit code `1`, plus
> `(string-length (string-append "AB" "CDE"))` returns exit code `5`, on the same seven backends.
> Named immutable top-level Twig strings now feed the same ops:
> `(define a "AB") (define b "CDE") (string-length (string-append a b))` returns `5`,
> `(define s "HELLO") (if (string=? s "HELLO") 42 0)` returns `42`, and
> `(define s "ABC") (string-ref s 2)` returns `67` everywhere.
> Lexical Twig string locals now feed E4 too:
> `(let ((s "ABC") (i 2)) (string-ref s i))` returns `67` everywhere.
> Sequential lexical Twig string locals also run:
> `(let* ((s "HELLO")) (string-length s))` returns `5` everywhere.
> Lexical local strings can drive equality branches too:
> `(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))` returns `42`
> everywhere.
> Local string concat now runs on the same path:
> `(let ((a "AB") (b "CDE")) (string-length (string-append a b)))` returns
> `5` everywhere.
> Local concat results can feed byte indexing too:
> `(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))`
> returns `68` (`D`) everywhere.
> Local string lengths can compute byte indexes too:
> `(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))` returns
> `69` (`E`) everywhere.
> The matrix also proves the bounds contract: `(string-ref "ABC" 3)` traps on
> native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
> Native AOT lowers the literal to `alloc_bytes` + `store_byte` + `print_string`;
> LLVM emits a private `{len,bytes}` constant and calls `@__print_str(payload,len)`;
> WASM stores literal bytes in linear memory and calls `env.__print_str(ptr,len)`;
> JVM maps `str_const` to `ldc` + `CONSTANT_String` and `print_str` to
> `PrintStream.print(String)`; CLR maps them to `ldstr` and `Console.Write(string)`.
> Literal length/index/equality/append metadata is folded/read from metadata on
> native/LLVM/WASM and uses host string APIs on JVM/CLR; the OOB index row proves
> each backend fails closed. Dynamic byte-string ops, broader dynamic string
> copies, and captured string variables stay follow-up E4/BA4 slices.

> **LANG-FULL O2 — Oct bitwise `~` + u8 wrap on all 7 backends (v0.92.0):**
> `tests/lang_matrix.rs` adds `out(1, ~0)` → `255` and `out(1, 200 + 100)` → `44` (wrap).
> `oct-iir-compiler` 0.7.0 emits the `u8` hint on arithmetic/bitwise/`~` (Oct's only integer
> type is u8); `iir-to-jvm-class-file` 0.14.0 masks a narrow op on the JVM **long** model
> (`i2l; land`) since Oct's printing programs keep the i64 model — the int `iand` was
> unverifiable over longs. Completes Oct O2.

> **LANG-FULL N3 — Nib bitwise `~` runs on all 7 backends (v0.91.0):**
> `tests/lang_matrix.rs` adds two executed `~` programs — `~0u8 == 255` and `~15u4 == 0`.
> `nib-iir-compiler` 0.16.0 lowers unary `~` to the IIR `not` op (it had been silently
> dropped) with the narrow width so every backend masks it mod-2ⁿ; `iir-to-cil-bytecode`
> 0.21.0 adds the unary `not` arm to its textual `.il` emitter — the last backend that
> couldn't assemble `~` on CoreCLR. Completes Nib N3 (`& | ^ ~`).

> **LANG-FULL B1-stdin — Brainfuck reads real input on all 7 backends (v0.90.0):**
> `tests/lang_matrix.rs` adds two executed stdin programs — `,+.` (read a byte, `+`,
> print: `"A"` → `"B"`) and `,.,.` (echo two bytes: `"Hi"` → `"Hi"`). The four
> subprocess columns (native/LLVM/JVM/CLR) read real process stdin via the new
> `output_with_stdin` helper; WASM/VM/JIT `getchar` drains a per-program `program_stdin`
> buffer. Harness-only — every backend already compiled `,`→`getchar`. Both programs
> read exactly the supplied bytes (no EOF-gated loop), so they terminate identically
> despite the backends' divergent `getchar`-EOF convention (0 vs -1); normalising EOF
> (so `,[.,]` cat works) is a separate item.

> **LANG-MATRIX Brainfuck — CODE-GEN MATRIX COMPLETE (v0.65.0–v0.68.0):** Brainfuck now
> runs on **every code-gen backend** — LLVM, WASM, JVM, **and CLR** (plus native).
> `lower_brainfuck_for_aot` widens the frontend's narrow cell (`u8`) and pointer (`u32`)
> hints to `i64` — byte width survives only at the `load_byte`/`store_byte` tape boundary
> — and each backend grew the matching byte-tape ops: `iir-to-llvm` 0.9.0
> (`@calloc`/`getelementptr i8`/libc `putchar`), `iir-to-wasm` 0.13.0 (linear-memory
> `i32.load8_u`/`i32.store8`, `env.putchar`/`env.getchar` imports), `iir-to-jvm-class-file`
> 0.11.0 (static `byte[] __tape` via `baload`/`bastore`, `env.BFRuntime`), and
> `iir-to-cil-bytecode` 0.18.0 (`unsigned int8[]` via `newarr Byte`/`ldelem.u1`/`stelem.i1`,
> `Console::Write(char)`). Verified by RUNNING `++++++++[>++++++++<-]>+.` → `A` on real
> `clang`, the in-repo `wasm-runtime`, real `java`, and real `dotnet` (`tests/lang_matrix.rs`).
> The frontend itself is unchanged, so the `vm-core`/`jit-core` Brainfuck paths (which key
> CIR widths off `u8`/`u32`) keep working.

> **LANG-MATRIX — PLATFORM MATRIX COMPLETE (v0.71.0):** every language runs on every
> backend except BEAM, **verified by running**. The two **execution columns** are both
> generic over the shared IIR, so a future Ruby/JS frontend runs on them unchanged:
> - **VM** (`v0.69.0`–`v0.70.0`): `run_vm` does `compile_source_to_iir` → `vm_core::VMCore`,
>   the general register VM — no per-language code. The byte-tape ops live in `vm-core`
>   0.4.0, over its flat `memory`.
> - **JIT** (`v0.71.0`): `run_jit` does `compile_source_to_iir` → `jit_core::JITCore` +
>   the language-agnostic `GenericCirJit`. `execute_with_jit` compiles fully-typed
>   functions to JIT bytecode and interprets the rest, so each program runs *through the
>   JIT pipeline*. (`jit-core` 0.4.0 made compiled functions bind their parameters, so
>   Nib's `double(21)` JITs correctly.)
>
> Both run **all six languages** in-process — Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2,
> BASIC→`42`, Brainfuck→`A` — the I/O languages via registered `print_i64`/`putchar`
> builtin closures.
> (McCarthy Lisp is wired as of L3a — scalar programs run
end-to-end natively; symbol/cons backend support is L3b.)  As of L3b-3a-3c,
`compile_source_to_wasm` also compiles McCarthy **cons** programs to a runnable
WasmGC module — `(CAR (CONS 7 9))` → `7` on the in-repo `wasm-runtime` (integer
atoms boxed as `i31ref`, the cons cell a `$LispyPair` struct) — and the McCarthy
predicates, `COND`, and symbols: `pair?`/`ATOM` (`(ATOM 5)` → 1; L3b-3a-4b),
`EQ` atom equality (`(EQ 5 5)` → 1; L3b-3a-4c), `COND` with lisp-truthiness
(`(COND (0 7) (5 9))` → 7 — `0` is truthy; L3b-3a-4d), and symbols
(`(EQ 'A 'A)` → T, `(EQ 'A 'B)` → nil; W1), and `LAMBDA`/`LABEL`/recursion
(`((LAMBDA (X) X) 5)` → 5; a recursive `LABEL` walks a list; W2). The **full
McCarthy core** — cons, `ATOM`, `EQ`, `COND`, symbols, and lambda/label/recursion
(F1–F7) — now runs on the wasm backend.

`compile_source_to_jvm` is the second managed target: scalar McCarthy runs on the
in-repo `jvm-simulator` (`42` → 42; W3a), **cons** runs on a real JVM —
`(CAR (CONS 7 9))` → 7 (W3b) — and now `ATOM`/`EQ`/`COND` too: `(ATOM 5)` → 1,
`(EQ 5 5)` → 1, `(COND ((EQ 1 1) 7) (5 9))` → 7 (W4). It runs the *same* structural
passes as the wasm path; the JVM backend lowers the backend-agnostic
`box`/`unbox`/`alloc`/`field_*` + `pair?`/`not`/`equal?` to `Integer`/`Object[]` +
`instanceof`/`ixor`/`if_icmpeq` (where wasm uses `i31ref`/`$LispyPair`/`ref.test`).
**Symbols** run too (W5a): `(EQ 'X 'X)` → 1 — their interned ids (`2²⁹`) load via
the JVM `ldc` constant-pool path. And `LAMBDA`/`LABEL`/recursion (W5b):
`((LAMBDA (X) X) 5)` → 5, a recursive `LABEL` → 99. **The JVM backend is now
McCarthy-complete (F1–F7)** — the second managed backend after WASM.

`compile_source_to_cil_artifact` is the third managed target: scalar McCarthy
emits CIL that **runs** on the in-repo `clr-simulator` (`42` → 42; W6a), and
**cons** too — `(CAR (CONS 7 9))` → 7 (W6b, after the simulator gained an
object/reference value model). It runs the *same* structural passes as the
wasm/JVM paths; the CLR backend lowers the backend-agnostic
`box`/`unbox`/`alloc`/`field_*` to `box [int32]`/`unbox.any` + `object[]` cells.
McCarthy's predicates run too (W7, F3–F5): `(ATOM 7)`→1, `(EQ 7 7)`→1,
`(COND …)` — `pair?`→`isinst object[]`, `not`→`x^1`, `equal?`→`unbox; unbox; ceq`.
**Symbols** (W8a, F6) run with *zero new backend code*: the shared
`intern_symbols_structural` pass interns each symbol to an `i32` id
(`SYMBOL_ID_BASE = 1<<29`), so `(QUOTE A)`→536870912 and `(EQ (QUOTE A) (QUOTE A))`
→1 fall out of W6b boxing + W7 `equal?`. **Lambda** (W8b, F7) **completes the CLR
backend (F1–F7):** `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7 — the lambda is hoisted to
its own method, applied via `call <MethodDef>`, and run on `clr-simulator` 0.4.0's
inter-method call-frame model. The CLR is the third managed backend to reach full
McCarthy support after WASM and JVM.

`compile_source_to_cil_text` is the **real-CoreCLR** path (CLR-real C1): the same
lowered program emitted as textual `.il`, assembled by real `ilasm` into a loadable
PE, and run on real `dotnet` — the CLR analog of `compile_source_to_llvm` (`.ll` →
real `clang`). The full McCarthy **F1–F7** set runs today — scalar, **cons/car/cdr**,
**predicates + `COND`**, **symbols**, and **lambda / LABEL / recursion** (`42`→42,
`(CAR (CONS 7 9))`→7, `(ATOM 7)`→1, `(EQ 7 7)`→1, `(COND ((ATOM 7) 11) …)`→11,
`(EQ (QUOTE A) (QUOTE A))`→1, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, and a recursive
`LABEL`→7 on real CoreCLR; `tests/clr_real_scalar.rs` + `clr_real_cons.rs` +
`clr_real_predicates.rs` + `clr_real_symbols.rs` + `clr_real_lambda.rs`, gated on
`dotnet`+`ilasm` via the shared `tests/clr_support` harness). The W16 conformance
suite (`tests/conformance.rs`) now carries a ninth **`CLR-real`** column that runs
the same programs on real .NET (19 programs × 9 backends all agree locally), so the
CLR column is verified on real CoreCLR rather than only the in-repo simulator — the
**CLR-real verification chapter (C1–C6) is complete**.

`compile_source_to_beam` targets the **Erlang VM**: scalar McCarthy emits a
`.beam` that **runs** on a real `erl` (OTP), `42` → 42 (W9a), and **cons** too —
`(CAR (CONS 7 9))` → 7, `(CONS 7 9)` → `[7|9]` (W9b). BEAM uses the **native
Erlang-terms** value model, NOT the boxing structural pass: a cons cell is a
native list cell `[H|T]`, `car`/`cdr` are `hd`/`tl`, integers are native.
`lower_heap_builtins` produces `alloc`/`field_*` that `iir-to-beam` maps to
`put_list`/`get_hd`/`get_tl`. Its **predicates** run too (W10, F3–F5):
`(ATOM 7)`→1, `(EQ 7 7)`→1, `(COND …)` — `pair?`→`is_nonempty_list`,
`equal?`→`is_eq_exact` (`=:=`), `not`→`x==0`. **Symbols + lambda** (W11, F6–F7)
**complete the BEAM backend (F1–F7):** `(QUOTE A)`→536870912 (the shared
`intern_symbols_structural` id, same as the other backends), and
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7 — lambda needed no BEAM-specific work (a
`(LAMBDA …)` application is a method `call`, already lowered as a BEAM fun). The
BEAM is the fifth backend to reach full McCarthy support after VM, WASM, JVM, CLR.

`compile_source_to_llvm` targets **LLVM IR** — the first **tagged-word** backend
(the LLVM/AOT/JIT family that links the shared `lispy_runtime.c`). Scalar McCarthy
emits LLVM IR that **runs**: the test emits host-triple IR (`clang -dumpmachine`),
builds it with `clang -x ir`, and runs the native executable — its exit code is
the result, `42` → 42 (W12a). It uses the `clang` already on the box (no
`lli`/`qemu`), the LLVM analogue of `wasm-runtime`/the `clr-simulator`/real `erl`.
**Cons** runs too (W12b-1, F2): the native lisp pipeline
(`lower_heap_builtins_runtime`→`intern_symbols`→`lower_lisp_repr`) lowers cons/car/cdr
to `call @__twig_lispy_*`, and the test **links `lispy_runtime.c`** into the
clang-built executable — `(CAR (CONS 7 9))` → 7, `(CDR (CONS 7 9))` → 9.
**Predicates** run too (W12b-2, F3–F4): a predicate returns a *tagged* boolean, so
the shared `lower_lisp_repr` coerces a boolean program result with `lispy_truthy`
(→ 0/1) rather than `lispy_unbox_int` — `(ATOM 7)` → 1, `(EQ 7 7)` → 1,
`(EQ 7 8)` → 0. **`COND`** runs too (W12b-3, F5): a clause-result variable assigned
across blocks is promoted to a stack slot (`alloca`/`store`/`load`, a cross-block SSA
merge) — `(COND ((ATOM 7) 11) ((ATOM 8) 22))` → 11, nested `COND` → 44.
**Symbols** run too (W13a, F6): an interned symbol is a tagged `i64` immediate, and a
bare symbol result is returned verbatim (not unboxed) — `(EQ (QUOTE A) (QUOTE A))` →
1, `(EQ (QUOTE A) (QUOTE B))` → 0, `(ATOM (QUOTE A))` → 1.
**Lambda** runs too (W13b, F7): an integer atom argument is boxed before crossing
into the lambda, and the polymorphic result is coerced at the program exit by
`__twig_lispy_to_exit_code` (a runtime tag switch) — `((LAMBDA (X) X) 5)` → 5,
`((LAMBDA (X) (CAR X)) (CONS 7 9))` → 7, `((LAMBDA (X Y) (EQ X Y)) 3 3)` → 1,
lambda-with-`COND`-body → 100/200. **LLVM is now McCarthy-complete (F1–F7)** — the
sixth backend to finish, after VM/WASM/JVM/CLR/BEAM. **Native AOT** completed too
(W14): the macOS Mach-O runtime-link gap closed (W14a — external symbols now carry
the leading-`_` C decoration), and lambda runs via one `lispy_to_exit_code` row in
each native backend's `V1_BUILTINS` (W14b) — the seventh backend.

`run_mccarthy_on_jit(source)` runs McCarthy on the **universal JIT** —
`jit-core::GenericCirJit`, the **eighth and final backend** (W15, **F1–F7**). The JIT
dispatches `call_builtin "lispy_*"` to Rust callbacks backed by the shared
`dynval-runtime` crate (the C runtime's Rust twin), with a `LispyValue` riding inside
the VM's `Value::Int` as its bit pattern — `(CAR (CONS 7 9))` → 7, `(ATOM 7)` → 1,
nested `COND` → 44, `(EQ (QUOTE A) (QUOTE A))` → 1, `((LAMBDA (X) X) 5)` → 5, and a
recursive `LABEL` → 7. **With the JIT, McCarthy 1960 LISP now runs on every LANG VM
backend (F1–F7): VM, native AOT, JIT, WASM, JVM, CLR, BEAM, LLVM.**

`tests/conformance.rs` (W16) is the capstone: one shared table of **19** McCarthy
programs (F1–F7) run through **all eight backends**, each asserting the identical
integer result. The four in-process backends (VM/JIT/WASM/CLR) always run; JVM
(`java`), BEAM (`erl`), LLVM (`clang`), and native AOT (`ld`) skip gracefully when
their tool is absent. One source, eight code generators, three value models
(tagged-word / uniform-anyref / object-boxing / Erlang-terms), **one answer** — the
proof the platform is complete and uniform.

`tests/lang_matrix.rs` generalizes that idea from McCarthy to **every** language
(`LANG-PLATFORM-MATRIX`): a per-language program battery run through each non-BEAM
backend, asserted by running. The **native-AOT** column is uniformly green — all six
non-Lisp languages (Twig, Nib, Brainfuck, Dartmouth BASIC, Oct, ALGOL 60) compile to a
host executable and produce the right result (exit code for the expression languages;
stdout for the I/O languages). The **LLVM** column is green for Twig / Nib / Oct /
ALGOL 60 (exit code) and Dartmouth BASIC (stdout, via a generic `__print_i64`
runtime); Brainfuck is deferred. The **WASM** column (in-process `wasm-runtime`) is
green for Twig / Nib / Oct / ALGOL 60 (exit code) and Dartmouth BASIC (stdout — a tiny
`PrintHost` resolves the `env.__print_i64` import and captures the printed value);
Brainfuck (tape ops) is the only WASM follow-up. The **JVM** column runs on **real
`java`** (the W16 wrapper-launcher pattern) and is green for Twig / Nib / Oct / ALGOL 60
(exit code) and Dartmouth BASIC (stdout — `run_jvm` compiles an `env.BasicRuntime` host
class with `javac` and discards the entry result); Brainfuck (tape ops) is the only JVM
follow-up. The **CLR** column runs on the **real CoreCLR** (textual `.il` → `ilasm` →
`dotnet`, the CLR-real path) and is green for Twig / Nib / Oct / ALGOL 60 (exit code —
this needed `iir-to-cil-bytecode` to grow integer arithmetic + comparison opcodes) and
Dartmouth BASIC (stdout — `print_i64` → `Console.WriteLine(int32)` with an I/O-aware
launcher that discards the entry result), and Brainfuck (tape ops, v0.68.0) — the CLR
column is complete. The **VM** and **JIT** columns are generic register interpreters/JITs
over the shared IIR (`vm_core::VMCore`, `jit_core::JITCore` + `GenericCirJit`) and now run
all six languages too — **every column except BEAM is complete.**

## Stack position

```
<lang> source
   │
   ▼ <lang>-iir-compiler                  (per-language frontend)
interpreter_ir::IIRModule                  ← lingua franca
   │
   ▼ twig_aot::compile_module_to_*_executable
       (x86_64-backend / aarch64-backend  →  elf/pe/macho_object  →
        system linker)
   │
   ▼
native executable
```

The point of this crate is the **dispatch layer** at the top: pick the
right frontend based on the input file's extension or an explicit
`--lang` flag, then hand the resulting `IIRModule` to twig-aot for the
rest of the chain.

Besides native executables, `lang-aot` also exposes text/bytecode emit
pipelines — `compile_file_to_llvm_ir`, `compile_file_to_riscv32_bin`, …, and
(LANG77 / McCarthy L3b-3a) **`compile_source_to_wasm` / `compile_file_to_wasm`**.
The wasm path currently handles **scalar** programs (a polymorphic lisp
`"any"` value is concretised to `i64` for functions with no heap ops); a
scalar McCarthy `42` emits a `.wasm` whose `main` returns `i64 42`, verified by
running it on the in-repo `wasm-runtime`. Cons/symbol programs (the
boxed-`anyref` WasmGC value model) are a follow-up.

## CLI

```text
lang-aot <FILE> [-o <OUT>] [--lang <LANG>]
```

| Language | Extensions | Frontend crate | Status |
|---|---|---|---|
| Twig            | `.twig`         | `twig-ir-compiler`       | full |
| Nib             | `.nib`          | `nib-iir-compiler`       | full |
| Brainfuck       | `.bf`, `.b`    | `brainfuck-iir-compiler` + BF07 lowering pass | full — `lang-aot foo.bf` compiles end-to-end (cells live in a 30000-byte `alloc_bytes` tape; `load_mem`/`store_mem` are rewritten to `load_byte`/`store_byte` per LANG76) |
| Dartmouth BASIC | `.bas`, `.basic` | `dartmouth-basic-iir-compiler` | full integer subset — LET / PRINT / INPUT / IF / GOTO / FOR / NEXT / END / REM, GOSUB / RETURN, DEF FN, DIM arrays, and READ / DATA / RESTORE compile and run end-to-end through the matrix. Strings, floating point, and `^` remain LANG-FULL follow-ups |
| Oct             | `.oct`          | `oct-iir-compiler` (OCT02 phases 1–4) | full — integer subset compiles end-to-end (`fn`/`let`/`if`/`while`/`loop`/`break`, recursion).  8008 hardware intrinsics (`in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`) rejected cleanly with a pointer to the dedicated Intel-8008 simulator backend |
| McCarthy Lisp   | `.mcl`, `.lisp` | `mccarthy-lisp-iir-compiler` | **L3a** — the full Lisp 1.0 frontend (literals, `QUOTE`, `CONS`/`CAR`/`CDR`/`ATOM`/`EQ`, `COND`, `LAMBDA`/`LABEL` closures) produces an `IIRModule`, and **scalar** programs run end-to-end on the native AOT pipeline (`echo 42 > p.mcl; lang-aot p.mcl` → exits 42).  Symbol/cons-returning programs (e.g. `(CAR '(A B C))`) are accepted by the frontend but the native backend `BackendRefused`s them until the `dynval-runtime` value model is lowered into each backend (**L3b**) |

If `--lang` is omitted the language is inferred from the file
extension; unknown extensions get a "could not infer language" error
listing the recognised ones.

## Example

```bash
$ echo 'fn main() -> u8 { return 42; }' > hello.nib
$ lang-aot hello.nib
$ ./hello
$ echo $?
42
```

That ran a Nib source file all the way to a native executable on the
host — via the same `x86_64-backend` and `elf_object` / `pe_object` /
`macho_object` packagers Twig uses.

## Adding a new language

1. Build a `<lang>-iir-compiler` crate whose `compile_source(&str, &str)
   -> Result<interpreter_ir::IIRModule, _>` mirrors `nib-iir-compiler`
   or `brainfuck-iir-compiler`.
2. Add a variant to the [`Language`] enum and wire
   `compile_source_to_iir` to call your new frontend.
3. Add the file extension to `detect_language_from_path`.
4. Add a smoke test in `tests/end_to_end_smoke.rs` with a small program
   that compiles to a known exit code.

No backend changes needed — every frontend gets x86-64 Linux, x86-64
Windows, and ARM64 macOS for free.

## Limitations

- **Host-targets-host only.** Same V1 policy as `twig-aot`.  Use the
  `twig-aot --target= --emit-object` workflow for cross-OS object
  emission.
- **BF backend gap.** `brainfuck-iir-compiler` emits IR ops
  (`load_mem`, `store_mem`, `putchar`, `getchar`, …) that the x86_64
  and aarch64 AOT backends don't lower today.  The dispatch layer here
  correctly produces the IIR, but compilation to executable fails at
  the backend step.  Extending the backends to support these ops is a
  separate follow-up.
- **No `--target` / `--emit-object` flags yet.** `lang-aot`'s CLI is
  intentionally minimal in V1.  Cross-OS support will land alongside
  multi-language `--emit-object` once we have a story for cross-host
  runtime archives.
