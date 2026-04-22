# F07: State Machine Markup Language

## Overview

This spec defines the repo's canonical serialization format for typed
state-machine definitions. The state-machine ecosystem should be able to:

- build machines in code
- export them to a format-agnostic `StateMachineDefinition`
- serialize definitions to a stable text file in a separate serializer package
- deserialize the same file back into an equivalent definition in a separate
  deserializer package
- compile validated definitions into language-specific source code
- emit visualization formats such as DOT
- import and export a useful subset of existing standards such as SCXML

The format is called **State Machine Markup v1**. Hand-authored source files use
`.states.toml` when the file is meant to be parsed by general TOML tooling, and
`.states` when we want the shorter educational extension. Both extensions carry
the same TOML-compatible content. Build tooling may also emit canonical
`.states.json` artifacts as normalized machine-readable snapshots of the same
typed definition model.

This is intentionally not a brand-new syntax. TOML gives us a readable existing
text format, and `F03-toml-parser.md` already makes TOML a first-class repo
foundation. The state-machine model and terminology are inspired by SCXML, the
W3C statechart XML recommendation, but narrowed to the automata our library
actually implements today.

Production packages should not load arbitrary `.states.toml` or JSON files at
runtime by default. Runtime deserialization is a tooling, test, and development
feature. Production wrappers should link generated source emitted by the
build-time compiler described in `F09-state-machine-source-compiler.md`.

The core `state-machine` library must not own TOML, JSON, SCXML, or source-code
serialization. Its boundary is typed definitions: states, transitions, alphabets,
machine kinds, and stack effects. Format-specific packages sit around that
definition layer:

```text
state-machine                  -> executable automata + StateMachineDefinition
state-machine-markup-serializer -> StateMachineDefinition -> .states.toml
state-machine-markup-deserializer -> .states.toml -> StateMachineDefinition
state-machine-markup-json-serializer -> StateMachineDefinition -> .states.json
state-machine-markup-json-deserializer -> .states.json -> StateMachineDefinition
state-machine-source-compiler  -> StateMachineDefinition -> static source
```

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

1. **Round-trip first.** If the serializer writes a `.states.toml` file and the
   deserializer reads it back, the resulting definition should be equivalent.
2. **TOML surface, typed core.** The file is TOML-compatible, but every runtime
   lowers it into the same typed `StateMachineDefinition` model.
3. **SCXML-compatible where practical.** Use SCXML names and semantics for
   event machines instead of inventing new vocabulary.
4. **No host-language code in files.** Actions and guards are named portable
   operations with structured arguments.
5. **Stable canonical output.** Serializers sort sections predictably so diffs
   stay reviewable.
6. **Typed machine kinds.** DFA, NFA, PDA, modal machines, and later
   statecharts share one definition model but validate against different rules.
7. **Profiles extend, they do not fork.** Tokenizers, protocol machines, and UI
   machines add profile sections on top of this definition model.
8. **Separate read and write boundaries.** Serialization and deserialization are
   different packages. The read path is a trust boundary and deserves its own
   validation, limits, and tests.

## Non-Goals

This spec does not define:

- the full SCXML execution algorithm
- a full UML/XMI interchange implementation
- arbitrary embedded JavaScript, Ruby, Python, Rust, or Go code
- graphical layout metadata beyond optional visualization hints
- tokenizer-specific actions; those live in `F08`

## Canonical Data Model

Every parser lowers source files into this conceptual model. The serialized file
has a `format` field, but the in-memory definition handed to the core
state-machine layer does not need to remember which file format produced it.

```text
StateMachineDefinition
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

## JSON Surface Format

JSON is a canonical build artifact, not the preferred human authoring format.
It exists so tools can snapshot expanded definitions, compare generated
intermediates, and hand the same typed model to source compilers without keeping
TOML syntax around.

The JSON surface uses the same root fields, state objects, and transition
objects as the TOML surface:

```json
{
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
    {"from": "locked", "on": "push", "to": "locked"},
    {"from": "unlocked", "on": "coin", "to": "unlocked"},
    {"from": "unlocked", "on": "push", "to": "locked"}
  ]
}
```

JSON transition `to` follows the same compact rule as TOML: single-target DFA
and PDA transitions write a string, while multi-target NFA transitions write a
string array. Epsilon transitions are written as `"on": null` at the JSON file
boundary. That differs from TOML's `"epsilon"` spelling on purpose: JSON can
represent absence directly, so a real event named `"epsilon"` remains distinct
from the typed in-memory `None` used for epsilon. Import boundaries must
continue rejecting empty-string event aliases.

The first JSON serializer profile covers the same phase 1 DFA, NFA, and PDA
fields as the TOML serializer:

- root string fields: `format`, `name`, `kind`, `initial`, `initial_stack`
- root string arrays: `alphabet`, `stack_alphabet`
- `states` array entries with `id`, `initial`, `accepting`, `final`, and
  `external_entry`
- `transitions` array entries with `from`, `on`, `to`, `stack_pop`, and
  `stack_push`

The JSON serializer must be deterministic and dependency-light. A hand-written
writer is acceptable for the phase 1 subset because it only emits strings,
booleans, arrays, and objects from trusted typed definitions.

The JSON deserializer is a separate trust boundary. It must reject unknown
fields, duplicate object keys, numbers, oversized sources, excessive nesting,
oversized arrays, malformed unicode escapes, and non-phase-1 value shapes before
constructing a `StateMachineDefinition`. It then runs the same semantic
definition validation used by TOML input, including state references, alphabet
membership, stack alphabet membership, DFA determinism, PDA stack-pop rules, and
unsupported-kind rejection. JSON `null` is accepted only as the transition `on`
value that maps to typed epsilon; a string event named `"epsilon"` remains a
normal alphabet symbol.

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
DFA -> StateMachineDefinition -> DOT
```

DOT import is not part of phase 1. DOT files are graphs, not executable machine
definitions. If we add DOT import later, it should require explicit attributes
for initial, accepting, stack effects, and transition events.

## Package Boundaries

Each language port should expose the same conceptual operations, but in separate
packages:

```rust
let dfa = DFA::new(...)?;
let definition = dfa.to_definition("turnstile");
let text = state_machine_markup_serializer::to_states_toml(&definition);

let parsed = state_machine_markup_deserializer::from_states_toml(&text)?;
let round_tripped = DFA::from_definition(parsed)?;
```

Minimum API:

- core state-machine: `to_definition(machine) -> StateMachineDefinition`
- core state-machine: `from_definition(definition) -> Machine`
- TOML serializer: `to_states_toml(definition) -> string`
- TOML deserializer: `from_states_toml(source) -> StateMachineDefinition`
- JSON serializer: `to_states_json(definition) -> string`
- JSON deserializer: `from_states_json(source) -> StateMachineDefinition`
- source compiler: `to_source(definition, target_language)`
- SCXML serializer: `to_scxml(definition) -> string`
- SCXML deserializer: `from_scxml(source) -> StateMachineDefinition`
- `Machine::to_dot()`
- `Machine::validate_definition(definition)`

JSON support is required because some tooling prefers generated
machine-readable artifacts. TOML remains the hand-authored source format.
Source generation is the production artifact path: applications link generated
code rather than loading TOML or JSON dynamically.

## Definition Import Rules

The core state-machine library owns conversion between typed definitions and
executable machines. That import layer is intentionally not a file parser: it
receives an already-built `StateMachineDefinition`, validates the shape required
by the target machine family, and delegates to the normal eager constructors.

Phase 1 imports cover DFA, NFA, and PDA definitions:

- `DFA::from_definition(definition)` accepts only `kind = "dfa"`, requires one
  initial state, requires every transition to have exactly one target, rejects
  epsilon transitions, rejects duplicate `(from, on)` pairs, and rejects all
  stack effects.
- `NFA::from_definition(definition)` accepts only `kind = "nfa"`, requires one
  initial state, allows epsilon transitions, allows one or more targets per
  transition, rejects empty-string events because epsilon is represented by
  `None`, and rejects all stack effects.
- `PushdownAutomaton::from_definition(definition)` accepts only `kind = "pda"`,
  requires one initial state and one initial stack symbol, requires every
  transition to have exactly one target and one `stack_pop` symbol, validates
  every stack symbol against `stack_alphabet`, and preserves epsilon
  transitions as `None`.

Importing a definition should produce the same observable behavior as the
machine that exported it:

```text
machine -> StateMachineDefinition -> executable machine
```

When the definition came from a serializer/deserializer pair, the complete
tooling round trip should also preserve behavior:

```text
machine -> definition -> .states.toml -> definition -> executable machine
```

Modal, statechart, transducer, action, and guard imports are later phases. Until
their runtime semantics are implemented in the core library, executable-machine
imports must reject those definitions with explicit diagnostics instead of
silently dropping behavior.

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

For the phase 1 Rust TOML and JSON serializers, the executable machine export
helpers already sort states, alphabets, and transition sets before constructing
the typed definition. The JSON serializer must also defensively sort set-like
arrays that can be constructed by hand, including `alphabet`, `stack_alphabet`,
and multi-target NFA `to` arrays. Order-sensitive arrays such as PDA
`stack_push` must stay in declaration order. Serializer packages may
defensively sort states and transitions by their stable identifiers when no
explicit `priority` field exists. Once prioritized statecharts and tokenizers
land, serializers must prefer the explicit priority/order field over lexical
sorting.

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
- include resolution must canonicalize paths against the current document or
  package root, reject absolute paths, reject parent-directory traversal,
  reject symlink escapes outside the allowed root, and reject URL-like imports
  unless a caller provides an explicit allowlisted resolver

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

## Phase 1 Deserializer Profile

The first deserializer should be deliberately stricter than the full future
format. It accepts the deterministic TOML-compatible subset emitted by the v1
serializer:

- root string fields: `format`, `name`, `kind`, `initial`, `initial_stack`
- root string arrays: `alphabet`, `stack_alphabet`
- repeated `[[states]]` tables with `id`, `initial`, `accepting`, `final`, and
  `external_entry`
- repeated `[[transitions]]` tables with `from`, `on`, `to`, `stack_pop`, and
  `stack_push`
- string, boolean, and string-array values only

The phase 1 deserializer must reject unsupported tables such as `[[actions]]`,
`[[guards]]`, `[[modes]]`, includes, dotted keys, inline tables, numeric values,
duplicate keys in the same table, duplicate states, malformed strings, and
documents with trailing garbage. It must also apply bounded input limits before
allocating untrusted content into the typed definition layer. These limits are
not part of the abstract format; they are implementation guardrails for the
runtime deserialization package.

## Implementation Plan

1. Add this spec as the canonical serialization target.
2. Update `F01-state-machine.md` so `.states` points to this TOML-compatible v1
   format instead of the older sketch examples.
3. Implement `StateMachineDefinition` in the Rust `state-machine` package first.
4. Add `to_definition` export helpers for DFA, NFA, and PDA.
5. Add a Rust `state-machine-markup-serializer` package for deterministic
   `.states.toml` output.
6. Add a Rust `state-machine-markup-deserializer` package for TOML input and
   validation.
7. Add executable-machine imports for DFA, NFA, and PDA definitions so
   serializer/deserializer round trips can return to runnable machines.
8. Add a Rust `state-machine-markup-json-serializer` package for deterministic
   `.states.json` output.
9. Add a Rust `state-machine-markup-json-deserializer` package for bounded
   `.states.json` input and typed-definition validation.
10. Add modal round-tripping through definitions once modal definitions can
   represent child-machine references and mode transitions.
11. Add SCXML core import/export packages for deterministic event machines.
12. Keep DOT export as visualization-only output.
13. Add the build-time source compiler from
   `F09-state-machine-source-compiler.md`.
14. Build tokenizer profiles from `F08` on top of this definition model.

## Test Strategy

- definition export tests for DFA, NFA, PDA, and modal machines
- golden TOML serializer tests for those definitions
- TOML deserializer round-trip tests in the deserializer package
- JSON round-trip tests for the same canonical definitions
- validation failures for malformed transitions and unknown references
- SCXML core import tests using small W3C-style examples
- SCXML export tests for deterministic event machines
- DOT snapshot tests for visualization output
- property tests where generated DFAs survive
  `machine -> definition -> TOML -> definition -> machine`

## Success Criteria

Phase 1 is successful when:

1. the Rust state-machine package can export DFA/NFA/PDA machines to typed
   `StateMachineDefinition` values
2. a separate Rust serializer package can serialize those definitions to
   deterministic `.states.toml`
3. a separate Rust deserializer package can deserialize the same file into an
   equivalent definition
4. the same definition can be exported as JSON by a separate serializer package
5. the same definition can be compiled into static source code for at least one
   target language
6. DOT export still works for visualization
7. the format is documented enough for Go and TypeScript ports to follow
8. tokenizer specs can extend this format instead of inventing a separate
   serialization language

## References

- W3C SCXML 1.0, "State Chart XML (SCXML): State Machine Notation for Control
  Abstraction": <https://www.w3.org/TR/scxml/>.
- Graphviz DOT Language: <https://graphviz.org/doc/info/lang.html>.
- OMG Precise Semantics of UML State Machines:
  <https://www.omg.org/spec/PSSM/>.
- XState API documentation, noting SCXML alignment:
  <https://xstate.js.org/api/>.
