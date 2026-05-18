# semantic-ir-to-go

Fourth backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Go source
code — every emitted `.go` file is `package main` with inlined runtime
helpers; no `go.mod` external dependencies.  `go build <file>.go` builds
a working binary.

Implements [SIR15](../../../specs/SIR15-semantic-ir-to-go.md).

## Public API

```rust
use semantic_ir_to_go::{compile, GoBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;
let backend = GoBackend::new();
let artifact = backend.compile(&sir_module)?;
```

## Capability declaration

Accepts the full v0 feature set minus `TailCalls` (Go has no TCO) and
`Intrinsics` (empty whitelist in v0).

## Value model

```go
type Value interface{}
type Symbol  struct { Name string }
type Pair    struct { Car, Cdr Value }
type Closure struct { Fn func(args []Value) Value }
```

Single-threaded; symbol interning + globals in module-level maps.

## Block-as-expression

Go has no expression-position blocks, so non-trivial `Block`s lower
to an immediately-invoked function expression:

```go
func() Value {
    x := someExpr
    return _sir_plus([]Value{x, intLit2})
}()
```

## `main` collision

SIR's synthesised `main` is renamed to `_sir_user_main`; the emitter
generates the real `func main()` that calls `_init()` (if present)
then `_sir_user_main()`.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- Sister backends: [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/),
  [`semantic-ir-to-rust`](../semantic-ir-to-rust/),
  [`semantic-ir-to-python`](../semantic-ir-to-python/)
