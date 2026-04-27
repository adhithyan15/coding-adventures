# state-machine

Formal automata from DFA to PDA, implemented in Rust. A port of the Python `state-machine` package.

## What's in this crate?

| Module     | What it does                                              |
|------------|-----------------------------------------------------------|
| `types`    | Core types: `TransitionRecord`                            |
| `dfa`      | Deterministic Finite Automaton (the workhorse)            |
| `nfa`      | Non-deterministic FA with epsilon transitions             |
| `minimize` | Hopcroft's DFA minimization algorithm                     |
| `pda`      | Pushdown Automaton (finite automaton + stack)              |
| `modal`    | Modal State Machine (multiple DFA sub-machines with mode switching) |
| `transducer` | Ordered effectful state machine for tokenizer-style runtimes |
| `definitions` | Format-agnostic typed definitions for export/compiler inputs |

## Where it fits in the stack

```
Layer D10: State Machines
├── DFA/NFA          — recognize regular languages (regex, tokenizers)
├── PDA              — recognize context-free languages (parsers)
├── Modal            — context-sensitive mode switching
└── Transducer       — emit effects while transitioning (HTML tokenization)
```

The 2-bit branch predictor (D02) is a DFA. The CPU pipeline (D04) is a linear DFA.
Regex engines convert patterns to NFAs, then to DFAs via subset construction.
Parsers use PDAs. HTML tokenizers use effectful transducers, with modal
machines available for simpler mode-switching examples.

The `definitions` module is the bridge between hand-built machines and the
build-time compiler pipeline. It exports deterministic snapshots of DFAs, NFAs,
and PDAs as `StateMachineDefinition` values, and imports validated definitions
back into executable machines. File formats such as State Machine Markup live in
sibling serializer/deserializer crates, so this runtime crate stays focused on
executable automata and typed definitions.

## Usage

```rust
use std::collections::{HashMap, HashSet};
use state_machine::dfa::DFA;

// Classic turnstile example
let mut turnstile = DFA::new(
    HashSet::from(["locked".into(), "unlocked".into()]),
    HashSet::from(["coin".into(), "push".into()]),
    HashMap::from([
        (("locked".into(), "coin".into()), "unlocked".into()),
        (("locked".into(), "push".into()), "locked".into()),
        (("unlocked".into(), "coin".into()), "unlocked".into()),
        (("unlocked".into(), "push".into()), "locked".into()),
    ]),
    "locked".into(),
    HashSet::from(["unlocked".into()]),
).unwrap();

assert!(turnstile.accepts(&["coin"]));
assert!(!turnstile.accepts(&["coin", "push"]));

turnstile.process("coin").unwrap();
assert_eq!(turnstile.current_state(), "unlocked");
```

## Exporting Typed Definitions

```rust
use std::collections::{HashMap, HashSet};
use state_machine::DFA;

let turnstile = DFA::new(
    HashSet::from(["locked".into(), "unlocked".into()]),
    HashSet::from(["coin".into(), "push".into()]),
    HashMap::from([
        (("locked".into(), "coin".into()), "unlocked".into()),
        (("locked".into(), "push".into()), "locked".into()),
        (("unlocked".into(), "coin".into()), "unlocked".into()),
        (("unlocked".into(), "push".into()), "locked".into()),
    ]),
    "locked".into(),
    HashSet::from(["unlocked".into()]),
).unwrap();

let definition = turnstile.to_definition("turnstile");
let imported = DFA::from_definition(&definition).unwrap();

assert_eq!(definition.kind.as_str(), "dfa");
assert_eq!(definition.initial.as_deref(), Some("locked"));
assert!(imported.accepts(&["coin"]));
```

Use the sibling `state-machine-markup-serializer` crate when you want to turn a
definition into `.states.toml` text. Use the sibling
`state-machine-markup-deserializer` crate when tooling needs to read TOML back
into a typed definition before calling `DFA::from_definition`,
`NFA::from_definition`, or `PushdownAutomaton::from_definition`.

## Effectful Transducers

```rust
use std::collections::HashSet;
use state_machine::{
    EffectfulInput, EffectfulMatcher, EffectfulStateMachine, EffectfulTransition,
};

let mut machine = EffectfulStateMachine::new(
    HashSet::from(["data".into(), "done".into()]),
    HashSet::from(["x".into()]),
    vec![
        EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
            .with_effects(&["append_text(current)"]),
        EffectfulTransition::new("data", EffectfulMatcher::End, "done")
            .with_effects(&["flush_text", "emit(EOF)"])
            .consuming(false),
    ],
    "data".into(),
    HashSet::from(["done".into()]),
).unwrap();

let text = machine.process(EffectfulInput::event("x")).unwrap();
assert_eq!(text.effects, vec!["append_text(current)".to_string()]);

let eof = machine.process(EffectfulInput::end()).unwrap();
assert_eq!(eof.effects, vec!["flush_text".to_string(), "emit(EOF)".to_string()]);
assert_eq!(machine.current_state(), "done");
```

This is the primitive we use for HTML-shaped tokenizer work: definitions name
portable effects, while wrapper packages interpret those effects into concrete
tokens and diagnostics.

## Building and testing

```bash
cargo test -p state-machine -- --nocapture
```

## Test coverage

255 tests across unit and integration test suites:
- **types**: 6 unit tests
- **dfa**: 22 unit + 52 integration tests
- **nfa**: 18 unit + 41 integration tests
- **minimize**: 5 unit + 8 integration tests
- **pda**: 17 unit + 33 integration tests
- **modal**: 13 unit + 23 integration tests
- **definitions**: 18 integration tests
- **transducer**: HTML-tokenizer skeleton and definition round-trip tests

Classic examples tested: turnstile, binary divisibility-by-3, 2-bit branch predictor,
balanced parentheses PDA, a^n b^n PDA, HTML tokenizer modal machine, and an
effectful HTML tokenizer skeleton.
