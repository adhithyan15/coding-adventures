# javascript-iir-compiler

The first native JavaScript execution slice. It parses JavaScript into the
generic parse tree (CST), lowers a deliberately small numeric/`console.log`
subset from that tree directly into InterpreterIR, and runs it on the Rust
`vm-core` interpreter. It does not use Semantic IR or a host JavaScript runtime
to execute programs.

## Pipeline: the compiler walks the parse tree

```text
source ──javascript-lexer (GrammarLexer, es2020.tokens, REGEX off)──▶ tokens
       ──javascript-parser::parse_javascript_cst_division_only──────▶ GrammarASTNode (CST)
             (GrammarParser over es2020.grammar + ASI + spine bound)        │
                                                                           │ compile_ast
                                              javascript-iir-compiler      ▼
                                                                       IIRModule ──▶ vm-core
```

Per LANG84 ("the IIR compiler walks the parse tree"), the compiler's only
input is the `GrammarASTNode` that the shared grammar-driven parser builds from
`code/grammars/ecmascript/es2020.grammar`. Since 0.3.0 it no longer goes
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
Number display outside the pilot's finite `1e-6` to `1e21` range is rejected
until full ECMAScript formatting lands.
If a later VM instruction fails, `run_source` returns a `JavaScriptRunError`
containing the output from earlier completed `console.log` calls and the error
message. A number-display failure appends none of the failing call's text,
even when it occurs in the last argument. The `jsvm` command flushes that output
to stdout before reporting the error on stderr and exiting unsuccessfully.
A compile error has empty output. The native numeric source path selects an
explicit division-only tokenizer/parser entry, so repeated divisions and
`console.log(-0, 1 / 0, 0 / 0)` execute correctly. Regex literals remain
unsupported. General JavaScript parser callers retain their existing regex
rules; full context-sensitive regex/division disambiguation remains open.
Source files are limited to 64 KiB; direct CST compilation has node and depth
limits (declarators and their initializers included), and the runner caps
instructions and captured output. A caller may hand `compile_ast` a tree it
built itself: malformed shapes are errors, and wrapper towers are walked
iteratively, so no tree can panic the compiler or exhaust its stack.

## Bindings

Top-level `let NAME = <numeric expression>;` and `const NAME = ...;` bind one
plain identifier. Each binding lowers to a `mov` into IIR register
`js.<name>`; `.` cannot appear in a JavaScript identifier, so user names never
collide with compiler temporaries (`js0`, `js1`, ...) or runner builtins
(`js_console_log`, ...).

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

## API

- `compile_source(src, module_name) -> Result<IIRModule, String>`
- `compile_ast(&GrammarASTNode, module_name) -> Result<IIRModule, String>` —
  for a `program` tree you already hold, from
  `parse_javascript_cst_division_only` or `parse_javascript_typed`
  (changed in 0.3.0: it used to take `&javascript_ast::Program`)
- `run_source(src) -> Result<String, JavaScriptRunError>`
