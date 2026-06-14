# V8C03 — `v8-ir-compiler`: AST → IIR lowering

> Status: spec. Implementation lands in a separate PR per the
> repo's specs-first workflow.

## 1. Purpose

`v8-ir-compiler` is the crate that turns a `javascript-ast::Program`
into an `interpreter-ir::IIRModule`. It's the third spec in the V8
series: V8C01 set the strategy, V8C02 nailed down the JS value
representation and the LangBinding surface; this spec specifies the
*compiler* — the function that walks the AST and emits the IIR
that the LANG VM will then execute through the binding from V8C02.

Concretely:

```text
                   ┌──────────────────────────┐
                   │ javascript-ast::Program  │
                   └────────────┬─────────────┘
                                │
                                ▼  v8-ir-compiler (this spec)
                   ┌──────────────────────────┐
                   │ interpreter-ir::IIRModule │
                   └────────────┬─────────────┘
                                │
                                ▼  vm-core + v8-binding (V8C02)
                          program output
```

`v8-ir-compiler` is to V8 what `closure-emitter` is to the Closure
clone — the layer that materialises a higher-level representation
into bytes the runtime can consume. The shape of the IR it emits
is fixed: it must speak the same `IIRInstr` / `IIRFunction` /
`IIRModule` API that every other LANG frontend speaks (see
[interpreter-ir](../packages/rust/interpreter-ir/src/lib.rs)).
That's what lets a single LANG VM run JS, Lispy, Lua, Ruby,
Python, etc., with the same execution engine.

## 2. Why this needs its own spec

We could have folded lowering into the binding. We're not, for two
reasons:

1. **Symmetry with the Closure clone.** The Closure clone has the
   AST in one crate (`javascript-ast`), the lowering in another
   (`closure-emitter` over the pass pipeline), and the runtime in
   a third (`closurec` driving the passes). The V8 clone follows
   the same layering: AST (shared with Closure!) → IR compiler
   (this crate) → runtime (`v8-binding` + `vm-core`). One concept
   per crate. Easy to reason about. Easy to test.
2. **The lowering rules are real content.** "How does a JS
   `for` loop become IIR branches?" "How does `var` vs `let`
   vs `const` affect emission?" "What does an `ArrayExpression`
   look like in IIR?" Each of these has a specific, defensible
   answer, and the answers compose into a couple hundred
   lines of careful code. Splitting them off into their own
   spec means we can review the lowering decisions independently
   of the value-representation decisions in V8C02.

## 3. Inputs and outputs

### 3.1 Input — `javascript-ast::Program`

The Phase 1 AST (per CLOC09) covers the JS subset V8 v1 supports:

**Expressions:** `Identifier`, `NumericLiteral`, `StringLiteral`,
`BooleanLiteral`, `NullLiteral`, `BinaryExpression`,
`LogicalExpression`, `UnaryExpression`, `AssignmentExpression`,
`ConditionalExpression`, `CallExpression`, `MemberExpression`,
`ArrayExpression`, `ObjectExpression`.

**Statements:** `ExpressionStatement`, `BlockStatement`,
`IfStatement`, `WhileStatement`, `ForStatement`, `ReturnStatement`,
`BreakStatement`, `ContinueStatement`, `EmptyStatement`.

**Declarations:** `VariableDeclaration` (var/let/const),
`FunctionDeclaration`.

Each node carries an optional `cv: Option<CvId>` per the CLOC09
amendment — the correlation vector ID minted by the parser
(`javascript-parser`) that we propagate through every IIR
instruction we emit on behalf of that node.

### 3.2 Output — `interpreter-ir::IIRModule`

A single `IIRModule` per source file:

```rust
IIRModule {
    name: String,            // e.g. "user.js"
    functions: Vec<IIRFunction>,
    exports: Vec<IIRExport>,
    imports: Vec<IIRImport>, // empty in v1 — no modules yet
}
```

Top-level code lowers to a function named `"<main>"` (matching
the convention `lispy-runtime` uses for its top-level form).
Function declarations and function expressions each become their
own `IIRFunction`.

### 3.3 Entry point

```rust
pub fn compile(
    program: &javascript_ast::Program,
    options: &CompileOptions,
    cv: &mut CVLog,
) -> Result<IIRModule, CompileError>;
```

`CompileOptions` is empty v1 (placeholder for future flags like
`strict_mode`, `module_format`, `target_es_version`). `CVLog` is
the shared correlation-vector recorder defined by
`coding_adventures_correlation_vector` — same instance the AST
and the binding will use, so traces line up end-to-end.

`CompileError` is a normal error enum with variants like
`UnsupportedExpression { kind, cv }`, `UnresolvedIdentifier`,
`DuplicateConstDeclaration`. v1 only fails on AST shapes the
Phase 1 lowering tables don't cover — the parser already
rejects malformed JS, so we don't redo syntactic checks.

## 4. Compilation strategy

Single-pass tree walk, like every other v1 frontend in the repo
(Lispy, Twig, mini-Lua). We are emphatically not building SSA or
CFG-style IR here; IIR is already linear with `label`-based
branches, and v1 leans on the existing optimisations in vm-core
(profiling + slot specialisation) rather than reinventing them
in the JS frontend.

The compiler keeps a small `Lowerer` struct as it walks:

```rust
struct Lowerer<'a> {
    module_name: &'a str,
    cv: &'a mut CVLog,

    // The function currently being built. We push instructions
    // into here, then move it into `module.functions` when the
    // function body's AST is fully walked.
    current_fn: IIRFunction,

    // Stack of nested function bodies (for nested fn decls /
    // function expressions). Walking enters by pushing; leaving
    // pops back to the enclosing function.
    fn_stack: Vec<IIRFunction>,

    // Lexical scope chain — see §6.
    scopes: Vec<Scope>,

    // Counters for fresh SSA-like variable names + label IDs.
    next_var_id: u32,
    next_label_id: u32,

    // Loop context for break/continue resolution.
    loop_stack: Vec<LoopContext>,
}

struct LoopContext {
    continue_label: String,  // jump target for `continue`
    break_label: String,     // jump target for `break`
}
```

Visiting is mutually recursive — `lower_statement`, `lower_expression`,
`lower_declaration`. Each `lower_expression` returns the
`Operand::Var(name)` it allocated to hold its result; the caller
threads that into the next instruction. `lower_statement` doesn't
return a value — it just appends instructions.

### 4.1 Fresh variable naming

We name temporaries `%v0, %v1, %v2, ...` — same convention every
other LANG frontend uses, and the same one IIR's `instr::Operand`
example uses. JS source variables get an SSA renaming with the
source name suffixed: `x` declared at the top of `<main>` becomes
`x_0`. If `x` is reassigned (legal for `var`/`let`), we emit a
fresh name `x_1` and the scope's environment binding now points
to `x_1`. This is exactly the convention `closure-pass-rename`
already uses on the Closure side.

### 4.2 Fresh label naming

Labels are `L0, L1, L2, ...` numerically. We don't try to be
clever with names — the only consumer is vm-core, which only
cares about uniqueness within the function.

## 5. Per-variant lowering tables

This is the meat of the spec. Each table cell answers: "for AST
node *X*, what IIR instructions does the compiler emit?" Names
in monospace are exact opcode strings from
[`interpreter-ir/src/opcodes.rs`](../packages/rust/interpreter-ir/src/opcodes.rs).

### 5.1 Literals

| AST node             | IIR emission                                  | Notes |
|----------------------|-----------------------------------------------|-------|
| `NumericLiteral(n)`  | `%v = box(<n>) : ref<number>`                 | JS numbers are doubles; `Operand::Float`. |
| `StringLiteral(s)`   | `%v = box(<s>) : ref<string>`                 | `Operand::Str`. |
| `BooleanLiteral(b)`  | `%v = box(<b>) : ref<bool>`                   | `Operand::Bool`. |
| `NullLiteral`        | `%v = box(null) : ref<null>`                  | Singleton; the binding from V8C02 interns one. |
| (undefined ident)    | `%v = box(undefined) : ref<undef>`            | `undefined` is parsed as `Identifier("undefined")`; resolved to the interned singleton, not a global lookup. |

We use `box` rather than `alloc` because the binding stores
JS values uniformly through the V8C02 `Value` enum; the immediate
literal becomes the boxed value the rest of the IIR speaks. v1
heap-spills numbers (per V8C02 §"Boxing strategy: tagged-u64 with
heap-spilled numbers"), so even number literals route through `box`.

### 5.2 Identifiers

| Scope kind          | Lookup IIR                                        |
|---------------------|---------------------------------------------------|
| **Function-local**  | `%v = load_reg(<ssa-name>)` (see §6)              |
| **Captured by closure** | `%v = closure_load_upvalue(<slot-index>)`     |
| **Global**          | `%v = global_load("<name>")`                      |
| **Builtin singleton** (`undefined`, `null` as ident, `NaN`, `Infinity`) | inlined `box` of the interned value — no global lookup |

Resolution happens in the scope-chain walk (§6). Unknown
identifiers don't error at compile-time — JS allows reads of
undeclared globals to throw `ReferenceError` at runtime, so we
emit `global_load` and let the binding fault.

### 5.3 Unary expressions

| Op  | IIR sequence (result in `%v`)                                                 |
|-----|-------------------------------------------------------------------------------|
| `-` | `%a = lower(arg); %v = call_builtin("v8_to_number", %a); %v = neg(%v)`        |
| `+` | `%a = lower(arg); %v = call_builtin("v8_to_number", %a)`                      |
| `!` | `%a = lower(arg); %t = call_builtin("v8_to_boolean", %a); %v = not(%t)`       |
| `~` | `%a = lower(arg); %i = call_builtin("v8_to_int32", %a); %v = not(%i) : i32`   |
| `typeof` | `%a = lower(arg); %v = call_builtin("v8_typeof", %a) : ref<string>`      |
| `void`   | `%_ = lower(arg); %v = box(undefined) : ref<undef>`                      |
| `delete` | `call_builtin("v8_delete", %obj, %key)` — `delete x.y` lowers to operate on the property reference; bare `delete x` is a no-op in v1 (matches V8 in strict mode would throw; deferred) |

Coercion builtins (`v8_to_number`, `v8_to_boolean`, `v8_to_int32`,
`v8_typeof`, `v8_delete`) are part of the V8C02 binding's
builtin surface — they live in `v8-binding`'s `call_builtin`
dispatch table, not in vm-core. v1 implements them eagerly;
v2's IC layer (V8C04) will profile them and specialise.

### 5.4 Binary expressions

| JS op  | IIR (result in `%v`, args `%l` and `%r`)                              | Notes |
|--------|------------------------------------------------------------------------|-------|
| `+`    | `%v = call_builtin("v8_add", %l, %r)`                                  | Special — strings concat, numbers add; binding picks. |
| `-`    | `%nl = call_builtin("v8_to_number", %l); %nr = call_builtin("v8_to_number", %r); %v = sub(%nl, %nr)` | Numeric only after coercion. |
| `*`, `/`, `%` | analogous to `-`                                                | |
| `**`   | `... %v = call_builtin("v8_pow", %nl, %nr)`                            | No native `pow` op in IIR; builtin. |
| `==`, `!=` | `%v = call_builtin("v8_loose_eq", %l, %r)` (then `not` for `!=`)   | Loose equality is its own builtin. |
| `===`, `!==` | `%v = call_builtin("v8_strict_eq", %l, %r)`                      | Strict equality is its own builtin. |
| `<`, `<=`, `>`, `>=` | `%v = call_builtin("v8_compare_lt", %l, %r)` etc.            | Relational ops follow JS abstract relational comparison. |
| `&`, `|`, `^`, `<<`, `>>` | coerce to int32 via builtin, then native bitwise         | `>>>` is `v8_ushr` builtin. |
| `in`, `instanceof` | `%v = call_builtin("v8_in", %l, %r)` / `"v8_instanceof"`     | |

The pattern is: any operator whose semantics involve type
coercion (which is most of them in JS) routes through a
`call_builtin` defined by the binding. This keeps the IR compact,
makes the per-op semantics testable in isolation against the
binding, and gives V8C04's optimiser a natural seam to specialise
on observed types.

### 5.5 Logical expressions (`&&`, `||`, `??`)

These short-circuit, so we have to emit branches:

```text
  %l = lower(left)
  %t = call_builtin("v8_to_boolean", %l)
  jmp_if_false %t, L_else        ; for &&  (for || use jmp_if_true L_skip)
  %r = lower(right)
  %v = %r
  jmp L_end
L_else:
  %v = %l                        ; preserve left's value (NOT bool)
L_end:
```

`??` is the same shape, but the test is "is `%l` null or
undefined?" via `call_builtin("v8_is_nullish", %l)`.

Note the result is `%l` (not its boolean coercion). JS `a && b`
returns `a` when `a` is falsy, not `false`. Spec-correct.

### 5.6 Assignment expressions

| Form                  | IIR                                                            |
|-----------------------|----------------------------------------------------------------|
| `x = e`               | `%v = lower(e); store_reg(<x-ssa>, %v)` — `x` gets a fresh SSA name; the scope rebinding records it (§6) |
| `obj.prop = e`        | `%o = lower(obj); %v = lower(e); call_builtin("v8_set_named", %o, "<prop>", %v)` |
| `obj[k] = e`          | `%o = lower(obj); %k = lower(k); %v = lower(e); call_builtin("v8_set_computed", %o, %k, %v)` |
| `x += e`              | `%cur = load(x); %r = lower(e); %v = call_builtin("v8_add", %cur, %r); store_reg(<x'>, %v)` — and ditto for `-=`, `*=`, `/=`, `%=`, `**=`, `<<=`, `>>=`, `>>>=`, `&=`, `|=`, `^=` |
| `obj.prop += e`       | as above, but with `v8_get_named` and `v8_set_named`           |

The compound-assignment lowering uses the same builtins as the
matching binary operator — we don't have a separate `v8_add_assign`.

### 5.7 Conditional expression (ternary)

```text
  %c = lower(test)
  %t = call_builtin("v8_to_boolean", %c)
  jmp_if_false %t, L_else
  %v = lower(consequent)
  jmp L_end
L_else:
  %v = lower(alternate)
L_end:
```

### 5.8 Call expressions

| Callee shape            | IIR                                                                              |
|-------------------------|----------------------------------------------------------------------------------|
| Free function (identifier)   | `%f = load(callee); %args = lower(args); %v = call_closure(%f, %args...)`   |
| Method (`obj.foo()`)    | `%o = lower(obj); %f = call_builtin("v8_get_named", %o, "foo"); %v = call_closure(%f, %o, %args...)` — `%o` is passed as `this` |
| Computed method (`obj[k]()`) | analogous with `v8_get_computed`                                            |
| Builtin (resolved by name when callee is a known builtin) | `%v = call_builtin("<name>", %args...)`        |

`call_closure` is the IIR closure-call op; v8-binding maps it to
a JS function-call protocol that handles `arguments`, `this`,
and the `new` target (deferred — no `new` in v1 except for
built-in constructors via `call_builtin`).

### 5.9 Member expressions

| Form               | IIR                                                                |
|--------------------|--------------------------------------------------------------------|
| `obj.prop`         | `%o = lower(obj); %v = call_builtin("v8_get_named", %o, "<prop>")` |
| `obj[expr]`        | `%o = lower(obj); %k = lower(expr); %v = call_builtin("v8_get_computed", %o, %k)` |

### 5.10 Array expressions

```text
  %arr = call_builtin("v8_new_array", <length>)
  %e0  = lower(elements[0]); call_builtin("v8_array_set", %arr, 0, %e0)
  %e1  = lower(elements[1]); call_builtin("v8_array_set", %arr, 1, %e1)
  ...
  %v = %arr
```

Holes (`[1, , 3]` has `elements[1] = None`) skip the `array_set`
— the binding's array implementation reads holes as `undefined`
on lookup.

### 5.11 Object expressions

```text
  %obj = call_builtin("v8_new_object")
  ; for each property { kind: Init, key, value }:
  %v  = lower(value)
  call_builtin("v8_set_named", %obj, "<key>", %v)
  ; computed keys use v8_set_computed; getters/setters deferred
  %v = %obj
```

`PropertyKind::Get` and `PropertyKind::Set` (accessor properties)
are deferred — v1 emits a `CompileError::UnsupportedAccessor`
with the node's `cv` for now.

### 5.12 Variable declarations

| Kind   | Emission                                                                  |
|--------|---------------------------------------------------------------------------|
| `var`  | Hoisted to the enclosing function's prologue as `%x_0 = box(undefined)`. Initialiser, if any, lowers in-place as a normal assignment to a fresh SSA name. |
| `let`  | Declared at point of declaration; scope-bound to the surrounding block (§6). Temporal-dead-zone: reads before initialisation lower to `%v = call_builtin("v8_throw_tdz", "<name>")`. |
| `const` | Same as `let` but the scope marks the binding as immutable; subsequent assignments produce `CompileError::ConstReassignment`. |

### 5.13 Function declarations and expressions

```text
; In the enclosing function, at the *top* of the function body
; (hoisted), we emit:
%f_decl = alloc_closure(fn_id=<inner-function-index>, captures=[%up0, %up1, ...])
store_reg(<name-ssa>, %f_decl)

; At the inner function's site, the compiler also emits a new
; IIRFunction into the module with:
;   - name = <source name or "<anonymous>">
;   - params = parameter list
;   - body  = lowered statements (recursively)
;   - return_type = "any"  (JS is dynamic)
```

The captures list is computed by §6's scope analysis: any free
variable in the inner function's body that's bound in an outer
function scope becomes a captured upvalue. `alloc_closure` is
the standard LANG IIR op for this (matches `lispy-runtime`'s
emission for `lambda`).

Function *expressions* differ only in not being hoisted —
they're lowered in-place at the expression position. Named
function expressions bind the name only inside the function's
own body (per JS semantics).

### 5.14 Statements

| Statement              | Emission                                                                    |
|------------------------|-----------------------------------------------------------------------------|
| `ExpressionStatement`  | `lower(expr); ` — result discarded. Side effects already emitted. |
| `BlockStatement`       | Push a block scope (§6); lower body; pop scope.                              |
| `IfStatement`          | Test + `jmp_if_false` + consequent + (jmp end + alternate)?; labels.        |
| `WhileStatement`       | `L_head:` + test + `jmp_if_false L_break`; body; `jmp L_head`; `L_break:`. Pushes a `LoopContext`. |
| `ForStatement`         | init; `L_head:` test? + `jmp_if_false L_break`; body; `L_continue:` update; `jmp L_head`; `L_break:`. |
| `ReturnStatement`      | `lower(arg); ret %v` — or `ret_void` if no arg.                              |
| `BreakStatement`       | `jmp <loop_stack.last().break_label>`. Errors if not inside a loop.          |
| `ContinueStatement`    | `jmp <loop_stack.last().continue_label>`. Errors if not inside a loop.       |
| `EmptyStatement`       | Emits nothing.                                                              |

`for-in` and `for-of` are deferred — v1 supports only C-style
`for(init; test; update)`. `switch`, `try/catch/finally`, `do-while`,
labelled statements, generators, async functions: all deferred.

## 6. Scope analysis

The scope stack lives on the `Lowerer` and is built lazily as
the compiler walks. Each scope is:

```rust
struct Scope {
    kind: ScopeKind,                                  // Function | Block
    bindings: HashMap<String, BindingInfo>,           // source name -> info
}

enum ScopeKind { Function, Block }

struct BindingInfo {
    ssa_name: String,                                 // current SSA name (mutable refs update this)
    var_kind: VarKind,                                // Var | Let | Const
    captured_by_inner_fn: bool,                       // upvalue?
    initialized: bool,                                // for let/const TDZ tracking
}
```

### 6.1 Resolution algorithm

`lookup(name)`:
1. Walk `scopes` from top (innermost) to bottom (global).
2. At each frame, if `frame.bindings.contains_key(name)`:
   - If the frame is in the *current* function, return
     `Local(ssa_name)`.
   - If the frame is in an *enclosing* function (we crossed a
     `ScopeKind::Function` boundary), return
     `Captured(upvalue_index)` and mark the binding
     `captured_by_inner_fn = true`. The compiler also threads
     this through the current function's `upvalues` list so
     `alloc_closure` knows what to pass.
3. If we walk off the bottom without finding it, return
   `Global(name)`.

### 6.2 Hoisting

`var` and `FunctionDeclaration` are hoisted to the top of the
enclosing function. We pre-walk the function body once to collect
hoisted names, allocate their initial SSA names + `box(undefined)`
emission, then walk again to emit the rest. This costs one extra
tree traversal per function but avoids two-pass IR fixup.

### 6.3 Temporal dead zone (TDZ)

`let` and `const` bindings exist from the moment their scope opens
but error if accessed before initialisation. We track this via
`BindingInfo::initialized`. A `load_reg` of a `let`/`const`
binding before its initialiser flips a flag and emits a
`call_builtin("v8_throw_tdz", "<name>")` rather than the load.

v1 detects the simple case (binding declared in same block, used
before the initialiser line). Cross-block TDZ violations (closure
captures a `let` before its init) are deferred — we'd need flow
analysis.

## 7. Correlation-vector propagation

Every AST node carries an `Option<CvId>` (per CLOC09's
amendment). Every IIR instruction we emit on behalf of a node
gets a `cv` field stamped with that same `CvId`. If a single AST
node produces multiple instructions (the typical case — a
`BinaryExpression` becomes load-l, load-r, builtin-call), they
all share the AST node's `cv`.

When the compiler synthesises an instruction without a direct
AST parent (e.g. the implicit `ret box(undefined)` at the end of
a function body), it mints a fresh child CV via
`cv.derive_child("v8-ir-compiler.<reason>")`. Same convention
the Closure side uses in `closure-emitter`.

This is what lets the V8 stack trace, source map, and debugger
all line up. The vm-core executes an IIR instruction, looks up
its `cv`, and the binding can resolve that back to the source
range via the parser's CV log. End-to-end correlation, no extra
machinery.

## 8. What this crate is NOT responsible for

To keep the layering clean:

- **No optimisation.** v8-ir-compiler emits *correct* IIR.
  Folding constants, dead-code-eliminating unreachable branches,
  inlining short functions: all that lives in V8C04's pass
  pipeline.
- **No type inference.** Every type-hint is `"any"`. Profiling +
  slot specialisation in vm-core gives us monomorphic specialisation
  at runtime; static type inference is deferred indefinitely (it
  would compete with profiling, not complement it).
- **No source maps.** The CV log is enough — `v8-source-map`
  (a future V8 spec) will read the CV log post-facto. v8-ir-compiler
  just stamps `cv` on every instruction.
- **No JS *parsing*.** That's `javascript-parser` (already shipped
  for the Closure clone). v8-ir-compiler starts from a parsed AST.
- **No runtime semantics.** The binding from V8C02 implements
  what JS values *do*; v8-ir-compiler just decides which IIR
  instructions invoke those operations.

## 9. Testing strategy

Three tiers:

### 9.1 Unit tests — per AST-node lowering

For each AST node variant, a focused unit test asserts the
exact instruction sequence emitted:

```rust
#[test]
fn lower_binary_add_emits_v8_add_builtin() {
    let ast = parse("1 + 2");
    let module = compile(&ast, &Default::default(), &mut CVLog::new(true)).unwrap();
    let main = module.functions.iter().find(|f| f.name == "<main>").unwrap();
    assert_eq!(main.body[0].op, "box");
    assert_eq!(main.body[1].op, "box");
    assert_eq!(main.body[2].op, "call_builtin");
    assert_eq!(main.body[2].srcs[0].as_str_lit(), Some("v8_add"));
}
```

One test per row of the §5 lowering tables. ~60 tests.

### 9.2 Round-trip tests — lower then run

For each idiom, lower the JS, run the resulting IIR through
the LANG VM with v8-binding bound in, and assert the program's
output. This is what catches "the lowering is internally
consistent but doesn't actually match JS semantics" bugs.

About 25 round-trip tests covering: arithmetic, equality
quirks (`0 == "0"`, `NaN !== NaN`), string concat, array
indexing, object property access, function declaration and
call, recursion, closures capturing locals, scoping
(let/const TDZ), loops with break/continue.

### 9.3 CV-trace tests

For a representative program (~10 statements), assert that:
- every IIR instruction has a non-None `cv`;
- the set of `cv` IDs is a subset of the AST's `cv` IDs ∪
  the compiler's synthesised children;
- two instructions emitted for the same AST node share its `cv`.

These tests are what catch the inevitable "forgot to stamp CV
in the new lowering path" regressions.

Target coverage: 95% lines, 90% branches per the repo
standards.

## 10. Crate layout

```text
code/packages/rust/v8-ir-compiler/
├── BUILD
├── Cargo.toml
├── CHANGELOG.md
├── README.md
├── required_capabilities.json
└── src/
    ├── lib.rs              # public API (compile, CompileOptions, CompileError)
    ├── lowerer.rs          # Lowerer struct + tree walk
    ├── scope.rs            # Scope + BindingInfo + resolution
    ├── lower_expr.rs       # per-Expression-variant lowering
    ├── lower_stmt.rs       # per-Statement-variant lowering
    ├── lower_decl.rs       # per-Declaration-variant lowering
    └── builtins.rs         # canonical builtin-name constants (v8_add, v8_to_number, ...)
```

### 10.1 Dependencies

```toml
coding-adventures-javascript-ast = { path = "../javascript-ast" }
interpreter-ir = { path = "../interpreter-ir" }
coding_adventures_correlation_vector = { path = "../correlation-vector" }
```

No dependency on `v8-binding` itself — the compiler emits
*calls* to builtins by name; resolving those names happens at
runtime in the binding. This is exactly how `lispy-compiler`
and `lispy-runtime` are layered.

Dev-deps: `javascript-parser` for ergonomic test setup.

## 11. Versioning + roadmap

- **0.1.0** — All Phase 1 AST variants lowered, all §9.1 unit
  tests passing. Round-trip tests landed as a separate follow-up
  PR if scope is too large.
- **0.2.0** — Round-trip tests landed. v1 considered complete.
- **0.3.0+** — Phase 2 AST variants (classes, `for-of`,
  destructuring, spread, template literals, async/await,
  generators) as they're added to `javascript-ast`. Each AST
  expansion gets a matching v8-ir-compiler bump.

## 12. Interaction with V8C04 (lowering pipeline)

V8C04 will introduce a Closure-style pass pipeline for the V8
clone — passes that *transform* the IIRModule after v8-ir-compiler
emits it. The contract this spec establishes:

- The IIRModule v8-ir-compiler emits is **correct but
  unoptimised**.
- v8-ir-compiler emits IIR that's amenable to optimisation —
  e.g. it doesn't pre-fold constants (so V8C04's constant-fold
  pass has work to do), and it emits straight-line IIR even for
  trivially-foldable conditionals (so V8C04's
  fold-control-flow pass can prove them dead).
- v8-ir-compiler is **deterministic** — same AST, same options,
  byte-identical IIRModule. V8C04 passes can therefore checksum
  before/after and regenerate test fixtures.

## 13. What this PR locks down

This spec PR is documentation-only — no code yet. The scaffold PR
that follows creates the crate skeleton (Cargo.toml, lib.rs with
the public API signature, empty modules per §10, a single
`compile_empty_program_returns_module_with_main` smoke test).
Real lowering bodies land per-AST-node as follow-up PRs, with
the §9.1 tests gating each.

Reviewers should focus on:

- Do the §5 lowering tables compose into something a JS dev would
  recognise as "what V8 does"? (Spec-correctness, not perf.)
- Is the §6 scope algorithm self-consistent? (No accidental
  dynamic scoping; correct closure capture; hoisting handled.)
- Is the §3.3 entry-point signature future-proof for V8C04?
  (Returning `Result<IIRModule, CompileError>` rather than panicking;
  threading the shared `CVLog` rather than minting an internal one.)

When the spec lands, V8C03's scaffold PR begins immediately, and
real lowering bodies are an autonomous chain just like the
Closure-clone passes were.
