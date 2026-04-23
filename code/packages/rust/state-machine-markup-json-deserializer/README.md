# state-machine-markup-json-deserializer

Strict JSON deserialization for typed state-machine definitions.

## Where It Fits

The core `state-machine` crate stays format-agnostic. This package owns the
tooling read boundary for canonical `.states.json` snapshots:

```text
.states.json -> state-machine-markup-json-deserializer -> StateMachineDefinition
```

Runtime browser packages should still link generated source instead of loading
JSON files in production. This reader exists for build tools, tests, and source
compiler pipelines that need to validate canonical snapshots before generating
static code.

The reader accepts transducer transition `actions` and `consume` flags, and now
also lowers lexer-profile metadata such as token/register/input declarations,
typed matcher objects, fixtures, and transition guards before delegating
semantic checks to the shared markup validator.

## Usage

```rust
use state_machine::DFA;
use state_machine_markup_json_deserializer::from_states_json;

let source = r#"{
  "format": "state-machine/v1",
  "name": "turnstile",
  "kind": "dfa",
  "initial": "locked",
  "alphabet": ["coin", "push"],
  "states": [
    {"id": "locked", "initial": true},
    {"id": "unlocked", "accepting": true}
  ],
  "transitions": [
    {"from": "locked", "on": "coin", "to": "unlocked"},
    {"from": "locked", "on": "push", "to": "locked"}
  ]
}"#;

let definition = from_states_json(source).unwrap();
let machine = DFA::from_definition(&definition).unwrap();
assert!(machine.accepts(&["coin"]));
```

## Security Profile

The parser accepts only the current bounded State Machine Markup JSON profile:
objects, arrays, strings, booleans, and `null` for generic transition epsilon.
It rejects numbers, unknown fields, duplicate object keys, excessive nesting,
oversized sources, and oversized arrays before returning a typed definition.
Lexer-profile matchers travel as small JSON objects such as
`{"literal":"<"}` or `{"eof":true}`.

## Dependencies

- state-machine
- state-machine-markup-deserializer

## Development

```bash
# Run tests
bash BUILD
```
