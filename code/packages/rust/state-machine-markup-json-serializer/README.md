# state-machine-markup-json-serializer

Canonical JSON serialization for typed state-machine definitions.

## Where It Fits

The core `state-machine` crate exposes executable automata and
`StateMachineDefinition` values. This crate is the format layer that turns those
definitions into deterministic `.states.json` text for build tooling, source
compiler snapshots, and cross-language package tests.

Transducer transition `actions` and non-default `consume` flags are preserved in
the canonical JSON output so tokenizer definitions can round-trip through the
same typed model as DFA/NFA/PDA machines.

This package intentionally only writes JSON. Reading JSON is a separate
deserializer concern because parsed files are an untrusted boundary and need
their own size limits, validation, and diagnostics.

## Usage

```rust
use std::collections::{HashMap, HashSet};
use state_machine::DFA;
use state_machine_markup_json_serializer::StateMachineJsonSerializer;

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

let json = turnstile.to_definition("turnstile").to_states_json();
assert!(json.contains("\"kind\": \"dfa\""));
```

## Dependencies

- state-machine

## Development

```bash
# Run tests
bash BUILD
```
