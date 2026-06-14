# Ruby lexer state machine

## Status

Detail spec for the TOML-encoded state machine that drives the
[ruby-parser](ruby-parser.md) pipeline.  Defines the EXPR_* lex
states, the register vocabulary, the parser-feedback contract, and
the per-state transitions for the ambiguous constructs.

Same runtime as [html1.lexer.states.toml](../packages/rust/html-lexer/html1.lexer.states.toml)
— the `state-machine-tokenizer/0.1` engine.  Ruby-specific machinery
is in extra register types and a small set of new action verbs
(catalogued below).

## Reading guide

- §1 enumerates the **lex states** (`EXPR_BEG`, `EXPR_END`, etc.)
  that Ruby's lexer carries.
- §2 enumerates the **registers** (text buffers, name tables,
  heredoc queue).
- §3 spells out the **parser-feedback contract** — what the lexer
  asks the parser, what the parser tells the lexer.
- §4 walks through the **ambiguity-resolution rules** for each of
  the context-sensitive constructs.
- §5 lists the **new action verbs** the state machine runtime must
  learn (extensions to the HTML vocabulary).

## §1. Lex states

Mirrors MRI's `parse.y` enum.  The lex state controls how the lexer
classifies the *next* token — most importantly, whether a `/` starts
a regex or denotes division, and whether `+` / `-` / `*` are binary
or unary.

| State           | Mnemonic                          | `/` means    | Unary `+/-` allowed | Heredoc start `<<x` allowed |
|-----------------|-----------------------------------|--------------|---------------------|------------------------------|
| `EXPR_BEG`      | beginning of expression           | regex start  | yes                 | yes                          |
| `EXPR_END`      | after a value-yielding token      | division     | no (binary)         | no (treat `<<` as shift)     |
| `EXPR_ARG`      | after a method name, may take arg | regex start  | yes (unary)         | yes                          |
| `EXPR_CMDARG`   | inside a command-style call arg   | regex start  | yes                 | yes                          |
| `EXPR_ENDARG`   | after a paren-closed arg list     | division     | no                  | no                           |
| `EXPR_MID`      | mid-expr (after binary op)        | regex start  | yes                 | no                           |
| `EXPR_FNAME`    | after `def` / `.`                 | name-part    | no                  | no                           |
| `EXPR_DOT`      | after `.` or `::`                 | name-part    | no                  | no                           |
| `EXPR_CLASS`    | after `class` keyword             | name-part    | no                  | no                           |
| `EXPR_LABEL`    | after `:` in a hash literal       | regex start  | yes                 | yes                          |
| `EXPR_LABELED`  | after a labelled arg              | regex start  | yes                 | yes                          |
| `EXPR_ENDFN`    | after a method-def signature      | division     | no                  | no                           |
| `EXPR_VALUE`    | after `return` / `yield` etc.     | regex start  | yes                 | yes                          |

State transitions happen on token emit; the table is encoded in the
TOML as a per-token-class `next_state` field.  Several transitions
also depend on parser state (see §3) — those use the action verb
`set_state_via(parser_oracle_query)`.

## §2. Registers

Beyond the HTML lexer's `text_buffer`-style registers, Ruby needs:

| Register             | Type                          | Use                                                  |
|----------------------|-------------------------------|------------------------------------------------------|
| `token_buffer`       | string                        | accumulating identifier / number / operator chars     |
| `string_buffer`      | string                        | accumulating string-literal contents (escapes resolved) |
| `string_delim`       | char                          | the opening delimiter of the current string literal   |
| `interp_depth`       | uint                          | nesting depth of `#{...}` inside strings              |
| `paren_stack`        | vec<char>                     | for `%w[...]` / `%q{...}` matched-delimiter tracking  |
| `heredoc_queue`      | vec<HeredocSpec>              | terminators of heredocs whose bodies are pending      |
| `heredoc_indent`     | int                           | for `<<~`, the smallest indentation seen              |
| `current_lex_state`  | enum LexState (§1)            | the active expression-position state                  |
| `cmdarg_stack`       | uint bitfield                 | for nested command-arg contexts                       |
| `cond_stack`         | uint bitfield                 | for `cond` contexts (in `while`/`until` headers)      |
| `paren_balance`      | int                           | gross `(`/`)` balance for error recovery              |
| `version`            | const string                  | which ruby version's rules are active                  |

`HeredocSpec` is `{ terminator: string, kind: enum (Plain, Indent, Squiggly), interpolating: bool, expanding: bool, body_buffer: string }`.

## §3. Parser-feedback contract

The lexer holds a reference to a `ParserOracle` and may call any of:

```
oracle.is_local(name)           -> bool       # disambiguates `f /x/`
oracle.in_def()                 -> bool       # disambiguates some keyword behaviours
oracle.in_block()               -> bool       # `next` / `break` / `return` differ
oracle.in_lambda()              -> bool       # `return` is hard-return in lambdas
oracle.in_class_body()          -> bool       # `def self.x` vs `def x` lex differently
```

The parser drives lexer state changes via:

```
lexer.set_lex_state(state)      # parser commits to EXPR_BEG, EXPR_FNAME, etc.
lexer.push_heredoc(spec)        # parser saw `<<X`; lexer queues a body capture
lexer.enter_string(delim, ...)  # parser entered a string-literal context
lexer.exit_string()
```

The HTML lexer already exposes a one-way version of this
(`apply_html_lex_context`).  Ruby's contract is bidirectional: the
parser can both push state changes **and** answer queries during the
parse.

Implementation note: to avoid lifetime tangles, the oracle is a
`&dyn ParserOracle` passed at construction.  The parser is the
oracle (it implements the trait); during `parse_ruby` it holds a
mutable handle to the lexer and a reference to itself via `Rc`
indirection.  See [ruby-parser.md](ruby-parser.md) §"Public API".

## §4. Ambiguity-resolution rules

### `f /x/` — method call with regex vs division

| `lex_state` at `/` | `is_local(f)` | Resolution             |
|--------------------|---------------|------------------------|
| `EXPR_END`         | (any)         | division              |
| `EXPR_ENDARG`      | (any)         | division              |
| `EXPR_ARG`         | true          | division              |
| `EXPR_ARG`         | false         | regex (method call)   |
| `EXPR_BEG`         | (any)         | regex                 |
| `EXPR_MID`         | (any)         | regex                 |

In words: `/` is **regex** when we're at an expression-start
position and the preceding identifier could be a method.

### `a + b` vs `a +b`

After a name token in `EXPR_ARG`, when the next char is `+` or `-`:
- If followed immediately by a digit / identifier with **no space
  before**, treat as unary applied to a method-call argument
  (`a(+b)`).
- If followed by space-then-operand or by operand-then-space, treat
  as binary (`a + b`).

This mirrors MRI's exact heuristic: significant whitespace.

### `do...end` vs `{...}` block-binding precedence

Both are blocks.  Difference:
- `{...}` binds **tightly** to the immediately preceding method call
- `do...end` binds **loosely**, attaching to the leftmost outer call

```ruby
foo bar do end       # parsed as `foo(bar) { ... }` — do-block binds to foo
foo bar { }          # parsed as `foo(bar { })` — brace-block binds to bar
```

The lexer just emits `do` or `{` tokens; the parser disambiguates.
The parser uses a precedence shift rule keyed on the token kind.

### Heredocs

`<<X` (or `<<-X`, `<<~X`, `<<"X"`, `<<'X'`) declares a heredoc whose
body extends to a line equal to `X` (with optional indentation
stripping for `<<-` / `<<~`).

Body capture is **deferred**: the lexer keeps lexing the current
line as normal tokens, pushing the heredoc spec into
`heredoc_queue`.  At the first newline, the lexer enters
`heredoc_body` state, scans until the terminator line, then resumes
the queue's next entry (if any) before returning to the normal
state.

Multiple heredocs on one line are legal and stack:

```ruby
x = <<A + <<B
a body
A
b body
B
```

The queue preserves FIFO order so `A`'s body comes before `B`'s.

### String interpolation `#{...}`

Inside a `"..."` or backtick or interpolating heredoc, the literal
`#{` begins an embedded expression.

- Lexer pushes the current string-literal context onto
  `paren_stack` and re-enters `EXPR_BEG`.
- A matching `}` (tracked by `interp_depth`) pops back into the
  string-literal context.
- Nested interpolation works recursively; `interp_depth` and
  `paren_stack` handle it.

The state machine encodes this as a *sub-machine entry* — same
mechanism HTML uses for CDATA inside `<script>`.

### Symbols vs ternary `?:`

`:foo` is a symbol literal; `?` followed by anything else is the
ternary.  The lex state at `:` decides:
- In `EXPR_BEG` / `EXPR_MID` / `EXPR_VALUE` / `EXPR_ARG` (with
  name-shaped follower): symbol.
- In `EXPR_END` / `EXPR_ENDARG`: ternary operator.

### `%w[...]`, `%q{...}`, `%r{...}` percent literals

After `%`, the lexer reads:
1. A type char (`w` / `i` / `q` / `Q` / `r` / `s` / `x`) — or none, meaning quoted string.
2. An opening delimiter char (`(` / `[` / `{` / `<` / any non-alphanumeric).
3. Body up to the matching closing delimiter, with nesting tracked
   in `paren_stack`.

These exist from Ruby 1.0; the type chars `i` (symbol array) and `I`
(interpolating symbol array) were added in 2.0.  Version gating in
the TOML uses an `available_in_version` predicate.

### Numbered block parameters (`_1` .. `_9`)

From Ruby 2.7.  A `NAME` token whose lexeme matches `_[1-9]` is
re-classified as `NUMBERED_PARAM` only inside block bodies when the
block had no explicit parameter list.  The lexer can't know this
alone — the parser sets a `numbered_params_active` flag on the
oracle when it enters such a block.

### Endless method def (`def foo() = body`)

From Ruby 3.0.  Pure parser-level disambiguation (the lexer doesn't
need to know): after `def NAME ( params )`, the parser looks for `=`
in the *grammar* and follows the expression-body production.  No
new lexer state.

## §5. New action verbs

Beyond the HTML action vocabulary (`append_text`, `emit_token`, etc.):

| Action                       | Effect                                                             |
|------------------------------|--------------------------------------------------------------------|
| `query_is_local(buffer)`     | sets transition condition based on `oracle.is_local(buffer)`        |
| `push_heredoc(kind)`         | reads terminator from `string_buffer`, pushes onto `heredoc_queue`  |
| `enter_heredoc_body`         | pops the head of `heredoc_queue`, switches to heredoc-body capture  |
| `set_lex_state(state)`       | writes `current_lex_state` register                                  |
| `push_paren(char)`           | appends to `paren_stack`                                            |
| `pop_paren_or_fail`          | pops `paren_stack`, fails if mismatched                             |
| `inc_interp_depth`           | for tracking `#{}` nesting                                          |
| `dec_interp_depth`           | for `#{}` close                                                     |
| `emit_with_state(kind,next)` | atomic emit + state-transition in one step                          |
| `version_gate(ver)`          | guard transition behind `version >= ver`                            |

The runtime extension is small (~10 verbs) and orthogonal to the
HTML verbs — the same engine runs both.

## TOML schema example (sketch)

```toml
format = "state-machine/v1"
profile = "lexer/v1"
name = "ruby-1.8-lexer"
kind = "transducer"
version = "0.1.0"
runtime_min = "state-machine-tokenizer/0.2"   # new minor: adds Ruby verbs
initial = "EXPR_BEG"
includes = []

[[tokens]]
name = "NAME"
fields = ["value", "lex_state"]

[[tokens]]
name = "REGEX"
fields = ["pattern", "flags"]

# ... ~50 token kinds total ...

[[registers]]
id = "current_lex_state"
type = "enum"
values = ["EXPR_BEG", "EXPR_END", "EXPR_ARG", ...]

[[registers]]
id = "heredoc_queue"
type = "vec"
element_type = "heredoc_spec"

[[states]]
id = "EXPR_BEG"

[[states.transitions]]
on_char = "/"
goto = "regex_body"
actions = ["set_state(EXPR_BEG)"]

[[states]]
id = "EXPR_ARG"

# `f /x/` disambiguation
[[states.transitions]]
on_char = "/"
guard = "query_is_local(token_buffer)"
goto = "divison_after_name"   # treat as binary divide

[[states.transitions]]
on_char = "/"
goto = "regex_body"            # otherwise regex
```

## Versioning the TOMLs

Each era gets its own `ruby-<ver>.lexer.states.toml`.  Most rules
are stable across versions; we use TOML inheritance:

```toml
extends = "ruby-1.8"
overrides = ["states.EXPR_BEG.transitions[3]", ...]
```

Inheritance is resolved at compile time by `grammar-tools`, not at
runtime.  Resulting per-version machines are byte-identical to a
hand-written version-specific TOML — inheritance is just authoring
convenience.

## Cross-cutting concerns

- **Source position tracking**: every token carries `line` /
  `column` (1-based).  The runtime increments on `\n` and resets
  column.  Same shape as HTML lexer output.
- **Error recovery**: lexer never panics.  Malformed input emits a
  `LexerError` token (with span and diagnostic message) and
  continues from the next character.  The parser is responsible
  for deciding whether to abort or skip ahead.
- **Determinism**: the same input + version + oracle always
  produces the same token stream.  Property tests verify this.
- **Performance budget**: ~10 µs per KB of typical Ruby source on
  a 2024 laptop.  This isn't a real number yet — to be tightened
  after Phase 1 benchmarks.
