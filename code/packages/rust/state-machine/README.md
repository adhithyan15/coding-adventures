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
| `document` | Shared export model for `.states.toml` and future source generation |

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

The `document` module is the bridge from hand-built machines to the build-time
compiler pipeline. It exports deterministic snapshots of DFAs, NFAs, and PDAs as
`StateMachineDocument` values or `.states.toml` text. Production packages can
compile those documents into static source code later, instead of loading
machine definitions from text at runtime.

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

## Exporting State Machine Markup

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

let markup = turnstile.to_states_toml("turnstile");
assert!(markup.contains("kind = \"dfa\""));
```

The exported text is deterministic: states, alphabets, and transitions are
sorted so generated files remain reviewable and reproducible.

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
