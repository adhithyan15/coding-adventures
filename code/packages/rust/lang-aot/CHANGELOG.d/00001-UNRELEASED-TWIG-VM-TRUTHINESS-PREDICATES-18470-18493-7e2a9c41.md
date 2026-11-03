## Unreleased — Twig's value rules on the VM and JIT columns (#18470, #18493)

The `lang_matrix` VM and JIT harness (`lower_dynamic_for_generic_engine`) now
runs `iir-builtin-lowering::lower_lisp_for_vm` on every Twig module, after the
structural heap passes. It writes TW00 truthiness before every branch, `not`,
`number?`, `symbol?`, `boolean?`, `list?`, a `null?` that is `#f` for 0, and
the dynamic division names, over the five `dyn_*` primitives `vm-core` now
lowers; nil becomes `dyn_nil` and a symbol `dyn_symbol`, so neither is an
integer on the VM any more. It runs only for Twig: it rewrites every branch,
and a C-like frontend's branch on 0 must still jump. `vm_gc_reclamation.rs`'s
copy of the harness runs it too. A `Value::Null` result (Twig's nil, or a
void `main`) now exits 0 (`vm_exit_code`), as nil does on the tagged-word
columns.

Seventeen Twig rows, each on Vm and Jit; sixteen failed on both before (the
`(= 0 #f)` row is a pin — the VM already answered `#f`):

| row | exit | also declared on |
|---|---|---|
| `(if 0 1 2)` | 1 | |
| `(define (f x) (if x 1 2)) (f 0)` | 1 | |
| `(if "" 1 2)` | 1 | |
| `(define (f x) (if (null? x) 1 2)) (f 0)` | 2 | NativeAot, LLVM, WASM, JVM |
| `(define (f x y) (if (= x y) 1 2)) (f 0 #f)` | 2 | |
| `(if (car (cons #f (list))) 1 2)` | 2 | |
| `(define (k b) (lambda () (if b 1 2))) ((k #f))` | 2 | NativeAot, LLVM, WASM |
| `(define (f x) (if (= x (quote a)) 1 2)) (f 536870912)` | 2 | NativeAot, LLVM |
| `not` over 0, `#f`, nil | 6 | |
| `number?` over 5, `'a`, `#t`, nil | 1 | |
| `symbol?` over `'a`, 5, `#f`, nil | 1 | |
| `boolean?` over `#t`, `#f`, 0, nil | 3 | |
| `list?` over `(1 2)`, nil, `(1 . 2)`, 5 | 3 | |
| `(define (f x y) (/ x y)) (f 17 5)` | 3 | NativeAot, LLVM, WASM, JVM |
| `(define (f x y) (quotient x y)) (f -17 5)` | 253 | NativeAot, LLVM, WASM |
| `(define (f x y) (remainder x y)) (f -17 5)` | 254 | WASM |
| `modulo` of −17 by 5 and 17 by −5 | 45 | |

A row is declared on another column only where it was run there and passed;
the rows' comments record the columns that give a wrong answer today (raw 0,
`(= 0 #f)`, a stored `#f`, `remainder` / `modulo` on NativeAot and LLVM,
`modulo` on WASM). None is declared on CLR or BEAM (no toolchain where they
were added). `twig_truthiness_of_a_raw_word_matches_the_runtime_rule_it_replaced`
adds `(f 0)` → 1, which the VM answered 2 before. `feature_coverage_doc_counts_match_programs_source`
and `LANG-VM-FEATURE-COVERAGE.md` move Twig from 60 rows / 456 cells to
77 / 507.

`op_conformance`: `pr7f_dynamic_values_run_on_the_tagged_word_backends`
expects the VM to lower the five kind primitives and refuse the other five;
the `dyn_symbol` case, refused by every compiled backend, now runs on the VM
(`dyn_tag` of a symbol is 2).

**JIT promotion (security review M1).** New
`twig_dynamic_values_agree_between_the_vm_and_a_promoted_jit` runs
`(define (id x) x) (if (id V) 1 2)` and `(… (id V))` for `#f`, `#t`, nil, a
symbol, `0` and `""` three times on one `JITCore` with promotion thresholds
of 1, and requires every run to equal the plain VM's answer. Before the
jit-core fix the second run of `(if (id #f) 1 2)` answered 1 (the compiled
`id` returned `#f` as `Int(0)`, true under TW00) and nil / symbol arguments
failed the call; `GenericCirJit` now leaves a function with an `any`
signature on the interpreter tier.

No Twig function changes JIT tier in the corpus: `execute_with_jit` compiled none of the
Twig rows' functions before this change and compiles none after (every one
has an `any`- or `void`-typed instruction).

- Matrix: `assert_cell` compares a Windows exit code as the byte POSIX would
  report (`posix_exit_status`). A Windows process keeps all 32 bits, so the new
  `(quotient -17 5)` row exited `-3` there instead of `253`. Only `-255..=-1` is
  narrowed; NTSTATUS crash codes such as `0xC0000005` are never folded into a
  small exit value.
