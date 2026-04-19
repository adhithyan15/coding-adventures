# F07: State Machine Markup Language

## Overview

This spec defines the repo's canonical serialization format for state machines.
The state-machine libraries should be able to:

- build machines in code
- export them to a stable text file
- load the same file back into an equivalent machine
- emit visualization formats such as DOT
- import and export a useful subset of existing standards such as SCXML

The format is called **State Machine Markup v1**. Source files use
`.states.toml` when the file is meant to be parsed by general TOML tooling, and
`.states` when we want the shorter educational extension. Both extensions carry
the same TOML-compatible content.

This is intentionally not a brand-new syntax. TOML gives us a readable existing
text format, and `F03-toml-parser.md` already makes TOML a first-class repo
foundation. The state-machine model and terminology are inspired by SCXML, the
W3C statechart XML recommendation, but narrowed to the automata our library
actually implements today.

## Existing Formats We Should Learn From

### SCXML

SCXML is the closest existing standard. It is a W3C Recommendation for
state-machine notation and execution, with core constructs such as `state`,
`transition`, events, entry actions, exit actions, guards, and document-order
transition priority.

We should borrow these ideas:

- states have stable identifiers
- transitions react to events
- transitions may have guards
- states can have entry and exit actions
- transition order matters when more than one transition can match
- machines should be deterministic unless a nondeterministic kind says
  otherwise
- serialization should be independent from any one host language

We should not adopt full SCXML as our only source format because:

- XML is noisy for the educational examples in this repo
- SCXML is event-statechart oriented, while our foundation library also models
  acceptors such as DFA, NFA, and PDA
- PDA stack operations and automata accepting states need repo-specific
  conventions anyway
- SCXML executable content can embed datamodel-specific expressions, which would
  make cross-language ports drift

The library should still support a **SCXML core profile** for import/export when
the machine fits that model.

### XState JSON

XState shows that state machines can be represented as plain data and used by a
runtime interpreter. It also explicitly aligns with SCXML concepts. The useful
lesson for us is not to copy JavaScript object syntax, but to keep the machine
definition serializable as ordinary data with `id`, `initial`, `states`, and
event-keyed transitions.

### UML/PSSM/XMI

UML state machines and the OMG Precise Semantics of UML State Machines work are
valuable references, especially for formal statechart semantics. Their XMI
interchange model is too broad and tool-heavy for this repo's foundation layer,
so we should treat it as a later interoperability target, not the first
implementation format.

### DOT

DOT is excellent for graph visualization. Our library already exposes graph-like
views, and DOT export should remain a first-class debugging feature. DOT is not
enough as the canonical state-machine format because it does not define
execution semantics, accepting states, stack actions, guards, or tokenizer
effects.

## Design Principles

1. **Round-trip first.** If the library writes a `.states.toml` file and reads
   it back, the resulting machine should be equivalent.
2. **TOML surface, typed core.** The file is TOML-compatible, but every runtime
   lowers it into the same typed `StateMachineDocument` model.
3. **SCXML-compatible where practical.** Use SCXML names and semantics for
   event machines instead of inventing new vocabulary.
4. **No host-language code in files.** Actions and guards are named portable
   operations with structured arguments.
5. **Stable canonical output.** Serializers sort sections predictably so diffs
   stay reviewable.
6. **Typed machine kinds.** DFA, NFA, PDA, modal machines, and later
   statecharts share one document envelope but validate against different rules.
7. **Profiles extend, they do not fork.** Tokenizers, protocol machines, and UI
   machines add profile sections on top of this document model.

## Non-Goals

This spec does not define:

- the full SCXML execution algorithm
- a full UML/XMI interchange implementation
- arbitrary embedded JavaScript, Ruby, Python, Rust, or Go code
- graphical layout metadata beyond optional visualization hints
- tokenizer-specific actions; those live in `F08`

## Canonical Data Model

Every parser lowers source files into this conceptual model:

```text
StateMachineDocument
  format: string
  name: string
  kind: dfa | nfa | pda | modal | statechart | transducer
  version: string?
  profile: string?
  metadata: map<string, value>
  alphabet: list<string>
  stack_alphabet: list<string>
  initial: state_id?
  states: list<StateDefinition>
  transitions: list<TransitionDefinition>
  actions: list<ActionDefinition>
  guards: list<GuardDefinition>
  modes: list<ModeDefinition>
  includes: list<IncludeDefinition>
```

`StateDefinition`:

```text
id: string
initial: bool
accepting: bool
final: bool
external_entry: bool
parent: state_id?
children: list<state_id>
on_entry: list<ActionCall>
on_exit: list<ActionCall>
metadata: map<string, value>
```

`TransitionDefinition`:

```text
id: string?
from: state_id
on: event | epsilon | eof | matcher
to: state_id | list<state_id>
guard: guard_call?
actions: list<ActionCall>
priority: integer?
consume: bool?
stack_pop: stack_symbol?
stack_push: list<stack_symbol>
metadata: map<string, value>
```

The exact host-language structs can be idiomatic, but the fields and behavior
must map back to this model.

## TOML Surface Format

### DFA Example

```toml
format = "state-machine/v1"
name = "turnstile"
kind = "dfa"
initial = "locked"
alphabet = ["coin", "push"]

[[states]]
id = "locked"
initial = true

[[states]]
id = "unlocked"
accepting = true

[[transitions]]
from = "locked"
on = "coin"
to = "unlocked"

[[transitions]]
from = "locked"
on = "push"
to = "locked"

[[transitions]]
from = "unlocked"
on = "coin"
to = "unlocked"

[[transitions]]
from = "unlocked"
on = "push"
to = "locked"
```

### NFA Example

NFA documents allow repeated `(from, on)` pairs, multiple `to` states, and
`epsilon` transitions:

```toml
format = "state-machine/v1"
name = "contains-abc"
kind = "nfa"
initial = "q0"
alphabet = ["a", "b", "c"]

[[states]]
id = "q0"
initial = true

[[states]]
id = "q1"

[[states]]
id = "q2"

[[states]]
id = "q3"
accepting = true

[[transitions]]
from = "q0"
on = "a"
to = ["q0", "q1"]

[[transitions]]
from = "q1"
on = "b"
to = "q2"

[[transitions]]
from = "q2"
on = "c"
to = "q3"

[[transitions]]
from = "q3"
on = "epsilon"
to = "q0"
```

### PDA Example

PDA documents add stack alphabet and stack effects:

```toml
format = "state-machine/v1"
name = "balanced-parens"
kind = "pda"
initial = "scan"
alphabet = ["(", ")"]
stack_alphabet = ["(", "$"]
initial_stack = "$"

[[states]]
id = "scan"
initial = true

[[states]]
id = "accept"
accepting = true

[[transitions]]
from = "scan"
on = "("
to = "scan"
stack_pop = "$"
stack_push = ["(", "$"]

[[transitions]]
from = "scan"
on = "("
to = "scan"
stack_pop = "("
stack_push = ["(", "("]

[[transitions]]
from = "scan"
on = ")"
to = "scan"
stack_pop = "("
stack_push = []

[[transitions]]
from = "scan"
on = "epsilon"
to = "accept"
stack_pop = "$"
stack_push = ["$"]
```

### Modal Machine Example

Modal machines reference named child machines and mode transitions:

```toml
format = "state-machine/v1"
name = "html-tokenizer-modes"
kind = "modal"
initial_mode = "data"

[[modes]]
id = "data"
initial = true
machine = "html-data.states.toml"

[[modes]]
id = "tag_open"
machine = "html-tag-open.states.toml"

[[mode_transitions]]
from = "data"
on = "open_tag"
to = "tag_open"

[[mode_transitions]]
from = "tag_open"
on = "close_angle"
to = "data"
```

## Actions And Guards

Actions and guards use a portable call representation:

```toml
[[actions]]
id = "record-transition"
kind = "emit_trace"

[[guards]]
id = "has-credit"
kind = "register_equals"
register = "credit"
value = true

[[transitions]]
from = "waiting"
on = "vend"
to = "dispensing"
guard = "has-credit"
actions = ["record-transition"]
```

For F01-level DFA/NFA/PDA machines, actions and guards are optional. For
tokenizers and protocol machines, actions become the portable effect vocabulary
that lets a text file produce real output without host-language callbacks.

## SCXML Core Profile

The library should implement these mappings:

| SCXML | State Machine Markup |
|---|---|
| `<scxml initial="s">` | `initial = "s"` |
| `<state id="s">` | `[[states]] id = "s"` |
| `<final id="done">` | `[[states]] id = "done"; final = true` |
| `<transition event="e" target="t">` | `[[transitions]] on = "e"; to = "t"` |
| `<transition cond="...">` | named guard, if the condition is in the supported guard profile |
| `<onentry>` / `<onexit>` | `on_entry` / `on_exit` action calls |
| document order | `priority` |

Initial SCXML import should reject or explicitly mark unsupported:

- `<parallel>`
- `<history>`
- `<invoke>`
- executable `<script>`
- datamodel-specific expressions outside the supported guard/action profile

SCXML export is allowed only when the machine fits the supported profile. For a
plain DFA or modal event machine, export should be straightforward. For PDA and
NFA machines, export should either use repo-specific `ca:` extension attributes
or refuse with a clear diagnostic.

## DOT Export

DOT remains an output format:

```text
DFA -> StateMachineDocument -> DOT
```

DOT import is not part of phase 1. DOT files are graphs, not executable machine
definitions. If we add DOT import later, it should require explicit attributes
for initial, accepting, stack effects, and transition events.

## Serializer API

Each language port should expose the same conceptual operations:

```rust
let dfa = DFA::new(...)?;
let document = dfa.to_document();
let text = document.to_states_toml();

let parsed = StateMachineDocument::from_states_toml(&text)?;
let round_tripped = DFA::from_document(parsed)?;
```

Minimum API:

- `to_document(machine) -> StateMachineDocument`
- `from_document(document) -> Machine`
- `StateMachineDocument::parse_toml(source)`
- `StateMachineDocument::to_toml()`
- `StateMachineDocument::parse_json(source)`
- `StateMachineDocument::to_json()`
- `StateMachineDocument::parse_scxml(source)`
- `StateMachineDocument::to_scxml()`
- `Machine::to_dot()`
- `Machine::validate_document(document)`

JSON support is required because some tooling prefers generated machine-readable
artifacts. TOML remains the hand-authored source format.

## Canonical Output Rules

Serializers must:

- write `format`, `name`, `kind`, and initial fields first
- write arrays in deterministic order
- preserve transition declaration order through `priority`
- omit fields that are empty or false unless required for clarity
- quote all string event names
- write one `[[states]]` table per state
- write one `[[transitions]]` table per transition
- preserve unknown `metadata` keys under a namespaced table such as
  `[metadata.visual]`

Serializers must not:

- reorder transitions in a way that changes priority
- inline host-language code
- silently drop unsupported actions, guards, stack effects, or modes

## Validation Rules

All documents:

- `format` must be `state-machine/v1`
- `kind` must be known
- state IDs must be unique
- transition source and target states must exist
- action and guard references must exist
- include paths must be relative to the current document unless explicitly
  marked as package imports

DFA documents:

- exactly one initial state
- `to` is a single state
- no duplicate `(from, on)` transition pairs
- no `epsilon` transitions
- no stack effects

NFA documents:

- exactly one initial state
- `to` may be one or more states
- duplicate `(from, on)` pairs are allowed
- `epsilon` transitions are allowed
- no stack effects

PDA documents:

- `initial_stack` must exist in `stack_alphabet`
- `stack_pop` and every `stack_push` symbol must be in `stack_alphabet`
- accepting-by-final-state is the default; accepting-by-empty-stack can be added
  later as an explicit option

Modal documents:

- exactly one initial mode
- every mode references an inline or external machine
- mode transitions reference known modes
- mode transition events are declared or inferred from child machine outputs

## Implementation Plan

1. Add this spec as the canonical serialization target.
2. Update `F01-state-machine.md` so `.states` points to this TOML-compatible v1
   format instead of the older sketch examples.
3. Implement `StateMachineDocument` in Rust first.
4. Add `to_document`, `from_document`, `to_toml`, `from_toml`, `to_json`, and
   `from_json` for DFA.
5. Add NFA, PDA, and modal round-tripping.
6. Add SCXML core import/export for deterministic event machines.
7. Keep DOT export as visualization-only output.
8. Build tokenizer profiles from `F08` on top of this document model.

## Test Strategy

- golden TOML round-trip tests for DFA, NFA, PDA, and modal machines
- JSON round-trip tests for the same canonical documents
- validation failures for malformed transitions and unknown references
- SCXML core import tests using small W3C-style examples
- SCXML export tests for deterministic event machines
- DOT snapshot tests for visualization output
- property tests where generated DFAs survive
  `machine -> document -> TOML -> document -> machine`

## Success Criteria

Phase 1 is successful when:

1. the Rust state-machine package can serialize a DFA to `.states.toml`
2. the Rust state-machine package can deserialize the same file into an
   equivalent DFA
3. the same document can be exported as JSON
4. DOT export still works for visualization
5. the format is documented enough for Go and TypeScript ports to follow
6. tokenizer specs can extend this format instead of inventing a separate
   serialization language

## References

- W3C SCXML 1.0, "State Chart XML (SCXML): State Machine Notation for Control
  Abstraction": <https://www.w3.org/TR/scxml/>.
- Graphviz DOT Language: <https://graphviz.org/doc/info/lang.html>.
- OMG Precise Semantics of UML State Machines:
  <https://www.omg.org/spec/PSSM/>.
- XState API documentation, noting SCXML alignment:
  <https://xstate.js.org/api/>.
