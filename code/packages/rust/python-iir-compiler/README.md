# Python IIR compiler

This crate holds two Python frontends that share nothing but the crate:

| Path | Entry points | Subset | Grammar |
|------|--------------|--------|---------|
| LANG84 release path (phase 7a) | `compile_source_at`, `compile_cst`, `run_source_at` | `py-int-print`: `print` of integer arithmetic, on all nine backends | one tagged CPython release's `python<release>.*` pair |
| LANG79 float pilot | `compile_source`, `compile_ast`, `run_source` | float expressions and bindings, on `vm-core` | the legacy minor line `"3.12"` |

## The release path: `py-int-print` (LANG84 phase 7a)

```rust
use python_iir_compiler::{compile_source_at, run_source_at, FrontendBudgets};

// None = DEFAULT_RELEASE ("3.13.16", the host python3); or an exact
// releases.csv ID such as Some("3.0a1").
let module = compile_source_at("print(-7 // 2)\n", None, "demo", &FrontendBudgets::default()).unwrap();
assert_eq!(module.functions[0].return_type, "i64");

let ran = run_source_at("print(1)\nprint(9223372036854775807 + 1)\n", Some("3.12.3")).unwrap();
assert_eq!(ran.stdout, "1\n");
assert_eq!(ran.trap.as_deref(), Some("Trap.Overflow"));
```

The pipeline is LANG84's:

```text
source ─► python-parser::parse_python_at (GrammarLexer + GrammarParser over
          python<release>.tokens/.grammar) ─► GrammarASTNode ─► int_lower.rs
       ─► IIRModule of LANG83 registry ops ─► x86_64 | aarch64 | LLVM | WASM |
          JVM | CLR | BEAM | vm-core | JIT
```

What `py-int-print` accepts: statements `print(<expr>)` (one positional
argument, optional trailing comma) and `print()`, separated by newlines or
`;`. Expressions are decimal `int` literals (with `_` separators where the
release lexes them), parentheses, unary `+`/`-`, and `+ - * // %` at Python's
precedence. The lowering:

| Python | IIR |
|--------|-----|
| `a + b`, `a - b`, `a * b`, `-a` | `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`, `i64_neg_ovf` |
| `a // b`, `a % b` | `i64_div_floor`, `i64_mod_floor` |
| `a // -1`, `a % -1` (divisor known) | `i64_neg_ovf a`, `const 0` |
| `+a` | nothing |
| `print(a)` / `print()` | `print_i64` / `putchar 10` |
| end of module | `const 0`, `ret` (`main` returns `i64`) |

Python ints are unbounded and this subset's are `i64`. Where CPython would
produce an `int` outside `i64`, the program stops with `Trap.Overflow` after
the output so far. This is a stated divergence; bignum promotion is later
work. A zero divisor stops with `Trap.DivZero`, like `ZeroDivisionError`.
Everything else is a typed `FrontendError` (`Lex`, `Parse`, `Unsupported`,
`Budget`, `UnknownRelease`, `ReleaseNotAvailable`) that names the release and
a 1-based position. Refused examples: `/` (true division gives a float),
names, another callee, two arguments, and a literal outside `i64`.

Releases: 3.0a1, 3.11.17, 3.12.3, 3.13.16 and 3.16.0a1. All use the `py3`
profile in `src/profiles.rs`, which cites the CPython sources it rests on.
They emit byte-identical IIR for every corpus case they all accept. Tests:

| Test | Proves |
|------|--------|
| `tests/release_subsets.rs` | LANG84 stage I: every case of `code/grammars/python/corpus/py-int-print` on every claiming release (vm-core output and trap kind), C7 op checks, IIR identity, and the host CPython 3.11/3.12/3.13 oracle |
| `tests/release_lowering.rs` | which op each construct becomes, the `-1` fold, `main: i64`, located refusals, forged trees |
| `tests/cst_contract.rs` | LANG84 C8: every dispatched rule and token exists in each release |
| `tests/hostile_release.rs` | LANG84 C5 hostile probes, in a child process |
| `tests/iir_path.rs` | the release path never touches a legacy grammar or compares release IDs |

## LANG79's float pilot

This part of the crate lowers a bounded Python 3.12 float-expression, module-level
binding and `print` subset directly from `python-parser`'s grammar tree to
`interpreter-ir`, then runs it on the Rust `vm-core` interpreter. Semantic IR
is not in this execution path.

The accepted subset and explicit rejections are specified in
`code/specs/LANG79-python-direct-iir-pilot.md`. This is a native interpreter
pilot, not a complete Python implementation.

The bounded `print()` form writes one newline through a zero-argument Rust VM
builtin. `print(1.0)` uses the one-float builtin, and
`print(1.0, 2.0)` writes two supported floats with one separating space.
`print(1.0, 2.0, 3.0)` writes three supported floats with one space between
each value. Four or more arguments, keyword arguments, other callees, and
unsupported expressions fail explicitly.

Module-level float bindings are supported. A statement `NAME = <float
expression>` evaluates the value and then binds it; names may be rebound and
read in any float expression, including every `print` argument:

```python
rate = 1.5
total = rate * 2.0
total = total + 1.0
print(rate, total)      # 1.5 4.0
print(missing)          # NameError: name 'missing' is not defined
```

Reading a name before any statement binds it raises `NameError` at run time;
output from earlier statements is kept and later statements do not run. User
names live in the `py:` register namespace (`total` is `py:total`) so they
cannot collide with compiler temporaries (`py0`, `py1`, ...) or builtins.
Binding `print`, chained (`a = b = 1.0`), tuple, annotated, augmented
(`x += 1.0`), attribute and subscript assignment, integer values, and using
`print` or an unbound builtin name such as `float` as a value all fail at
compile time. Direct AST callers must supply canonical NAME and EQUALS tokens.

Run a supported source file with `cargo run -p python-iir-compiler --bin pyvm --
path/to/source.py` from the Rust workspace.

`compile_source` limits input to 64 KiB. The public `compile_ast` entry point
also caps tree items, depth, and text bytes before lowering. `run_source`
returns `PythonRunError` on failure; its `output` field contains any successful
earlier `print` calls, and `pyvm` writes those bytes before the error message.

Direct AST arithmetic also requires canonical unary/binary operator tokens
and grouping delimiters, including token kind, effective type and spelling.
Forged syntax tokens fail before execution; supported source expressions and
the existing resource bounds retain their behavior.

Direct AST float atoms also require the canonical named `FLOAT` token kind and
the complete ASCII spelling declared by the Python 3.12 lexer, checked before
underscore removal or f64 conversion. Forged integer spellings, malformed
underscores, nonnumeric names and conflicting enum kinds fail explicitly.
Valid exponent overflow/underflow and raw-versus-cooked metadata behavior,
source syntax and all existing runtime/resource bounds remain unchanged.
