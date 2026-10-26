# Python IIR compiler

This crate holds two Python frontends that share nothing but the crate:

| Path | Entry points | Subset | Grammar |
|------|--------------|--------|---------|
| LANG84 release path (phases 7a–7c) | `compile_source_at`, `compile_cst`, `run_source_at` | `py-int-print`: `print` of integer arithmetic; `py-float-print` (7c slice 3): plus floats; `py2-int-print`; on all nine backends | one tagged CPython release's `python<release>.*` pair |
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

Python 3 releases: 3.0a1 (profile `py30`), and 3.5.10, 3.6.0, 3.11.17,
3.12.3, 3.13.16 and 3.16.0a1 (profile `py3`), which also compile
`py-float-print` (LANG84 phase 7c slice 3, below). Python 2 releases (LANG84 phase 7c):
2.2, 2.4, 2.6 and 2.7.18, on the `py2` profile, which compile the
`py2-int-print` subset: the `print` statement, classic `/` (it floors on
ints and lowers like `//`), and every Python 2 integer spelling (`010` is
octal 8, `0x1F`, `0o17` and `0b101` from 2.6, and an `L` suffix that prints
without the `L`). A profile is a plain struct of switches in
`src/profiles.rs` (`slash`, `int_literals`, `floats`, `subsets`), which
cites the CPython sources it rests on. Releases with one profile emit byte-identical
IIR for every corpus case they all accept. The
readers of the inventory and corpus that the release tests share live in
`tests/common/mod.rs`. Tests:

| Test | Proves |
|------|--------|
| `tests/release_subsets.rs` | LANG84 stage I: every case of `code/grammars/python/corpus/py-int-print`, `py-float-print` and `py2-int-print` on every claiming release (vm-core output and trap kind), C7 op checks, IIR identity, and the host CPython 3.11/3.12/3.13 oracle |
| `tests/release_sweep.rs` | LANG84 "CI bounding": every positive case under every claiming release, grouped into IIR-identity classes (for `py-int-print`: 3.0a1 with 3.5.10, which refuse `int_underscores`, and 3.6.0 with 3.12.3, 3.13.16, 3.11.17 and 3.16.0a1; for `py2-int-print`: 2.2 with 2.4, and 2.6 with 2.7.18, the releases that lex PEP 3127 `0o`/`0b`; for `py-float-print`: 3.5.10 alone, which refuses `float_underscores`, and the other five), checked against `code/grammars/python/release_classes.csv`. After a deliberate change, `PYTHON_RELEASE_SWEEP_WRITE=1 cargo test -p python-iir-compiler --test release_sweep` rewrites the file (and fails, so review the diff and re-run). `lang-aot`'s matrix runs the corpus on each class representative |
| `tests/release_lowering.rs` | which op each construct becomes, the `-1` fold, `main: i64`, located refusals, forged trees |
| `tests/cst_contract.rs` | LANG84 C8: every dispatched rule and token exists in each release |
| `tests/hostile_release.rs` | LANG84 C5 hostile probes, in a child process |
| `tests/iir_path.rs` | the release path never touches a legacy grammar or compares release IDs |

### Floats: `py-float-print` (LANG84 phase 7c slice 3)

On the `py3` profile a program may also use float literals (`1.5`, `.5`,
`1.`, `1e16`, `1_000.5` from 3.6.0), `/` as true division, and any mix of
`int` and `float` in `+ - * /` and unary `+ -`:

```rust
use python_iir_compiler::run_source_at;

let ran = run_source_at("print(7 / 2)
print(0.1 + 0.2)
print(-0.0)
print(1e308 * 10)
", None).unwrap();
assert_eq!(ran.stdout, "3.5
0.30000000000000004
-0.0
inf
");
let ran = run_source_at("print(1.5)
print(1 / 0.0)
", None).unwrap();
assert_eq!((ran.stdout.as_str(), ran.trap.as_deref()), ("1.5
", Some("Trap.DivZero")));
```

| Python | IIR |
|--------|-----|
| a float literal | `const x : f64` (correctly rounded) |
| an `int` operand of a float op | `int_to_real` |
| `+ - *` with a float, `/` always | `f64_add`, `f64_sub`, `f64_mul`, `f64_div` (LANG83 PR6c, IEEE, never trap) |
| `/` by a known `±0` | `trap "Trap.DivZero"` first (CPython's ZeroDivisionError) |
| `-a`, `+a` (float) | `f64_neg`, nothing |
| `print(a)` (float) | `write_f64_py` (Python `repr`), `putchar 10` |

Refused (`Unsupported`): `//` and `%` on a float, `int / int` with an operand
above 2^53 (CPython rounds the exact quotient once), and a literal that
overflows to `inf` (`1e400`). 3.0a1 has no floats: its `str(float)` is
`%.12g`, which no LANG83 writer produces. BEAM refuses the `f64_*`
arithmetic at compile time (its floats have no `inf`/`nan`); a literal-only
program runs there too.

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
