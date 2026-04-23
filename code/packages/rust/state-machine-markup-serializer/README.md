# state-machine-markup-serializer

State Machine Markup serialization for typed state-machine definitions.

## Where It Fits

The core `state-machine` crate exposes executable automata and
`StateMachineDefinition` values. This crate is the format layer that turns those
definitions into deterministic `.states.toml` text. Runtime automata never need
to know about TOML, JSON, SCXML, or files.

The serializer preserves transducer transition effects as portable `actions`
arrays and emits `consume = false` only when a transition does not advance the
input cursor.

Deserialization intentionally belongs in a separate sibling package so the read
path can have its own validation, trust-boundary checks, and tests.

## Usage

```rust
use std::collections::{HashMap, HashSet};
use state_machine::DFA;
use state_machine_markup_serializer::StateMachineMarkupSerializer;

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

let text = turnstile.to_definition("turnstile").to_states_toml();
assert!(text.contains("kind = \"dfa\""));
```

## Dependencies

- state-machine

## Development

```bash
# Run tests
bash BUILD
```
