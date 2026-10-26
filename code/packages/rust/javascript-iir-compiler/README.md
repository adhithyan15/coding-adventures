# javascript-iir-compiler

The first native JavaScript execution slice. It parses JavaScript into the
generic parse tree (CST), lowers a deliberately small numeric/`console.log`
subset from that tree directly into InterpreterIR, and runs it on the Rust
`vm-core` interpreter. It does not use Semantic IR or a host JavaScript runtime
to execute programs. Since 0.6.0 (LANG84 phase 6e) the IIR holds LANG83
registry operations only, so `lang-aot --lang javascript` also runs it on
native x86-64/AArch64, LLVM, WASM, JVM, CLR, BEAM and the JIT, with no
JavaScript code in any backend (see "Every backend" below).

## Pipeline: the compiler walks the parse tree

```text
release ──resolve_release──▶ one edition, e.g. es2017 (default es2020)
source ──javascript-lexer (GrammarLexer, es2017.tokens + F10 modes)──▶ tokens
       ──javascript-parser::parse_javascript_cst_bounded─────────────▶ GrammarASTNode (CST)
             (GrammarParser over es2017.grammar + ASI + spine bound)        │
                                                                           │ compile_ast
                                              javascript-iir-compiler      ▼
                                                                       IIRModule ──▶ vm-core
```

Per LANG84 ("the IIR compiler walks the parse tree"), the compiler's only
input is the `GrammarASTNode` that the shared grammar-driven parser builds from
one edition's `code/grammars/ecmascript/es<edition>.grammar`. Since 0.3.0 it no longer goes
through the parser crate's typed `javascript_ast::Program` bridge, which
remains available for other tools (closurec).

The compiler reads the tree through borrowed, `Copy` handles; nothing is
copied into a second tree:

- the twenty-odd single-child rules of the precedence ladder, and `( … )`,
  are seen through in a loop;
- a flat binary node such as `additive_expression [8, -, 2, +, 1]` is read as
  the left-associative `(8 - 2) + 1` by index arithmetic (its left operand is
  "the first two operands of the same node");
- a flat suffix chain such as `call_expression [console, ., log, (1)]` is read
  as `Call(Member(console, log), [1])` the same way.

A validation pass over the CST rejects what the typed-AST stage used to
reject — a numeric literal wider than 64 bits in hex/binary/octal, a
destructuring pattern, a malformed tree — at the same point and with the
same messages. The emitted IIR is identical to the 0.2.0 typed-AST pipeline.

```rust
let output = javascript_iir_compiler::run_source("console.log(1 / 2);")?;
assert_eq!(output, "0.5\n");
```

`cargo run -p javascript-iir-compiler --bin jsvm -- example.js` executes a
file through the same native path.

Only numeric literals, unary `-`, binary `+`, `-`, `*`, `/`, identifier
reads, expression statements, top-level numeric `let`/`const` declarations,
and zero-, one-, two-, or three-argument numeric `console.log` are accepted.
An empty call emits one newline; a call with multiple arguments separates the
formatted numbers with one space. Four or more arguments and
other syntax are rejected explicitly. This is a pilot, not a complete
JavaScript engine.
Every number prints as node's `console.log` prints it: `String(x)` (so `1e21`
is `1e+21` and `0.0000001` is `1e-7`), except that `-0` keeps its sign.
If the program stops on a ReferenceError, `run_source` returns a
`JavaScriptRunError` containing the output from earlier completed
`console.log` calls and the error message. Every argument is evaluated before
the line is written, so a ReferenceError in the last argument appends none of
the failing call's text. The `jsvm` command flushes that output
to stdout before reporting the error on stderr and exiting unsuccessfully.
A compile error has empty output. es2020.tokens carries the F10 mode table, so
the ordinary lexer reads a `/` after a value as division: repeated divisions
and `console.log(-0, 1 / 0, 0 / 0)` execute correctly, and a real regex
literal lexes as one `REGEX` token that the subset check rejects (regex
literals remain unsupported). The table approximates JavaScript's
context-sensitive lexical goals; full goal-directed lexing is LANG84 E7.
Source files are limited to 64 KiB; direct CST compilation has node and depth
limits (declarators and their initializers included), and the runner caps
instructions and captured output. A caller may hand `compile_ast` a tree it
built itself: malformed shapes are errors, and wrapper towers are walked
iteratively, so no tree can panic the compiler or exhaust its stack.

## Bindings

Top-level `let NAME = <numeric expression>;` and `const NAME = ...;` bind one
plain identifier. Each binding lives in IIR register `js.<name>`: the
instruction computing the initializer writes it directly (a `mov` only when
the initializer is another binding); `.` cannot appear in a JavaScript
identifier, so user names never collide with compiler temporaries and labels
(`js0`, `js1`, ...).

```rust
let output = javascript_iir_compiler::run_source(
    "let x = 1 / 2; const y = x * 4; console.log(x, y);",
)?;
assert_eq!(output, "0.5 2\n");
```

Without control flow or assignment, each name's state is known at every
program point, so the compiler applies JavaScript's script rules directly:

| Program | Result |
| --- | --- |
| `let a = 1; let a = 2;` | compile error `SyntaxError: Identifier 'a' has already been declared`, no output |
| `console.log(1); console.log(a); let a = 2;` | prints `1`, then `ReferenceError: Cannot access 'a' before initialization` |
| `console.log(1); console.log(z);` | prints `1`, then `ReferenceError: z is not defined` |
| `const k;` | compile error `SyntaxError: Missing initializer in const declaration` |

The runner's global object holds the ECMA-262 standard globals, `escape`,
`unescape`, `Intl`, and `console`. None of them is implemented as a value
yet, so reading one (`NaN`, `undefined`, `Math`) or binding one with
`let`/`const` is rejected at compile time rather than misreported as an
undeclared name. Node-only host globals such as `process` are not part of
this runner's host and read as undeclared. `var`, `let x;`, several
declarators in one declaration, destructuring, assignment, and non-numeric
initializers are rejected at compile time.

See [LANG78](../../../specs/LANG78-native-dynamic-iir-frontends.md).

## Editions (LANG84 phase 6d)

A release is a row of `code/grammars/ecmascript/releases.csv`, named by its
exact ID. Ten editions claim the `es-number-log` subset:

| Request | Result |
| --- | --- |
| `None` | `es2020` (`DEFAULT_RELEASE`, what the IIR path always parsed) |
| `"es2016"` … `"es2025"` | that edition's `.tokens`/`.grammar` pair; the module records it in `IIRModule::release` |
| `"es1"`, `"es3"`, `"es5"`, `"es2015"`, `"es2026"`, `"js1.x"` | `ReleaseNotAvailable` (a row that does not claim the subset yet) |
| anything else (`"ES2020"`, `"2020"`, `""`) | `UnknownRelease`, with up to five suggestions |

Nothing falls back to a neighbouring edition. Editions agree on what an
accepted program means; they differ only in what they accept (numeric
separators are ES2021, so es2016–es2020 refuse `1_000`). The corpus
`code/grammars/ecmascript/corpus/es-number-log/` is the proof, and
`release_classes.csv` records the two IIR-identity classes it yields.

```rust
let module = compile_source_at("console.log(1_000 / 8);", Some("es2021"), "m")?;
assert_eq!(module.release.as_deref(), Some("es2021"));
let error = compile_source_at("console.log(1_000 / 8);", Some("es2020"), "m").unwrap_err();
assert_eq!(error.category(), "Parse");
```

## API

- `compile_source_at(src, release, module_name) -> Result<IIRModule, FrontendError>`
- `run_source_at(src, release) -> Result<String, JavaScriptRunError>`
- `supported_releases()`, `resolve_release(release)`, `DEFAULT_RELEASE`, `SUBSET`
- `compile_source(src, module_name) -> Result<IIRModule, String>` — the
  default edition, with the pre-0.5 error text
- `compile_ast(&GrammarASTNode, module_name) -> Result<IIRModule, String>` —
  for a `program` tree you already hold, from
  `parse_javascript_cst_bounded` or `parse_javascript_typed`
  (changed in 0.3.0: it used to take `&javascript_ast::Program`)
- `run_source(src) -> Result<String, JavaScriptRunError>`

## Every backend (LANG84 phase 6e)

The module uses LANG83 registry operations only:

| JavaScript | IIR |
| --- | --- |
| number literal | `const` (`f64`) |
| `a + b`, `a - b`, `a * b`, `a / b`, `-a` | `f64_add`, `f64_sub`, `f64_mul`, `f64_div`, `f64_neg` (IEEE, never a trap) |
| `console.log(a, b)` | evaluate `a` and `b`, then `write_f64_js a`, `putchar 32`, `write_f64_js b`, `putchar 10` |
| `-0` | `putchar 45` when `f64_signbit x` > `f64_signbit (x + 0)` |
| ReferenceError | `trap "Trap.ReferenceError"` |

`write_f64_js` is `String(x)`, which prints `-0` as `0`, while
`console.log` prints `-0`. Adding `+0` turns `-0` into `+0` and leaves every
other non-NaN value alone, so the sign bits of `x` and `x + 0` differ for
`-0` only. A literal can never be `-0`, so printing a literal (or a binding of
one) skips that test: `console.log(42)` has no arithmetic at all, and even
BEAM, whose floats cannot hold an infinity and which refuses the `f64_*` ops
at compile time, runs it.

`run_module` registers `write_f64_js` and `putchar` into one bounded buffer.
A `Trap.ReferenceError` ends the program (the compiler emits nothing after
it); `run_source` reports it with V8's message, which the compiler recorded,
since the IIR does not name the binding, while `run_module` on its own reports
`trap: Trap.ReferenceError`.
