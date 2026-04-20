# F08: Declarative Tokenizer State Machines

## Overview

This spec defines the **tokenizer profile** for the State Machine Markup
Language in `F07-state-machine-markup-language.md`. The first target is HTML
tokenization, but the design is generic enough for XML-ish formats, browser
protocols, programming language lexers with modes, and any stream format where
the next token depends on the current lexical state.

The tokenizer profile is intended to be compiled by
`F09-state-machine-source-compiler.md` before production use. The `.states.toml`
file is the authoring artifact, canonical JSON is a normalized build artifact,
and generated source code is what browser and wrapper packages should link.

The existing `F01-state-machine.md` package gives us formal automata, and `F07`
defines how those machines serialize and deserialize. HTML tokenization needs
one extra profile layer: transitions do not only change state. They also build
tokens, append text, track temporary buffers, report parse errors, reconsume the
current input character in a new state, and sometimes call a shared submachine
such as character-reference parsing.

So the answer to "can the states and transitions live purely in a text file?"
is:

- **Yes**, if the file is a `state-machine/v1` document with a tokenizer
  profile and a fixed portable action vocabulary.
- **No**, if "purely" means a bare DFA transition table with no registers,
  buffers, token-emission actions, EOF handling, or reconsume semantics.

The goal is a declarative file that every language port can load and execute the
same way, without embedding arbitrary Rust, Go, Ruby, Python, or TypeScript code
inside the machine definition.

## Why This Exists

HTML is not a clean context-free language in the way a teaching compiler grammar
usually is. Modern browser parsing is split into:

1. **Input preprocessing**: bytes become Unicode scalar values.
2. **Tokenization**: a state machine turns characters into tokens.
3. **Tree construction**: insertion modes, stacks, active formatting elements,
   and error-recovery algorithms turn tokens into a DOM-like tree.

The current `TE04-html1.0-lexer.md` spec hand-describes a small HTML 1.0 lexer.
That is useful as a first browser milestone, but it does not scale elegantly to
the WHATWG HTML Living Standard tokenizer, which currently defines dozens of
named tokenizer states such as data, tag open, attribute name, script data,
doctype, CDATA, and character-reference states.

We need an intermediate foundation package that lets us write those states once
as data, validate them, test them, and run them from every implementation
language.

## Relationship To Existing Specs

- `F01-state-machine.md`: provides the formal automata vocabulary and modal
  machine foundation.
- `F07-state-machine-markup-language.md`: defines the generic `.states.toml`
  document envelope, serializer/deserializer contract, SCXML inspiration, and
  DOT export boundary.
- `F04-lexer-pattern-groups.md`: supports grouped pattern lexing with callbacks;
  this spec is lower level and more deterministic, for byte/code-point state
  machines like HTML.
- `F06-haskell-layout-mode.md`: handles a layout-sensitive token transform after
  physical tokenization; this spec handles the tokenization loop itself.
- `TE04-html1.0-lexer.md`: should eventually become a thin wrapper around an
  `html1.tokenizer.states.toml` definition.
- `TE05-html1.0-parser.md`: consumes tokens from this system but remains a
  separate tree-construction concern.

## Design Principles

1. **Profile, not fork.** Tokenizers extend the `state-machine/v1` document
   model instead of inventing a separate serialization language.
2. **Portable by construction.** A definition loaded in Rust must behave the
   same way when loaded in Go, TypeScript, Ruby, Python, or future ports.
3. **No arbitrary host callbacks in definition files.** Escape hatches are how
   specs become unportable. Add new built-in actions instead.
4. **Streaming by default.** Tokenizers must accept network-fed chunks without
   requiring the full document in memory.
5. **Error recovery is normal behavior.** HTML tokenization must produce tokens
   and diagnostics for invalid input instead of failing fast.
6. **Traceability matters.** A developer should be able to ask, "which state
   consumed this character and why did it emit this token?"
7. **Versioned standards are first-class.** HTML 1.0, historical HTML profiles,
   and the living standard can share common definitions while evolving
   separately.

## Non-Goals

This spec does not define:

- DOM tree construction
- CSS or JavaScript parsing inside `<style>` or `<script>`
- network character encoding detection
- browser scripting reentrancy
- a complete WHATWG tokenizer definition in phase 1
- arbitrary regex-based lexing for programming languages

Those can sit above or beside this package. This layer only defines an
effectful state-machine tokenizer runtime and a profile for the generic `F07`
file format.

## Core Model

A tokenizer definition is a `StateMachineDocument` with
`profile = "tokenizer/v1"`. It contains the generic `F07` fields plus:

- token type declarations
- named input classes
- registers and buffers
- validation expectations and fixture hooks

At runtime, the tokenizer owns:

- an input cursor
- the current code point or EOF sentinel
- the current state
- an output token queue
- a diagnostics list
- declared registers such as `current_token`, `current_attribute`,
  `temporary_buffer`, `return_state`, and `last_start_tag_name`
- source position tracking for line, column, and byte/code-point offset

Each loop iteration:

1. Reads the current code point, or EOF if the stream is closed.
2. Finds the first matching transition in the current state.
3. Consumes the code point unless the transition says `consume: false` or an
   action requests reconsume.
4. Executes actions in declaration order.
5. Switches to the target state.
6. Emits zero or more tokens.

The machine is deterministic because transition matching is ordered and
`anything`/`anything_else` matchers are only valid as the last transition in a
state.

## File Format

The canonical source format is `.tokenizer.states.toml`. It is a
TOML-compatible `state-machine/v1` document with `profile = "tokenizer/v1"`.
The shorter `.tokenizer` extension may be accepted by repo tools, but writers
should prefer `.tokenizer.states.toml` for clarity and easy TOML tooling.

Example filename:

```text
code/tokenizers/html/html1.tokenizer.states.toml
```

The first version uses the same document envelope as `F07`:

```toml
format = "state-machine/v1"
profile = "tokenizer/v1"
name = "html1-tokenizer"
kind = "transducer"
initial = "data"
done = "done"
includes = ["html-common-inputs.states.toml"]

[[tokens]]
name = "Text"
fields = ["data"]

[[tokens]]
name = "StartTag"
fields = ["name", "attributes", "self_closing"]

[[tokens]]
name = "EndTag"
fields = ["name"]

[[tokens]]
name = "Comment"
fields = ["data"]

[[tokens]]
name = "Doctype"
fields = ["name", "force_quirks"]

[[tokens]]
name = "EOF"
fields = []

[[inputs]]
id = "ascii_whitespace"
matcher = { one_of = "\t\n\f\r " }

[[inputs]]
id = "ascii_upper"
matcher = { range = ["A", "Z"] }

[[inputs]]
id = "ascii_lower"
matcher = { range = ["a", "z"] }

[[inputs]]
id = "ascii_alpha"
matcher = { any_of_classes = ["ascii_upper", "ascii_lower"] }

[[registers]]
id = "text_buffer"
type = "string"

[[registers]]
id = "current_token"
type = "token?"

[[registers]]
id = "current_attribute"
type = "attribute?"

[[registers]]
id = "temporary_buffer"
type = "string"

[[registers]]
id = "return_state"
type = "state?"

[[registers]]
id = "last_start_tag_name"
type = "string?"

[[states]]
id = "data"
initial = true

[[states]]
id = "done"
final = true

[[transitions]]
from = "data"
matcher = { literal = "<" }
to = "tag_open"
actions = ["flush_text"]

[[transitions]]
from = "data"
matcher = { literal = "&" }
to = "data"
actions = [
  "set_return_state(data)",
  "consume_character_reference(text)",
]

[[transitions]]
from = "data"
matcher = { literal = "\\0" }
to = "data"
actions = [
  "parse_error(unexpected-null-character)",
  "append_text_replacement",
]

[[transitions]]
from = "data"
matcher = { eof = true }
to = "done"
actions = ["flush_text", "emit(EOF)"]

[[transitions]]
from = "data"
matcher = { anything = true }
to = "data"
actions = ["append_text(current)"]
```

The compact action-call strings above are a human-friendly TOML surface. The
canonical JSON form expands them into structured action calls with `name` and
`args` fields.

## Matchers

Matchers describe what a transition can see at the current input position.
Every implementation must support:

- `"<literal>"`: match one literal code point or literal string
- `eof`: match the end-of-file sentinel
- `anything`: match any non-EOF code point
- `anything_else`: alias for `anything`, intended for readability after
  specific cases
- `class(name)`: match a named input class
- `any_of_classes([name, ...])`: match any one of several named input classes
- `not_class(name)`: match any non-EOF code point outside a class
- `one_of("...")`: match one code point from a literal set
- `range("A", "Z")`: match an inclusive code-point range
- `lookahead("<literal>")`: match without consuming
- `lookahead_ascii_case_insensitive("<literal>")`: match ASCII-insensitively
  without consuming

The file parser lowers these matchers into a canonical AST so language ports do
not have to preserve the original syntax.

## Actions

Actions are deliberately small and boring. Boring is the portability tax we pay
once so every runtime behaves the same way.

### Cursor And State Actions

- `reconsume_in(state)`: do not advance the cursor; run the same code point
  again in `state`.
- `set_return_state(state)`: store a state name for submachines that return to
  their caller.
- `goto_return_state`: switch to the stored return state.
- `advance(count)`: consume additional code points after a lookahead match.

### Text Actions

- `append_text(current)`: append the current code point to `text_buffer`.
- `append_text("<literal>")`: append a literal string to `text_buffer`.
- `append_text_replacement`: append U+FFFD.
- `flush_text`: emit a `Text` token if `text_buffer` is not empty.
- `emit_current_as_text`: append the current code point as text and flush.

### Token Construction Actions

- `create_start_tag`: set `current_token` to a new start tag token.
- `create_end_tag`: set `current_token` to a new end tag token.
- `create_comment`: set `current_token` to a new comment token.
- `create_doctype`: set `current_token` to a new doctype token.
- `append_tag_name(current_lowercase)`: append the lowercased current code point
  to the current tag token name.
- `append_tag_name(current)`: append the current code point without casing.
- `start_attribute`: create an empty current attribute on the current tag.
- `append_attribute_name(current_lowercase)`: append to the current attribute
  name.
- `append_attribute_value(current)`: append to the current attribute value.
- `commit_attribute`: attach the current attribute to the current tag.
- `set_self_closing`: mark the current tag token as self-closing.
- `emit_current_token`: emit `current_token` and clear it.
- `record_last_start_tag`: remember the emitted start tag name for later raw
  text and script checks.

### Temporary Buffer Actions

- `clear_temporary_buffer`
- `append_temporary(current)`
- `append_temporary(current_lowercase)`
- `emit_temporary_as_text`
- `matches_last_start_tag`: a predicate action result used by guarded
  transitions in RCDATA, RAWTEXT, and script states.

### Diagnostics Actions

- `parse_error(code)`: add a recoverable diagnostic at the current source
  position.
- `parse_error_if(condition, code)`: add a diagnostic when a guard is true.

### Submachine Actions

- `consume_character_reference(destination)`: run the character-reference
  machine and append its result to either `text`, `attribute_value`, or
  `temporary_buffer`.
- `call(machine_name)`: run a named submachine that communicates only through
  declared registers and output tokens.

Submachines must be declared in the same file or an included file. They cannot
call arbitrary host-language functions.

## Guards

Some HTML tokenizer states depend on small pieces of parser/tokenizer context.
Transitions may include guards:

```text
state rcdata_end_tag_name:
  on class(ascii_alpha) when temporary_is_possible_end_tag:
    actions:
      - append_temporary(current)
      - append_tag_name(current_lowercase)
    goto: rcdata_end_tag_name

  on ">" when current_tag_name_equals_last_start_tag:
    actions: emit_current_token
    goto: data
```

The first version supports only named built-in guards:

- `current_tag_name_equals_last_start_tag`
- `temporary_is_possible_end_tag`
- `current_token_is_start_tag`
- `current_token_is_end_tag`
- `current_attribute_is_duplicate`
- `scripting_enabled`
- `in_foreign_content`

If a future HTML state needs a new guard, we add it to the portable vocabulary
and implement it in every runtime. Definition files must not include host
language expressions.

## Streaming Semantics

The tokenizer supports chunked input:

```text
let tokenizer = Tokenizer::new(definition)
tokenizer.push("<di")
tokenizer.push("v>Hello")
tokenizer.finish()
```

A transition that needs more input for a literal or lookahead matcher enters an
internal `need_more_input` condition instead of guessing. When the caller pushes
another chunk, matching resumes from the same state and cursor. When the caller
calls `finish`, unresolved lookahead is evaluated against EOF.

EOF is not a byte and not a code point. It is a sentinel that can only be
matched by `eof`.

## Tracing

Every runtime should be able to produce optional transition traces:

```text
offset=0 state=data input="<" transition=on("<") actions=[flush_text] goto=tag_open
offset=1 state=tag_open input="d" transition=on(class(ascii_alpha)) actions=[create_start_tag,reconsume_in(tag_name)] goto=tag_name
offset=1 state=tag_name input="d" transition=on(class(tag_name_char)) actions=[append_tag_name(current_lowercase)] goto=tag_name
```

Tracing is off by default. It is required for tests and debugging because
declarative machines are otherwise hard to inspect when a single transition is
wrong.

## HTML Phase 1 Profile

The first HTML definition should target the current HTML 1.0 package shape:

- `Text`
- `StartTag`
- `EndTag`
- `Comment`
- `Doctype`
- `EOF`
- entity handling for the HTML 1.0 named entities already described in
  `TE04-html1.0-lexer.md`
- start and end tags with quoted and unquoted attributes
- error recovery compatible with the current HTML 1.0 lexer spec

This profile proves the generic engine without requiring the whole living
standard on day one.

## WHATWG HTML Compatibility Path

The modern tokenizer can be represented by this model because it is specified as
a named state machine that emits doctype, start tag, end tag, comment,
character, and EOF tokens. The hard part is not whether it is a state machine;
the hard part is faithfully encoding every effect attached to each transition.

To scale from HTML 1.0 to the living standard, add definitions in layers:

```text
code/tokenizers/html/
  html-common-inputs.states.toml
  html-common-actions.md
  html1.tokenizer.states.toml
  html4-compatible.tokenizer.states.toml
  whatwg-html.tokenizer.states.toml
```

The living-standard definition will need:

- RCDATA, RAWTEXT, script data, and PLAINTEXT states
- comment less-than-sign and comment bang states
- full DOCTYPE public/system identifier states
- CDATA states for foreign content
- full named and numeric character-reference states
- temporary-buffer based end-tag matching
- parse-error codes aligned with the standard
- tokenizer hooks that allow the tree constructor to change tokenizer state for
  raw text elements

That last point is important: the tokenizer and tree constructor are separate,
but modern HTML lets tree construction influence tokenizer state when entering
elements such as `script`, `style`, `title`, and `textarea`. The tokenizer
runtime must therefore expose a controlled `set_state(state)` API to the tree
constructor. That API is host code, but the state names and valid states remain
declared in the tokenizer file.

## Runtime API

Each language port should expose the same conceptual API:

```rust
let definition = TokenizerDefinition::parse_str(source)?;
definition.validate()?;

let mut tokenizer = Tokenizer::new(definition);
tokenizer.push("<p>Hello");
tokenizer.push(" &amp; bye</p>");
tokenizer.finish();

let tokens = tokenizer.drain_tokens();
let diagnostics = tokenizer.diagnostics();
```

Minimum operations:

- `parse_str(source) -> TokenizerDefinition`
- `parse_file(path) -> TokenizerDefinition`
- `validate(definition) -> ValidationReport`
- `Tokenizer::new(definition)`
- `push(chunk)`
- `finish()`
- `next_token()`
- `drain_tokens()`
- `diagnostics()`
- `trace()`
- `set_state(state)`
- `current_state()`
- `reset()`

The API may look idiomatic in each language, but behavior and naming should stay
close enough that docs and tests translate mechanically.

## Validation

The definition validator must reject:

- unknown states
- duplicate state names
- duplicate token declarations
- unknown registers
- unknown actions
- unknown guards
- unknown included files
- transitions after `anything` or `anything_else`
- unreachable states unless marked `external_entry`
- EOF states with no `eof` transition unless marked `nonterminal`
- submachine cycles that can recurse without consuming input
- action arguments of the wrong type
- registers referenced before declaration
- token fields not declared in `tokens:`

The validator should warn about:

- states with no incoming transitions
- states with no parse-error paths for malformed HTML patterns
- actions that mutate `current_token` when no token exists
- definitions that require a runtime feature newer than the package supports

## Test Strategy

### Definition Parser Tests

- parse metadata, token declarations, registers, inputs, includes, states, and
  transitions
- reject malformed TOML and unknown top-level sections
- preserve source locations for diagnostics
- produce a stable canonical AST

### Runtime Tests

- data-state text buffering
- tag creation and emission
- attribute name and value construction
- self-closing tag marker
- EOF flushing
- parse-error recording
- reconsume behavior
- lookahead across chunk boundaries
- character-reference submachine calls
- traces for representative transitions

### HTML Fixture Tests

Use small `.cases` files:

```text
case: simple-start-tag
input: <p>Hello</p>
tokens:
  StartTag(name="p", attributes=[], self_closing=false)
  Text(data="Hello")
  EndTag(name="p")
  EOF
```

Later, the WHATWG profile should import relevant tokenizer fixtures from
web-platform-tests/html and run them through the same runtime.

## Implementation Plan

1. Add this spec.
2. Add a `tokenizer-state-machine` package in Rust first, because Rust is the
   browser implementation direction and gives us strong data modeling.
3. Port the runtime to Go next, because Go is the other preferred systems
   direction for this repo.
4. Add TypeScript, Python, Ruby, and other ports only after the Rust and Go APIs
   settle.
5. Add `code/tokenizers/html/html1.tokenizer.states.toml`.
6. Compile the tokenizer definition into static source code with the F09
   compiler.
7. Rebuild `html1.0-lexer` as a wrapper over the generated definition.
8. Add conformance fixtures and transition traces.
9. Expand toward `whatwg-html.tokenizer.states.toml` state by state.
10. Create a later tree-construction spec for insertion modes, stack of open
   elements, active formatting elements, foster parenting, template insertion
   modes, and the adoption agency algorithm.

## Open Questions

- Should tokenizer runtimes parse TOML directly in every port, or should one
  reference parser generate a canonical JSON artifact that all ports consume?
- How much of the WHATWG character-reference table should live in the tokenizer
  definition versus a shared generated data package?
- Do we want a visualizer that renders tokenizer states as DOT graphs with
  action labels?

The recommended first implementation is: parse `.tokenizer.states.toml` text
into the `F07` canonical in-memory AST in each port, then add JSON import/export
once the AST stabilizes. That keeps the early system simple while preserving a
path to fast, prevalidated artifacts.

## Success Criteria

Phase 1 is successful when:

1. a `.tokenizer.states.toml` file can describe the HTML 1.0 tokenizer states
2. Rust and Go runtimes can load the same definition file
3. both runtimes pass the same HTML tokenizer fixtures
4. traces make transition behavior understandable
5. the old hand-written HTML 1.0 lexer can be replaced by a wrapper without
   changing its public token output
6. the design has a clear path to the WHATWG tokenizer without adding arbitrary
   host-language callbacks to definition files

## References

- WHATWG HTML Living Standard, "13.2 Parsing HTML documents" and "13.2.5
  Tokenization": <https://html.spec.whatwg.org/multipage/parsing.html>.
  Checked on 2026-04-19; the page reported "Last Updated 15 April 2026".
