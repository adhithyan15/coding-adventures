# F09: State Machine Source Compiler

## Overview

This spec defines the build-time compiler that turns State Machine Markup
documents into ordinary source code for each language port. The goal is to keep
the expressive, inspectable `.states.toml` authoring format from `F07`, while
avoiding production runtime loading of arbitrary text or JSON files.

The production path is:

```text
hand-written machine in code
        -> StateMachineDefinition
        -> optional serializer output: canonical .states.toml / .states.json
        -> deserializer or direct typed definition input
        -> state-machine source compiler
        -> generated Rust / Go / TypeScript / Python / Ruby / ...
        -> normal package compiler or interpreter
        -> statically linked machine tables
```

For Venture, this means the HTML tokenizer definition can remain data-driven,
but the browser binary links generated code. Runtime packages do not need to
parse tokenizer files from disk.

## Why This Exists

Loading parser or tokenizer definitions at runtime creates an unnecessary trust
boundary:

- include resolution can become path traversal if implemented carelessly
- large definitions can be used for memory or CPU exhaustion
- version mismatches are discovered late
- every runtime language must ship a parser for the definition format
- production behavior depends on files outside the compiled package

Build-time compilation moves those risks earlier. The compiler validates the
definition, expands includes, rejects unsafe constructs, and emits plain source
code that is reviewed, tested, and linked like any other package code.

## Relationship To Existing Specs

- `F01-state-machine.md`: defines the core automata library.
- `F07-state-machine-markup-language.md`: defines the canonical typed
  definition model and TOML/JSON file formats.
- `F08-declarative-tokenizer-state-machines.md`: defines tokenizer-specific
  actions and registers, including the future HTML tokenizer profile.

This spec adds the missing build layer between `F07` definitions and production
runtime packages.

## Design Principles

1. **No production runtime deserialization by default.** File parsing is for
   tooling, tests, and build steps.
2. **Static linking wins.** Generated packages expose ordinary constants,
   enums, transition tables, and action identifiers.
3. **One canonical IR.** TOML and JSON both lower to
   `StateMachineDefinition`; code generators consume that typed IR, not raw
   text.
4. **Generated code is boring.** The output should be predictable tables and
   simple constructors, not clever metaprogramming.
5. **Validation before generation.** Codegen never tries to repair malformed
   documents. Invalid input fails the build.
6. **Language runtimes stay small.** Each language gets a table interpreter
   and generated machine definitions, not a complete markup parser in every
   production package.
7. **Examples scale deliberately.** Start with DFA, then NFA, PDA, modal
   machines, transducers, and finally tokenizer profiles.

## Artifacts

The compiler recognizes three artifact classes:

### Authoring Artifacts

These are handwritten or exported for humans:

```text
turnstile.states.toml
balanced-parens.states.toml
html-living.tokenizer.states.toml
```

They are readable, diffable, and may use includes.

### Canonical IR Artifacts

These are normalized build intermediates:

```text
turnstile.states.json
html-living.tokenizer.expanded.states.json
```

They contain no unresolved includes, have deterministic ordering, and are ideal
for snapshot tests. They are not meant to be loaded by production packages. The
canonical JSON writer and bounded JSON reader are separate serializer and
deserializer packages so source compiler crates can consume typed definitions
without depending on TOML writing, TOML parsing, or runtime file loading.

### Generated Source Artifacts

These are checked in only when the package convention calls for generated
source, or produced during package builds otherwise:

```text
src/generated/turnstile_machine.rs
generated/turnstile_machine.go
src/generated/html_living_tokenizer.ts
```

Generated source is what application and wrapper packages link.

## Compiler Pipeline

```text
parse authoring file
  -> resolve includes with safe root checks
  -> validate document shape
  -> lower to canonical StateMachineDefinition
  -> expand profiles, actions, guards, and input classes
  -> emit canonical JSON
  -> emit language-specific source code
  -> run generated-code tests
```

Include resolution must follow the safety rules in `F07`: reject absolute paths,
parent-directory traversal, symlink escapes, and URL-like imports unless the
caller provides an explicit allowlisted resolver.

## Generated Source Shape

For the first implementation slice, Rust generation emits a small module that
reconstructs the validated typed definition and exposes a convenience
constructor for the executable machine kind. This keeps the generated code
boring, reviewable, and statically linked while avoiding any runtime TOML/JSON
file loading:

```rust
pub fn turnstile_definition() -> StateMachineDefinition { /* table data */ }

pub fn turnstile_dfa() -> Result<DFA, String> {
    DFA::from_definition(&turnstile_definition())
}
```

Later optimization passes may lower the same definition into enum-backed static
tables. For a simple DFA, that optimized Rust output should look conceptually
like:

```rust
pub enum TurnstileState {
    Locked,
    Unlocked,
}

pub enum TurnstileEvent {
    Coin,
    Push,
}

pub const TURNSTILE_TRANSITIONS: &[Transition<TurnstileState, TurnstileEvent>] = &[
    Transition { from: TurnstileState::Locked, on: TurnstileEvent::Coin, to: TurnstileState::Unlocked },
    Transition { from: TurnstileState::Locked, on: TurnstileEvent::Push, to: TurnstileState::Locked },
];

pub fn turnstile_machine() -> CompiledDfa<TurnstileState, TurnstileEvent> {
    CompiledDfa::new(
        TurnstileState::Locked,
        &[TurnstileState::Unlocked],
        TURNSTILE_TRANSITIONS,
    )
}
```

Other languages should preserve the same structure in idiomatic form:

- Go: typed `const` state/event identifiers and `[]Transition`
- TypeScript: `as const` state/event unions and frozen transition arrays
- Python: frozen dataclasses or tuples plus enums
- Ruby: frozen hashes and symbols

The generated source must not contain file reads, dynamic eval, arbitrary code
execution, or network access.

Rust end-to-end completion means the compiler test suite proves this full
build-time chain:

```text
manual DFA/NFA/PDA/transducer
  -> StateMachineDefinition
  -> generated Rust module
  -> temporary Rust wrapper crate
  -> cargo test executes the generated constructors
  -> executable automata accept/reject expected inputs or emit expected effects
```

The generated module itself still contains only typed table data and
constructors. Test harnesses may write temporary crates to prove linkability,
but the source compiler API remains file-format and file-system agnostic.

For the first tokenizer-oriented Rust slice, transducer generation emits the
same kind of typed table data as DFA/NFA/PDA generation, plus `actions` and
`consume` fields on each transition. The generated constructor calls
`EffectfulStateMachine::from_definition(...)`, so wrapper packages link static
source and do not load tokenizer definitions from disk at runtime.

## Manual Machine Export Layer

The state-machine libraries also need to export manually constructed machines
into the same definition model:

```text
DFA::new(...)
  -> dfa.to_definition("turnstile")
  -> state-machine-markup-serializer emits .states.toml
  -> state-machine-markup-deserializer reads .states.toml back into a definition
  -> state-machine compiler
  -> generated source
```

This gives us round-trip confidence and lets educational examples graduate from
hand-written code to reusable generated definitions.

Phase 1 export support:

- DFA to `StateMachineDefinition`
- NFA to `StateMachineDefinition`
- deterministic PDA to `StateMachineDefinition`
- deterministic TOML writer in a separate serializer package

Phase 2 export support:

- modal machines with external child-machine references
- modal machines with inline child-machine documents
- JSON writer in a separate serializer package, exposed as
  `to_states_json(definition) -> string`
- JSON reader in a separate deserializer package, exposed as
  `from_states_json(source) -> StateMachineDefinition`
- TOML/JSON parsers in separate deserializer packages for tooling only

Phase 3 export support:

- a Rust `state-machine-source-compiler` package that consumes typed
  `StateMachineDefinition` values and emits deterministic source text
- source code generators for Rust and Go
- source code generators for TypeScript, Python, and Ruby
- generated wrapper package templates

## Example Progression

### Example 1: Turnstile DFA

Purpose: prove the smallest possible round trip.

```text
manual DFA -> definition -> TOML -> generated Rust -> executable DFA
```

### Example 2: Contains-ABC NFA

Purpose: prove nondeterministic transitions and epsilon-free NFA export.

```text
manual NFA -> definition -> TOML -> generated Rust -> NFA runtime
```

### Example 3: Balanced Parentheses PDA

Purpose: prove stack alphabets, stack reads, and stack writes.

```text
manual PDA -> definition -> TOML -> generated Rust -> PDA runtime
```

### Example 4: HTML Mode Skeleton

Purpose: prove mode switching without the full tokenizer action vocabulary.

```text
data mode DFA + tag mode DFA -> modal definition -> generated source
```

### Example 5: Effectful Tokenizer Transducer

Purpose: prove actions, registers, emitted tokens, reconsume, EOF, diagnostics,
and streaming input.

```text
html1.tokenizer.states.toml -> generated tokenizer tables -> HtmlLexer wrapper
```

The Rust foundation starts with a smaller skeleton:

```text
manual html-skeleton transducer
  -> StateMachineDefinition(actions, consume)
  -> generated Rust module
  -> EffectfulStateMachine
  -> wrapper/interpreter emits Text, StartTag, EndTag, EOF tokens
```

### Example 6: WHATWG HTML Tokenizer

Purpose: prove the architecture scales to the living standard's tokenizer
states and character-reference table.

```text
whatwg-html.tokenizer.states.toml
  -> expanded canonical JSON
  -> generated source tables
  -> thin HtmlLexer wrapper
  -> tokenizer fixture compatibility tests
```

## Wrapper Package Pattern

Generated definitions should be wrapped by small, stable packages:

```text
generated html tokenizer tables
        -> generic tokenizer runtime
        -> HtmlLexer wrapper
```

The wrapper owns:

- friendly public names
- token type conversions
- package-specific docs and examples
- streaming API shape

The wrapper must not reimplement the state machine.

## Validation

The compiler must reject:

- unknown states, events, actions, guards, registers, or token fields
- unresolved includes
- unsafe include paths
- duplicate deterministic transitions
- missing initial states
- invalid stack symbols
- profile features unsupported by the target language generator
- generated identifiers that would collide after language-specific casing
- generated identifiers that are reserved words in the target language
- unsupported source generator targets or machine kinds for the selected phase

The compiler should warn about:

- unreachable states
- accepting states that cannot be reached
- unused actions or guards
- very large generated tables
- source-output drift when generated files are checked in

## Test Strategy

- definition export tests for manually constructed DFA, NFA, and PDA examples
- canonical TOML snapshot tests
- canonical JSON snapshot tests once JSON export exists
- generated Rust source snapshot tests
- generated Rust compile-and-run tests for DFA, NFA, and PDA behavior
- generated Go source snapshot tests
- compile-and-run tests for generated simple machines
- tokenizer profile tests before attempting full HTML
- fixture-based HTML tokenizer tests before replacing hand-written lexers

## Implementation Plan

1. Update `F07` so runtime loading is described as tooling-only by default.
2. Add `StateMachineDefinition` export support to the Rust state-machine
   package.
3. Add deterministic TOML output for DFA/NFA/PDA definitions in a separate
   serializer package.
4. Add tests covering turnstile DFA, contains-abc NFA, and balanced-parens PDA.
5. Add bounded canonical JSON deserialization for tooling snapshots.
6. Add a Rust source compiler package that accepts validated typed definitions
   and emits deterministic Rust source without file IO.
7. Add Go source generation.
8. Add tokenizer-profile source generation.
9. Use the generated tokenizer path to rebuild the HTML 1.0 lexer wrapper.
10. Expand toward the WHATWG HTML tokenizer.

## Success Criteria

Phase 1 succeeds when:

1. manually constructed Rust DFA/NFA/PDA machines export to
   `StateMachineDefinition`
2. those definitions write deterministic `.states.toml` through a separate
   serializer package
3. tests prove simple, nondeterministic, and stack-machine examples
4. the specs clearly separate authoring/build inputs from production linked
   source code

Later phases succeed when a generated source definition can fully replace a
hand-written lexer state table without changing the wrapper package API.
