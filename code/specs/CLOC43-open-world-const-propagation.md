# CLOC43 — SIMPLE keeps a top-level `const` (open-world const propagation)

**Status:** implemented (CCR-078, #15863).
**Crate:** `closure-pass-inline-variables`.

## The divergence

At SIMPLE, closurec propagated a top-level `const` bound to a literal into
every use. The pinned oracle keeps it:

```text
input     let a = 1; const b = 2; console.log(a + b);
upstream  let a=1;const b=2;console.log(a+b);
closurec  let a=1;const b=2;console.log(a+2);     (before this change)
```

This was the one ladder divergence where closurec optimized *more* than
upstream. That output had never been validated against the reference.

## What upstream does at SIMPLE

Measured with `closure-compiler-v20260915` (sha256 verified), Java 21.0.10,
`--compilation_level SIMPLE_OPTIMIZATIONS`:

| Input | Upstream |
|---|---|
| `const b=2; console.log(b);` | unchanged |
| `const s="x"; console.log(s);` | unchanged |
| `const N=2; console.log(N+1);` | unchanged |
| `const T=true; T&&console.log(1);` | `const T=!0;console.log(1);` |
| `const D=false; if(D)a();else b();` | `const D=!1;b();` |
| `const D=false; D?a():b();` | `const D=!1;b();` |
| `const N=0; N\|\|console.log(1);` | `const N=0;console.log(1);` |
| `const N=2; if(N)console.log(1);` | `const N=2;console.log(1);` |
| `const N=2; if(N>1)console.log(1);` | `const N=2;N>1&&console.log(1);` |
| `const T=true; while(!T)a();` | `const T=!0;for(;!T;)a();` |
| `const D=false; console.log(D?1:2);` | `const D=!1;console.log(D?1:2);` |
| `const T=true; var x=T&&f();` | `const T=!0;var x=T&&f();` |
| `function f(){const b=2;return b+1}` | `function f(){return 3}` |

Two rules follow from the table.

1. **A top-level binding is never propagated to an ordinary use.** At SIMPLE
   the compiler is open-world: a top-level binding is a property of the
   global object, which another script may read. Locals are closed-world at
   every level and *are* propagated (last row).
2. **A literal-initialized top-level binding still decides a statement
   condition.** That condition must be the bare name, in one of three
   positions: an `if` test, the left operand of a statement-level `&&`/`||`,
   or the test of a statement-level `?:`. The condition then folds away. A
   compound test (`N>1`, `!T`), a value-position ternary, and an initializer
   don't qualify.

Upstream applies rule 2 to `let`/`var` as well
(`let T=true;T&&f()` → `let T=!0;f();`).

## What closurec does now

In open-world mode (`InlineVariablesPass::new()`, used at SIMPLE), a scalar
`const` candidate is substituted only at the condition sites in rule 2
(`is_condition_site`). constant-fold and fold-control-flow then remove the
condition. The declaration and every ordinary use are kept.
`InlineVariablesPass::closed_world()` (ADVANCED) is unchanged.

The TDZ guard (`prefix_is_inert`) and the single-declaration shadow guard
still apply. The multi-use size budget doesn't, because nothing is duplicated
into ordinary uses.

## Shadowing guard

A candidate must be bound exactly once in the whole program. The typed
`count_decl_names_*` walk sees only statement-level declarations. Since this
change, `count_binding_sites` also scans the serialized AST, so it counts the
bindings that walk missed:

- parameters and locals of function expressions, arrows and methods;
- the names of named function and class expressions;
- catch parameters.

Before, such a shadow received the const's value at both levels. For example,
`const T=true;(function(T){if(T)a();else b()})(false)` ran `a()`.

## Remaining gaps, deliberately out of scope

- **Rule 1's flip side.** closurec doesn't propagate *locals* at SIMPLE
  (`function f(){const b=2;return b+1}` stays). That is the CCR-068
  local-propagation gap.
- **Rule 2 for `let`/`var`.** It isn't implemented: open-world `let`/`var`
  candidates would need the write proof the closed-world path uses (CLOC29).

## Evidence in the tree

- `ladder_t6_let_const_simple` now matches upstream, so its ledger entry is
  deleted.
- `ladder_t6_top_const_and_simple` and `ladder_t6_top_const_cond_simple` keep
  matching.
- The `simple-inline-variables` fixture was re-captured from the jar. Until
  now it pinned closurec's old output.
