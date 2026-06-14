# SIR18 — Semantic IR → JavaScript

## Status

Fifth backend for the narrow-waist Semantic IR (after TypeScript,
Rust, Python, Go).  Implemented as the Rust crate
`semantic-ir-to-javascript`.

The crate consumes [`semantic_ir::Module`] (with SIR16 extensions) and
emits a single **self-contained** `.js` file — every produced file
embeds the runtime helpers; no `require()` / `import` of external
packages.  The file runs directly via `node <file>.js`.

## Public API

```rust
pub fn compile(module: &Module) -> Result<Artifact, BackendError>;

pub struct JavaScriptBackend;
impl semantic_ir::Backend for JavaScriptBackend {
    fn target_tag(&self) -> &'static str { "javascript" }
    fn accepts_features(&self) -> &'static [Feature];  // full v1
    ...
}
```

## Capability declaration

Accepts the **full SIR-v1** feature set (per SIR16):

- All v0 features (Closures, Pairs, Symbols, Strings, DynamicTyping,
  OptionalTypeAnnotations, MutualRecursion, Globals)
- Floats
- MutableBindings
- Loops
- Sequences
- Maps
- ShortCircuit

Rejects:

- `TailCalls` (V8 doesn't reliably TCO)
- `Intrinsics` (empty whitelist in v0)

## Value model

JavaScript is dynamically typed.  Every SIR value lowers to a native
JS value with no boxing:

| SIR concept   | JavaScript representation                                 |
|---------------|-----------------------------------------------------------|
| `Int`         | `number` (Number is the only JS numeric type; sufficient up to 2^53-1) |
| `Float`       | `number`                                                  |
| `Bool`        | `boolean`                                                 |
| `Nil`         | `null`                                                    |
| `Symbol`      | A `__SirSym` class instance with interned `.name`         |
| `Str`         | `string`                                                  |
| `Pair`        | A `__SirPair` class instance with `car` and `cdr`         |
| `Seq`         | `Array<any>` (native JS array)                             |
| `Map`         | `Object` keyed by string (native JS object)               |
| `Closure`     | A `__SirClosure` class instance wrapping the JS function  |

Backed by an inlined `__Sir` namespace at the top of every produced
file — same pattern as the TypeScript backend, simplified.

## Per-node lowering

| SIR node                             | Emitted JavaScript                                          |
|--------------------------------------|-------------------------------------------------------------|
| `IntLit { value }`                   | `<value>`                                                    |
| `FloatLit { value }`                 | `<value>` (with explicit decimal point if integer-valued)    |
| `BoolLit { value }`                  | `true` / `false`                                              |
| `NilLit`                             | `null`                                                       |
| `SymLit { name }`                    | `__Sir.intern("<name>")`                                     |
| `StrLit { value }`                   | `"<escaped>"`                                                |
| `VarRef { name, Local }`             | `<name>`                                                     |
| `VarRef { name, Param }`             | `<name>`                                                     |
| `VarRef { name, Capture }`           | `<name>`                                                     |
| `VarRef { name, Global }`            | `<name>`                                                     |
| `VarRef { name, Builtin }`           | `__Sir.builtins["<name>"]`                                   |
| `If { cond, then, else }`            | `(__Sir.truthy(cond) ? (then-block) : (else-block))`         |
| `Block` (in expr position)           | IIFE `(() => { stmts; return value; })()`                    |
| `Block` (in function body)           | flat `{ stmts; return value; }`                              |
| `Stmt::LetBinding`                   | `let <name> = <value>;`                                      |
| `Stmt::LetStarBinding`               | `let <name> = <value>;`                                      |
| `Stmt::Assign`                       | `<name> = <value>;`                                          |
| `Stmt::ExprStmt`                     | `<expr>;`                                                    |
| `Stmt::While`                        | `while (__Sir.truthy(<cond>)) { <body> }`                    |
| `Stmt::ForRange`                     | `for (let <v> = <start>; <v> < <stop>; <v> += <step>) { <body> }` |
| `Stmt::ForEach`                      | `for (const <v> of <iter>) { <body> }`                       |
| `Stmt::SeqSet`                       | `<seq>[<index>] = <value>;`                                  |
| `Stmt::MapSet`                       | `<map>[<key>] = <value>;`                                    |
| `Expr::DirectCall`                   | `<fn>(<args>)`                                               |
| `Expr::IndirectCall`                 | `__Sir.applyClosure(<target>, [<args>])`                     |
| `Expr::BuiltinCall { name, args }`   | `__Sir.builtins["<name>"](<args>)` *or specialised inline*    |
| `Expr::MakeClosure { fn, caps }`     | `new __Sir.Closure((..._a) => <fn>(<caps>, ..._a))`          |
| `Expr::SeqLit { items }`             | `[<items>]`                                                  |
| `Expr::SeqIndex { seq, index }`      | `<seq>[<index>]`                                             |
| `Expr::SeqLen { seq }`               | `<seq>.length`                                               |
| `Expr::MapLit { entries }`           | `{ "<k1>": <v1>, "<k2>": <v2>, ... }`                        |
| `Expr::MapGet { map, key }`          | `<map>[<key>]`                                               |
| `Expr::LogicalAnd { lhs, rhs }`      | `(<lhs> && <rhs>)`                                           |
| `Expr::LogicalOr { lhs, rhs }`       | `(<lhs> \|\| <rhs>)`                                         |

### Builtin specialisation

For idiomatic output, several builtins emit native JS rather than
runtime helper calls:

- `+`, `-`, `*`, `/`, `%` with 2 args → native `(a + b)`, `(a - b)`, etc.
- `=`, `!=`, `<`, `>`, `<=`, `>=` with 2 args → native `===`, `!==`, `<`, `>`, `<=`, `>=`
- `not` with 1 arg → native `!a`
- `neg` with 1 arg → native `(-a)`
- `len` with 1 arg → native `<arg>.length` (works for arrays and strings)
- `print` with 1 arg → `console.log(__Sir.format(<arg>))` — format ensures consistent stringification

Variadic operators (`+`/`-`/`*`/`/` with > 2 args) fall back to the
runtime dispatch table.

`range` always routes through `__Sir.builtins["range"]` since it
constructs an array.

## Inlined runtime

```js
const __Sir = (() => {
  class Sym { constructor(name) { this.name = name; } }
  class Pair { constructor(car, cdr) { this.car = car; this.cdr = cdr; } }
  class Closure { constructor(fn) { this.fn = fn; } }
  const symbolTable = new Map();
  function intern(name) {
    let s = symbolTable.get(name);
    if (s === undefined) { s = new Sym(name); symbolTable.set(name, s); }
    return s;
  }
  function applyClosure(c, args) {
    if (!(c instanceof Closure)) throw new TypeError("apply on non-closure");
    return c.fn(...args);
  }
  function truthy(v) { return v !== false && v !== null && v !== undefined; }
  function format(v) { ... }
  const builtins = { /* "+": plus, ..., "range": range, "len": len, ... */ };
  return { Sym, Pair, Closure, intern, applyClosure, truthy, format, builtins };
})();
```

Roughly 100 lines; mirrors the TS backend's `namespace __Sir { ... }`
without the type annotations.

## Output format

- Indentation: 2 spaces (idiomatic JS)
- Semicolons: always (no ASI reliance)
- Trailing newline at file end
- Banner comment at top with source language
- Strict mode: file starts with `"use strict";`

## Identifier sanitisation

JS identifiers match `[A-Za-z_$][A-Za-z0-9_$]*`.  Same rules as the
TS backend:

- Valid pass through
- Reserved words (`class`, `function`, `let`, `const`, `var`, `if`,
  `return`, etc.) get `_$` prefix
- Invalid chars `_$<hex>` encoded
- Empty → `_$empty`
- SIR's synthesised `main` → stays as `main` (JS has no entry-point
  collision)

## Tests

`cargo test -p semantic-ir-to-javascript`:

- Per-node lowering unit tests
- Determinism test
- End-to-end Python → SIR → JS via `python-to-semantic-ir` + this
  backend.  Programs:
  - factorial
  - fibonacci (iterative with mutation + while)
  - list-sum (for-each)
  - dict access
  - closure adder

When `node` is on PATH at test time, the emitted JS is also executed
and stdout is compared.  Without `node`, the syntactic tests still
verify the output shape.

## Out of scope

- Source maps
- ES modules / CommonJS exports (the output is a standalone script)
- `await` / `async function`
- BigInt for arbitrary-precision integers
- TypedArrays
- Symbols (JS-native; we use our own `__Sir.Sym` class)
