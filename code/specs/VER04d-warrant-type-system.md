# VER04d — Warrant Type System, Scope Resolution and Binding Schema (P007)

**Parent:** [VER04 §2–§6, §8, §11](VER04-warrant-language.md), [VER04a](VER04a-warrant-source-admission.md), [VER04b](VER04b-warrant-layout-capture.md), and [VER04c](VER04c-guarded-warrant-parser.md).

This specification defines the semantic analysis, static type checking, scope/symbol resolution, and closed host-language binding schema for the pure profile of Warrant v0. It operates on the bounded AST produced by VER04c (`coding_adventures_warrant_grammar::parser`).

It delivers the type-checking and binding verification tier for **VER-P007**, gating downstream code generation (P008) and property verification (P009+).

---

## 1. Architecture and Pipeline Placement

```
Raw Bytes ──► [VER04a: SourceDocument::admit] ──► Admitted Document
                     │ (normalized CRLF, original byte mappings)
                     ▼
              [VER04b: layout::capture_layout] ──► LayoutToken stream
                     │ (INDENT / DEDENT / NEWLINE, zero-width origins)
                     ▼
              [VER04c: parser::parse_document] ──► WarrantFile (AST)
                     │ (bounded nodes, recursion depth <= 64)
                     ▼
              [VER04d: checker::check_warrant] ──► CheckedWarrantModule
                     │ (scope resolution, acyclic models, type checking, contracts)
                     ▼
              [VER04d: binding::validate_bindings] ──► ValidatedBindings
```

Semantic analysis guarantees:
1. **Scope and Namespace Integrity:** Symbols, aliases, models, and operations do not shadow or conflict; test identifiers (`example`, `claim`) live in their own namespace; binder variables do not shadow outer bindings.
2. **Acyclic Definitions:** Forward references in type aliases and pure model functions are resolved, and dependency cycles are strictly rejected (`W0203`).
3. **Sound Type System:** Every expression possesses a well-defined type; subtyping is limited strictly to `Nat <: Int` and `Bytes<N> <: Bytes`; arithmetic and logical operations follow exact mathematical or fixed-width bitvector rules.
4. **Contract and Bound Admissibility:** Operational contracts (`labels`, `bound`, `input`, `assume`) reference valid operations/parameters and express admissible invariants.
5. **Closed Host Binding Contract:** Host operations and types map to public library exports with verified signatures, absence of vacuity, and mandatory negative control hooks (`W0501`, `W0502`).

---

## 2. Symbols, Scopes and Dependency Graph

### 2.1 Namespace Separation
A Warrant document defines two disjoint root symbol tables:
- **Module Symbol Namespace:**
  - Type definitions: Primitive types (`Bool`, `Int`, `Nat`, `U8`, `U64`), built-in parametric types (`Bytes`, `List`, `Option`, `Result`), type aliases (`model T = ...`), and error types (`error E = ...`).
  - Callable definitions: Pure model functions (`model f(...) -> T = ...`) and declared public operations (`op g(...) -> T`).
  - Error constructors: Scoped under their parent error type (`ErrorName.CASE`).
  - Built-in functions: `len`, `at`, `concat`, `fixed`, `div`, `Some`, `None`, `Ok`, `Err`.
  - Collision rule: Redefining an existing type or callable name in the module scope emits `W0201`.
- **Test Symbol Namespace:**
  - `example` names and `claim` names share this namespace.
  - No two examples or claims may share the same identifier (`W0201`).

### 2.2 Local Binders
- Model function parameters, operation parameters, match arm variables, and quantifier binders introduce local variables.
- Binders must be unique within their immediate scope (`W0201`).
- Binders **cannot shadow** any active enclosing binder or module callable (`W0203`).

### 2.3 Forward References and Dependency Cycle Detection
- Declarations in the module may appear in any order.
- Mutually recursive or cyclic definitions among type aliases or pure model functions are prohibited:
  $$\text{Deps}(A) \cap \text{Reach}(A) = \emptyset$$
- Cycle detection constructs a directed dependency graph of types and model expressions. Any detected cycle emits `W0203` citing the cycle path.

---

## 3. Types, Values and Subtyping

### 3.1 Type Universe
```
Type ::= Bool
       | Int
       | Nat
       | U8
       | U64
       | Bytes
       | Bytes<N>        (where N is static natural or static param)
       | List<T>
       | Option<T>
       | Result<T, E>    (where E is declared nominal error)
```

### 3.2 Explicit Subtyping and Coercions
Subtyping is strictly minimal:
1. **Natural Widening:** $\text{Nat} \le \text{Int}$. Any expression of type `Nat` may be supplied where `Int` is expected.
2. **Fixed-to-Dynamic Width Widening:** $\text{Bytes}\langle N \rangle \le \text{Bytes}$. Any fixed-width byte slice may be supplied where dynamic `Bytes` is expected.
3. **No other implicit conversions exist:**
   - No numeric conversions between `Int`/`Nat` and `U8`/`U64`.
   - No truthy/falsy coercions (integers are never `Bool`).
   - No implicit unwrapping of `Option<T>` or `Result<T, E>`.

### 3.3 Expressions and Typing Rules

| Expression | Operand Types | Result Type | Notes |
|---|---|---|---|
| `- a` | `Int` or `Nat` | `Int` | Mathematical negation |
| `not a` | `Bool` | `Bool` | Logical negation |
| `a + b`, `a - b`, `a * b` | `Int` or `Nat` | `Int` | Exact mathematical arithmetic |
| `a + b`, `a - b`, `a * b` | `U8`, `U8` | `U8` | Arithmetic modulo 256 |
| `a + b`, `a - b`, `a * b` | `U64`, `U64` | `U64` | Arithmetic modulo $2^{64}$ |
| `a < b`, `<=`, `>`, `>=` | `Int`/`Nat`, `Int`/`Nat` | `Bool` | Signed ordering |
| `a < b`, `<=`, `>`, `>=` | `U8`, `U8` or `U64`, `U64` | `Bool` | Unsigned ordering |
| `a == b`, `a != b` | $T_1, T_2$ with common supertype $T$ | `Bool` | Structural equality |
| `a and b`, `a or b`, `a => b` | `Bool`, `Bool` | `Bool` | Boolean logic (`=>` right assoc) |
| `if p then a else b` | `p: Bool`, $a: T_a, b: T_b$ | $\text{LUB}(T_a, T_b)$ | Least upper bound via widening |
| `len(s)` | `Bytes`, `Bytes<N>`, `List<T>` | `Nat` | Length of sequence |
| `at(s, i)` | `s: List<T>, i: Nat` | `Option<T>` | Indexing (None if out of bounds) |
| `at(s, i)` | `s: Bytes | Bytes<N>, i: Nat` | `Option<U8>` | Byte indexing |
| `concat(a, b)` | `s: List<T>, List<T>` | `List<T>` | Sequence concatenation |
| `concat(a, b)` | `Bytes, Bytes` | `Bytes` | Concatenation |
| `concat(a, b)` | `Bytes<M>, Bytes<N>` | `Bytes<M + N>` | Static width preservation |
| `fixed<N>(b)` | `b: Bytes | Bytes<M>` | `Option<Bytes<N>>` | Checked width conversion |
| `div(a, b)` | `a, b: Int | Nat` | `Option<Int>` | None if $b=0$, else $\lfloor a/b \rfloor$ |
| `Some(e)` | `e: T` | `Option<T>` | |
| `None` | context requires `Option<T>` | `Option<T>` | |
| `Ok(e)` | `e: T`, context `Result<T, E>` | `Result<T, E>` | |
| `Err(e)` | `e: E.CASE` | `Result<T, E>` | |
| `match e with ...` | $e: \text{Option}\langle T \rangle$ or $\text{Result}\langle T, E \rangle$ | common arm type | Arms must be exhaustive |
| `forall x: T [\| p]. q` | $p: \text{Bool}, q: \text{Bool}$ | `Bool` | First-order universal |
| `exists x: T [\| p]. q` | $p: \text{Bool}, q: \text{Bool}$ | `Bool` | First-order existential |

---

## 4. Contract and Specification Checking

### 4.1 Operation Signatures and Calls
- Operations (`op`) represent external public interface entry points.
- Model definitions may only call other pure models and built-in functions. They **cannot call subject operations** (`W0301`).
- Examples and claims may call operations and model functions.

### 4.2 Labels Declarations
- `labels op_name:` must name a declared `op` (`W0401`).
- Every row names `param` or `param.contents` or `param.length`.
- Classification must be `public` or `secret`.
- All operation parameters must have an explicit secrecy label; missing labels emit `W0401`.

### 4.3 Bounds Declarations
- `bound op_name:` must name a declared `op` (`W0401`).
- Metrics must be one of `steps`, `heap`, `depth`.
- Bound expressions may refer only to operation parameters and arithmetic operations; their type must be `Int` or `Nat` (`W0301`).

### 4.4 Input Contracts
- `input op_name.param:` must name a declared `op` and its existing parameter (`W0401`).
- Must declare `hostile`.
- `rejects <pred> with <Error.CASE>`:
  - `<pred>` must have type `Bool`.
  - `<Error.CASE>` must be a valid constructor of the error type $E$ in the op's return type `Result<T, E>` (`W0401`).
- `ensures` must contain recognized contract flags: `no_panic`, `terminates`.

---

## 5. Host Binding Schema (Rust)

A companion JSON or declarative data schema specifies how abstract Warrant declarations bind to concrete Rust public exports:

```json
{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/warrant-binding.schema.json",
  "version": 1,
  "package": "coding_adventures_ct_compare",
  "revision": "origin/main",
  "bindings": {
    "eq": {
      "export": "ct_eq",
      "fresh": false,
      "negative_controls": ["always_true", "always_false"]
    },
    "eq_fixed": {
      "export": "ct_eq_fixed",
      "static_params": ["N"],
      "fresh": false,
      "negative_controls": ["always_true"]
    },
    "select_bytes": {
      "export": "ct_select_bytes",
      "fresh": true,
      "error_normalization": {
        "kind": "panic_to_error",
        "error_case": "CT_SELECT_LENGTH_MISMATCH"
      },
      "negative_controls": ["corrupt_choice"]
    },
    "eq_u64": {
      "export": "ct_eq_u64",
      "fresh": false,
      "negative_controls": ["always_true"]
    }
  }
}
```

### Binding Validation Rules:
1. Every `op` declared in the Warrant document must have a binding (`W0501`).
2. An unknown operation name in bindings emits `W0501`.
3. If an `op` is declared `fresh`, the binding must specify `fresh: true` (`W0502`).
4. If an `op` returns `Result`, any panic normalization must map to a valid constructor (`W0501`).
5. Every bound operation must declare at least one negative control (`W0502`).

---

## 6. Diagnostic Codes

| Code | Trigger |
|---|---|
| `W0201` | Duplicate symbol, constructor, parameter or test name |
| `W0202` | Unbound identifier, type, or error constructor |
| `W0203` | Cyclic model/type dependency or illegal variable shadowing |
| `W0301` | Type mismatch, arity error, unhandled match arm, or operation call from model |
| `W0401` | Invalid or missing `labels`, `bound`, `input`, or `assume` contract |
| `W0402` | Hostile input contract mismatch or secrecy violation |
| `W0501` | Missing or invalid host binding, export mismatch |
| `W0502` | Missing negative control or missing fresh declaration |

---

## 7. Budgets and Limits

Semantic checking enforces defensive quotas:
- `max_work`: default 1,000,000 charged work units (nodes checked, type unifications).
- `max_depth`: default 64 recursive expression/type frames.
- Exceeding limits halts checking immediately and emits `W0602`.

---

## 8. Verification and Quality Standard

1. **Unit & Integration Tests:**
   - Scope tests: duplicate detection, forward references, cycle rejection.
   - Type inference & checking: arithmetic, widening, comparisons, matches, quantifiers, built-ins.
   - Contract verification: bounds, inputs, labels, and platform/environment assumptions.
   - Binding validation: schema parsing, complete coverage of ops, negative control checks.
2. **Strict Code Coverage:**
   - **≥ 90% Line Coverage AND ≥ 90% Branch Coverage** across all type-checker and binding modules.
3. **Pure Language Primitives:**
   - Zero third-party dependencies.
