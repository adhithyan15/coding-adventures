# Ruby parser

## Status

Spec for a from-scratch Ruby lexer + parser that covers **every Ruby
version from 1.0 (1996) through 3.3 (2023)**.  The current
`ruby-lexer` / `ruby-parser` crates in this repo are placeholders —
regex-based tokenizers that handle a Python-shaped subset and do not
implement Ruby's real semantics.  This spec replaces them.

Two companion specs hold the per-detail work:
- [ruby-lexer-state-machine.md](ruby-lexer-state-machine.md) — the
  TOML-encoded state machine, lex-state transitions, register
  vocabulary, and parser-feedback contract.
- [ruby-version-evolution.md](ruby-version-evolution.md) — the
  per-version syntax delta tables (what landed in 1.6, 1.8, 1.9,
  2.0, 2.7, 3.0, etc.).

## Why this is harder than Python or JavaScript

Ruby is **not context-free**.  Several constructs are syntactically
identical until the lexer is told what surrounding context applies:

| Source        | If `f` is a local           | If `f` is a method        |
|---------------|-----------------------------|---------------------------|
| `f /x/`       | `f / x / something`         | `f(/x/)`                  |
| `f +1`        | `f + 1`                     | `f(+1)`                   |
| `f *xs`       | `f * xs`                    | `f(*xs)`                  |
| `f %w[a]`     | `f % w[a]`                  | `f(%w[a])`                |

The lexer must know whether `f` is currently bound as a local
variable, and locals are introduced by assignment — so this knowledge
must come from the parser as parsing progresses.

Heredocs further compound the problem: `x = <<EOF + "more"` puts the
heredoc body on the *next line* but lexing continues on the current
line, then splices in the body when newline is hit.

String interpolation (`"a#{1 + foo("x")}b"`) recursively re-enters
the full lexer (and parser) inside `#{...}`.

## Approach

Adopt the same pattern that the HTML lexer/parser already use in
this repo: a **declarative state-machine lexer** (`html1.lexer.states.toml`)
that the parser drives mid-stream via `apply_html_lex_context`.  The
Ruby case extends that pattern with a *bidirectional* contract — the
lexer also queries the parser for local-variable membership.

```text
ruby source
   │
   ▼  ruby_lexer::tokenize(source, version, parser_oracle)
       ┌── parser_oracle: ────────────────────────────┐
       │   is_local(name)            → bool           │
       │   in_def?, in_block?, in_lambda? → bool      │
       │   current_lex_state          → LexState      │
       │   push_heredoc(terminator, kind)             │
       └──────────────────────────────────────────────┘
Tokens (with embedded lex-state annotations for debugging)
   │
   ▼  ruby_parser::parse(tokens, version)
GrammarASTNode  (same CST shape as python-parser / javascript-parser)
```

Both crates load **versioned** state machines + grammars, mirroring
the existing pattern from `python-parser` (versions `2.7`, `3.0`,
…) and `javascript-parser` (`es5`, `es2015`, …).

## Version coverage

This spec commits to supporting **all Ruby versions from 1.0
through 3.3** — including the very old ones.  Practical observation:
not every minor release changed syntax.  We model **15 syntax-era
versions**; intermediate point releases inherit the most recent era.

| Era version | Released  | Headline syntax additions / breaks                     |
|-------------|-----------|--------------------------------------------------------|
| `1.0`       | 1996-12   | baseline: def/end, if/end, while/end, blocks, regex, heredocs, symbols |
| `1.6`       | 2000-09   | minor lex-state refinement (`__END__`, `__FILE__`)     |
| `1.8`       | 2003-08   | block-local `|x; y|`, multiple assignment refinements   |
| `1.9.1`     | 2009-01   | hash shorthand `{a: 1}`, lambda `->()` , `__method__`, magic encoding comment, block-local `|x; y|` standardised |
| `1.9.3`     | 2011-10   | (no new syntax — fork point for the 2.x line)          |
| `2.0`       | 2013-02   | keyword arguments, `**` hash splat, `%i[]` symbol array, refinements |
| `2.1`       | 2013-12   | required kwargs `key:`, rational `1r` / complex `1i`   |
| `2.3`       | 2015-12   | safe navigation `&.`, `frozen_string_literal` pragma   |
| `2.5`       | 2017-12   | `rescue` inside `do...end` without `begin`             |
| `2.6`       | 2018-12   | endless ranges `(1..)`, `then`/`else` in `case`        |
| `2.7`       | 2019-12   | numbered block params `_1`..`_9`, beginless ranges, `case/in` pattern matching (experimental) |
| `3.0`       | 2020-12   | pattern matching stable, endless method def `def f = ...`, rightward `=>` assignment, one-line `expr in pat` |
| `3.1`       | 2021-12   | hash shorthand `{x:}` (no value), anonymous block `&`  |
| `3.2`       | 2022-12   | find pattern `[*, x, *]`, anonymous splat forwarding   |
| `3.3`       | 2023-12   | (no new surface syntax — pinning the Prism era)        |

Default version when caller passes `""` is `3.3`.  See
[ruby-version-evolution.md](ruby-version-evolution.md) for the full
delta tables and the inheritance rules between versions.

## Package layout

```
code/packages/rust/
  ruby-lexer/
    Cargo.toml
    src/
      lib.rs                  # public API: create_ruby_lexer, tokenize_ruby
      _machines.rs            # AUTO-GENERATED: compiled state machines, one per version
      parser_oracle.rs        # trait + default impl for parser-feedback queries
    grammars/
      ruby-1.0.lexer.states.toml
      ruby-1.8.lexer.states.toml
      ruby-1.9.1.lexer.states.toml
      ...
      ruby-3.3.lexer.states.toml
  ruby-parser/
    Cargo.toml
    src/
      lib.rs                  # public API: parse_ruby
      _grammar.rs             # AUTO-GENERATED: compiled parser grammars
      local_scope.rs          # ParserOracle impl — tracks locals during parse
      heredoc_queue.rs        # deferred-emit queue for heredoc bodies
    grammars/
      ruby-1.0.grammar
      ruby-1.8.grammar
      ...
      ruby-3.3.grammar
code/grammars/
  ruby-1.0.lexer.states.toml      # canonical source-of-truth copies
  ruby-1.0.grammar
  ...
```

Mirrors the per-version grammar layout already used by
[python-parser](../packages/rust/python-parser/) and
[javascript-parser](../packages/rust/javascript-parser/).  TOML state
files are compiled into Rust constants by `grammar-tools` at build
time so production runs link a static machine — no runtime TOML
parse.

## Public API

```rust
// ruby-lexer
pub fn create_ruby_lexer(source: &str, version: &str) -> Result<RubyLexer, String>;
pub fn tokenize_ruby(source: &str, version: &str) -> Result<Vec<Token>, String>;

/// The lexer's view of the parser.  Default impl (`NoLocals`)
/// treats every name as a method, suitable for paren-required Ruby
/// subsets and for round-trip tests of the lexer in isolation.
pub trait ParserOracle {
    fn is_local(&self, name: &str) -> bool { false }
    fn in_def(&self) -> bool { false }
    fn in_block(&self) -> bool { false }
    fn in_lambda(&self) -> bool { false }
}

pub fn tokenize_ruby_with_oracle(
    source: &str,
    version: &str,
    oracle: &dyn ParserOracle,
) -> Result<Vec<Token>, String>;

// ruby-parser
pub fn parse_ruby(source: &str, version: &str) -> Result<GrammarASTNode, String>;
```

Parser internally constructs its own `LocalScopeOracle` that the
lexer queries during the parse — call sites of `parse_ruby` don't
have to wire anything up.

## Tokens produced

The lexer emits a stream where each token carries:
- `kind`            — the Ruby token category (see [token list below](#token-categories))
- `value`           — original lexeme (interpolation-resolved for strings)
- `line` / `column` — 1-based source position
- `lex_state`       — the EXPR_* state active when the token was emitted (debug aid; parser may also use it)

### Token categories

Operators: `+`, `-`, `*`, `/`, `%`, `**`, `==`, `!=`, `<`, `>`, `<=`,
`>=`, `<=>`, `&&`, `||`, `!`, `&`, `|`, `^`, `~`, `<<`, `>>`, `..`,
`...`, `=>`, `&.` (2.3+), `&` (anon, 3.1+).

Punctuation: `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `:`, `::`, `.`,
`?`, `@`.

Keywords: `BEGIN`, `END`, `alias`, `and`, `begin`, `break`, `case`,
`class`, `def`, `defined?`, `do`, `else`, `elsif`, `end`, `ensure`,
`false`, `for`, `if`, `in`, `module`, `next`, `nil`, `not`, `or`,
`redo`, `rescue`, `retry`, `return`, `self`, `super`, `then`, `true`,
`undef`, `unless`, `until`, `when`, `while`, `yield`.  Newer eras
add: `__ENCODING__` (1.9), `__method__` (1.9).

Literals: `INT`, `FLOAT`, `RATIONAL` (2.1+), `IMAGINARY` (2.1+),
`STRING` (with interpolation tokens), `SYMBOL`, `REGEX`, `HEREDOC`.

Identifiers: `NAME` (local or method), `IVAR` (`@x`), `CVAR` (`@@x`),
`GVAR` (`$x`), `CONST` (capitalised), `FID` (`foo!` / `foo?`).

## Phasing

We **don't** build all 15 era machines upfront.  Phases:

**Phase 1 — paren-required Ruby 1.8 baseline**
- Single state machine, no parser oracle calls (treat every name as
  a method).  Disallows ambiguous forms: `f /x/`, `f *xs`, etc.
- Covers `def`/`end`, `if`/`end`, `while`/`end`, blocks (`do...end`
  and `{...}`), simple strings (no interpolation), regex `/.../`
  only at start-of-expression context, symbols, classes, modules.
- Goal: parse the existing `code/packages/ruby/` test corpus (most of
  which uses parens).

**Phase 2 — local-scope feedback**
- Add `ParserOracle` and the parser's local-scope tracker.
- Enable `f /x/` disambiguation, implicit-receiver method calls.
- Still single-version (1.8 baseline).

**Phase 3 — interpolation and heredocs**
- Sub-lexer stack for `"a#{expr}b"`.
- Heredoc deferred-emit queue.
- Multi-line strings (`%q`, `%Q`, `%w`, `%i`, `%r`, etc.).

**Phase 4 — version evolution**
- Fork the 1.8 machine into 1.9.1 (hash shorthand, lambda `->`).
- Add 2.0 (keyword args), 2.3 (`&.`), 2.7 (numbered params), 3.0
  (endless def, pattern matching).
- Older versions (1.0, 1.6) are forward-derived: start from 1.8 and
  *remove* features that didn't exist yet.

**Phase 5 — `ruby-to-semantic-ir` frontend**
- Wire the parser into the existing narrow-waist Semantic IR (the
  same pipeline as `python-to-semantic-ir` and
  `javascript-to-semantic-ir`).  Demonstrates Ruby → SIR → Python
  / JavaScript / TypeScript / Rust / Go.

Each phase is its own PR with full tests before merging.

### Phase 5 lowering rules added for #18778

The legacy grammar (`code/grammars/ruby/ruby.grammar`) ignores newlines, and
the lowerer used to read every bare name as a local.  Both broke ordinary
Ruby; the rules below follow Ruby's own:

| construct | rule |
|---|---|
| `[` at the start of a line, after an expression-ending token | starts a new statement.  The Rust lexer re-tags it `[ line start` (still an `LBRACKET`), and `index_suffix` and a paren-less call's first argument refuse that value, so `x = 5` ⏎ `[1].each …` is two statements, not `5[1]`. |
| `f() + f` | a parenthesised call is a whole statement only when no binary operator follows (lookaheads after `method_call` in `statement` and `modifier_statement`), so `expression_stmt` parses the sum — also inside a block or `->` body. |
| `for x in xs [do] … end` | a `for_statement`.  No new scope: `x` and every local first assigned in the body are locals of the enclosing scope (declared `nil` before the loop if new), so they keep their last values; the loop iterates a fresh `__for_<n>` and stores it into `x`.  `a...b`, and `a..b` with an integer-literal `b`, are a `ForRange`; `a..b` with any other `b` counts with `<=` in a `While`; a Float-literal start is refused; anything else is a `ForEach`.  A parameter as the loop variable is refused. |
| `rescue => e` | binds `e` with an empty class list (Ruby's default `StandardError`, a catch-all here); `e` is a local of the clause body only (in Ruby it stays visible after the clause; #18798). |
| a bare name `g` | a local if an assignment or parameter `g` appears earlier in the scope (or in a scope around a block/lambda); otherwise a call `g()` when `g` is a top-level `def` that no class or module also defines; otherwise unchanged (refused by the validator). |
| a loop body | statements only: a trailing expression is an `ExprStmt`, never the body's value (the C and Ruby backends emit only a loop body's statements). |
| `x if c` as a method's last statement | the method's value is `x` when `c` holds, `nil` otherwise. |

### Statement ends and lowering rules added for #18798

The Rust lexer's last pass, `mark_statement_ends`, says where a newline (or
a `;`) ends a statement.  Each rule below re-tags a token or inserts one; the
grammar refuses or consumes it with a plain literal, so the other languages'
lexers, which never produce these, parse as before.

| construct | rule |
|---|---|
| `;` between statements | a line break: the token becomes a `Newline` (value `;`), which the parser steps over.  The `;` that starts block-local names (`do \|x; y\|`) is kept. |
| `(`, `-`, `+` at the start of a line, after an expression-ending token | starts a new statement, like `[`: re-tagged `( line start`, `- line start`, `+ line start` (type kept).  A call's argument list (`method_call`, `dot_call`, `method_with_block`, a `def` header) refuses the `(`; `sum` refuses the signs as binary operators; a paren-less call's first argument refuses all three.  `x = 5` ⏎ `-1` leaves `x` at 5; `x = foo` ⏎ `(1)` does not pass 1 to `foo`.  An operator ending a line (`x = 5 -` ⏎ `1`) and a `.` starting one still continue the expression. |
| unary `+` | `unary_plus = "+ line start" factor`, lowered to the operand.  It takes only the tagged token, so `super + 1` stays a sum. |
| a name alone on its line, where a statement starts | followed by a synthetic `;` (`Semicolon`, flagged `TOKEN_SYNTHETIC`) when the next line could be its argument; `expression_stmt = expression [ ";" ]` consumes it.  `puts` ⏎ `puts 2` is two statements. |
| `rescue` with no `=>` on its line | a synthetic `;` after the class list (or after a bare `rescue`); `rescue_clause` accepts `exception_list ";"` and `";"`.  `rescue Boom` ⏎ `puts 40` rescues `Boom`. |
| `super`, `yield`, `return`, `break`, `next` ending a line (or before a `;`) | followed by a synthetic `;`, which the keyword's own rule takes in place of its argument list (`super_expr = "super" ( ";" \| [ super_args ] )`, and the same for the others), so `super` ⏎ `puts "hi"` and `return; 42` take nothing from what follows.  `super` gets it anywhere but a method-name position (`obj.super`, `def super`, `:super`); the four statement keywords only where a statement starts. |
| a trailing-modifier keyword (`if_modifier`, …) | never a bare value: `factor`'s KEYWORD alternative refuses it, so `puts if c` is `puts`, guarded by `c`. |
| a bare `puts` that is a whole statement (not a local, not a user `def`) | lowers like `puts()`: a `__sys_write__` with no values, which prints a newline.  As a value (`x = puts`) it stays an unknown name. |
| a `def` with method-level `rescue`/`ensure` | its value is the body's last value, or the catching clause's, through an invented local `__rescue_value_<n>` (`let` nil, stored as the last statement of the body and of each `rescue` clause, returned).  `ensure` never changes it. |
| invented names (`__for_<n>`, `__rescue_value_<n>`) | claimed from one pool holding every name the source spells (including every word inside a string, which covers `#{…}`), so no program name, even one assigned later, can be one. |

`mark_statement_ends` is linear in the number of tokens: its insertions are
spliced in by one rebuild of the token vector.

## End-to-end success criteria

A program of moderate complexity must round-trip:

```ruby
def factorial(n)
  if n == 0
    1
  else
    n * factorial(n - 1)
  end
end

puts factorial(5)
```

- Tokenizes correctly (one `INT 5` not three `tok<?>`).
- Parses without error.
- Lowers via `ruby-to-semantic-ir` (Phase 5).
- Re-emits as Python / JavaScript via existing backends, and runs.

And the heredoc-heavy ambiguity test must also work:

```ruby
greeting = <<~END
  hello
END
puts greeting.length / 2
```

- Heredoc body captured before `puts` is lexed.
- `greeting.length / 2` parses as division (because `length` is
  already known to be a method on `greeting`, not a local).

## Tests

Each lexer state machine ships with golden tests: a `.rb` input
file paired with a `.tokens` expected-output file.  Per CLAUDE.md
the per-package coverage target is ≥ 90%.

Parser fuzzing: random valid Ruby strings drawn from a small
generator, asserting parse-then-roundtrip-to-source equality.

Differential testing against MRI's `ripper` library where Ruby is
available at CI time (optional; not blocking on Windows).

## Out of scope (this spec)

- Code formatting / pretty-printing
- Semantic analysis (use-before-def, type inference)
- Refinements activation (the `using` directive is parsed but
  refinement resolution is a separate IR pass)
- `BEGIN { ... }` / `END { ... }` blocks — parsed but lowering is
  deferred
- Methods defined with non-ASCII identifiers — accepted syntactically,
  no normalisation
- `eval` / `instance_eval` strings — opaque, treated as `STRING`
