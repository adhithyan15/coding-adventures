# state-machine-source-compiler

`state-machine-source-compiler` is the build-time bridge between State Machine
Markup definitions and statically linked Rust code.

The package accepts the typed `StateMachineDefinition` model from the core
`state-machine` crate. It does not read TOML, JSON, or any other file format.
Parsers and deserializers stay in their own tooling packages so production
wrappers can link ordinary generated source instead of loading untrusted files
at runtime.

## What It Emits

The first backend emits Rust modules with two pieces:

- a `<machine>_definition()` function that reconstructs the typed definition
- a kind-specific convenience function such as `<machine>_dfa()` or
  `<machine>_transducer()` that calls the corresponding executable importer

Example:

```rust
use state_machine_source_compiler::to_rust_source;

let source = to_rust_source(&definition)?;
assert!(source.contains("pub fn turnstile_definition()"));
```

The generated source contains table data and constructors only. It does not
perform file IO, dynamic evaluation, network access, or format parsing.

## End-To-End Rust Proof

The test suite also proves the Rust path as linked code:

```text
StateMachineDefinition
        -> generated Rust module
        -> temporary wrapper crate
        -> cargo test
        -> executable DFA/NFA/PDA behavior and transducer effects
```

Those temporary crates live only in tests. The public compiler API still returns
source text and does not read or write files.

## Fit In The Stack

```text
.states.toml / .states.json tooling input
        -> StateMachineDefinition
        -> state-machine-source-compiler
        -> generated Rust source
        -> wrapper package links generated module
```

The Rust backend currently supports DFA, NFA, PDA, and effectful transducer
definitions. Future slices can add Go, TypeScript, Python, Ruby, and
tokenizer-specific wrapper generators while preserving the same typed
definition input boundary.
