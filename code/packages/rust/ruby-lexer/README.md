# coding-adventures-ruby-lexer

Ruby lexer driven by a **TOML-encoded state machine**.  Phase 1 of
the multi-phase plan in
[code/specs/ruby-parser.md](../../../specs/ruby-parser.md).

## How it works

The state machine lives in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml)
— a hand-authored TOML file using the
`state-machine-markup-deserializer/v1` schema.  It declares the
states (`data`, `ident_body`, `int_body`, `string_d_body`,
`comment_body`, the multi-character-operator peek states, etc.),
the transitions between them, and the **portable action verbs**
the runtime fires (`emit(<TokenName>)`, `append_text(current)`,
`clear_text`, `parse_error(<code>)`).

This crate:

1. Loads the TOML at build via `include_str!`.
2. Parses it through `state_machine_markup_deserializer::from_states_toml`.
3. Builds an `EffectfulStateMachine` from the typed definition.
4. Drives it character-by-character; on each step the engine
   returns a list of effect strings.  The action interpreter in
   this crate turns those into `lexer::token::Token` values.

```
ruby-1.8.lexer.states.toml   (source of truth, hand-authored)
     │
     ▼  state_machine_markup_deserializer::from_states_toml
StateMachineDefinition       (typed AST of the TOML)
     │
     ▼  state_machine::EffectfulStateMachine::from_definition
EffectfulStateMachine        (engine, runs states + transitions)
     │
     ▼  step()  → emits effect strings on every character
action interpreter (ruby-lexer/src/lib.rs)
     │
     ▼  Vec<Token>
ruby-parser consumes the tokens
```

## Phase 1 scope (what this crate covers today)

- Identifiers (keywords distinguished by the action interpreter)
- Integers (decimal only — `0x` / `0b` / `0o` arrive in a later phase)
- Strings (`"..."` and `'...'`) — no interpolation yet
- Line comments (`# ...`)
- Operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`,
  `>=`, `=`, `!`, `&&`, `||`, `=>`, `**`
- Punctuation: `( ) [ ] { } , ; : :: .`
- Newlines emitted as `Newline` tokens (Ruby treats them as
  statement terminators)
- Method-name `?` / `!` suffixes (`empty?`, `save!`)

## Out of Phase 1 (later phases per
[ruby-parser.md](../../../specs/ruby-parser.md))

- Heredocs, `%w[]` / `%q{}` / `%r{}` percent literals
- String interpolation `"a#{expr}b"`
- Regex `/.../` (needs parser-feedback to disambiguate `f /x/`)
- Parser-driven local-variable lexing (Phase 2)
- Hash shorthand, lambdas, keyword args (later eras: 1.9.1+)

## Usage

```rust
use coding_adventures_ruby_lexer::{tokenize_ruby, RubyLexer};

// One-shot convenience.
let tokens = tokenize_ruby("def factorial(n)\n  n * factorial(n - 1)\nend\n");

// Or drive the lexer directly:
let mut lexer = RubyLexer::new("1.8").unwrap();
lexer.push("foo + 1").unwrap();
lexer.finish().unwrap();
let tokens = lexer.drain_tokens();
let diagnostics = lexer.diagnostics();  // non-fatal lex errors
```

## Adding a new Ruby version

1. Author a `ruby-<ver>.lexer.states.toml` at the crate root.
2. Wire it into `src/machine.rs::definition_for_version`.
3. Note the syntax additions in
   [`code/specs/ruby-version-evolution.md`](../../../specs/ruby-version-evolution.md).

## Tests

`cargo test -p coding-adventures-ruby-lexer` runs the unit suite,
including a factorial-program tokenization smoke test and several
operator-precision tests.
