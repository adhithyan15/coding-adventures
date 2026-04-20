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
| `definitions` | Format-agnostic typed definitions for export/compiler inputs |

## Where it fits in the stack

```
Layer D10: State Machines
├── DFA/NFA          — recognize regular languages (regex, tokenizers)
├── PDA              — recognize context-free languages (parsers)
└── Modal            — context-sensitive tokenization (HTML mode switching)
```

The 2-bit branch predictor (D02) is a DFA. The CPU pipeline (D04) is a linear DFA.
Regex engines convert patterns to NFAs, then to DFAs via subset construction.
Parsers use PDAs. HTML tokenizers use modal state machines.

The `definitions` module is the bridge from hand-built machines to the
build-time compiler pipeline. It exports deterministic snapshots of DFAs, NFAs,
and PDAs as `StateMachineDefinition` values. File formats such as State Machine
Markup live in sibling serializer/deserializer crates, so this runtime crate
stays focused on executable automata and typed definitions.

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
assert_eq!(definition.kind.as_str(), "dfa");
assert_eq!(definition.initial.as_deref(), Some("locked"));
```

Use the sibling `state-machine-markup-serializer` crate when you want to turn a
definition into `.states.toml` text. Future deserializers will perform the
opposite conversion and hand this crate typed definitions, not raw files.

## Building and testing

```bash
cargo test -p state-machine -- --nocapture
```

## Test coverage

240 tests across unit and integration test suites:
- **types**: 6 unit tests
- **dfa**: 22 unit + 52 integration tests
- **nfa**: 18 unit + 41 integration tests
- **minimize**: 5 unit + 8 integration tests
- **pda**: 17 unit + 33 integration tests
- **modal**: 13 unit + 23 integration tests

Classic examples tested: turnstile, binary divisibility-by-3, 2-bit branch predictor,
balanced parentheses PDA, a^n b^n PDA, HTML tokenizer modal machine.
