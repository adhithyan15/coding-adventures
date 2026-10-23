# F11: Token-Adjacency Predicates in `.grammar`

**Status:** Draft, 2026-10-20. Implements the *predicate* half of
[LANG84 extension E6](LANG84-grammar-driven-versioned-frontends.md#e6-token-adjacency-predicates-and-asi-in-grammar).
`@asi(...)` (automatic semicolon insertion) is the other half of E6 and is
**not** part of F11; it lands in a follow-up that reuses the flags and the
element defined here.

## Overview

Some syntax depends on the *gaps* between tokens rather than on the tokens
themselves:

| Language | Source | Meaning | Fact needed |
| --- | --- | --- | --- |
| Ruby | `f -1` | `f(-1)`, a command call | space before `-`, none after |
| Ruby | `f - 1`, `f-1` | `f - 1`, a subtraction | space after `-`, or none before |
| Ruby | `foo [1]` vs `foo[1]` | argument vs index | space before `[` |
| ECMAScript | `return` ⏎ `x` | `return; x;` | line terminator before `x` |
| ECMAScript | `a` ⏎ `++b` | `a; ++b;` | line terminator before `++` |

Until now a `.grammar` file could only see a line break if newlines were
emitted as `NEWLINE` tokens, and referencing `NEWLINE` anywhere makes the whole
grammar newline-significant (`grammar_references_newline`). Whitespace was not
visible at all. F11 makes both facts visible as **zero-width predicates** that
read per-token flags recorded by the lexer.

### Relationship to existing specs

- **LANG84 E6** proposed the syntax (`@no_newline_before`, `@space_before`,
  `!@space_after_next`). F11 implements it unchanged and adds the obvious
  `no_` / plain complements, so every fact has both polarities.
- **02-lexer** gains the `TOKEN_PRECEDED_BY_SPACE` flag and a precise
  definition of when `TOKEN_PRECEDED_BY_NEWLINE` is set.
- **03-parser** gains one element kind, `AdjacencyPredicate`.
- **F04 / F10** (pattern groups, declarative modes) are unaffected: the flags
  describe the trivia between tokens, which is independent of the active mode.

## Lexer: what is recorded

`Token.flags` (an `Option<u32>` bitmask that already carried
`TOKEN_PRECEDED_BY_NEWLINE = 1` and `TOKEN_CONTEXT_KEYWORD = 2`) gains:

```rust
pub const TOKEN_PRECEDED_BY_SPACE: u32 = 4;
```

No token's type, value, line or column changes. Only `flags` gains bits.

For each token matched from a `.tokens` pattern, the `GrammarLexer` sets:

- `TOKEN_PRECEDED_BY_SPACE` when *anything* was consumed between the previous
  emitted token and this one without becoming a token: a skip-pattern match
  (whitespace or a comment), the built-in whitespace skip (` `, `\t`, `\r`)
  used by grammars without `skip:` patterns, a newline swallowed inside
  brackets in indentation mode, a token suppressed by an F04 callback with
  nothing emitted in its place, or a built-in `NEWLINE` token.
- `TOKEN_PRECEDED_BY_NEWLINE` when that gap contains a line terminator: a skip
  match containing `\n`, `\r`, U+2028 or U+2029 (so a line break inside a
  multi-line `/* … */` comment counts, as ECMAScript requires), a built-in
  `NEWLINE` token, or a suppressed (unreplaced) token whose text contains a
  line terminator.

When an F04 callback suppresses a token **and emits replacements** (as
excel-lexer turns `NAME` into `FUNCTION_NAME` before `(`), the first emitted
token stands where the suppressed one stood: the suppressed token's gap bits
are OR'd into that token's own `flags`, and the suppressed text is *not*
trivia for what follows. So `SUM(1)` gives `FUNCTION_NAME` no flags and `(`
no flags, and `x + SUM(1)` gives `FUNCTION_NAME` the space before it. Only a
suppression with nothing emitted turns the suppressed text into trivia.

The built-in `NEWLINE` token counts as trivia **for the token after it**: the
next token is flagged `SPACE | NEWLINE`. This gives the same answer whether or
not the parser keeps `NEWLINE` tokens (it drops them when the grammar never
references `NEWLINE`). The `NEWLINE` token's own `flags` stay `None`, as before.

Positions are measured between *emitted* tokens. Two tokens with nothing
between them (`f-1`, `a.b`) have neither bit. A line break *inside* a token (a
multi-line string or template) is part of that token and sets nothing.

What does not get flags:

| Token | Flags |
| --- | --- |
| Built-in `NEWLINE`, `INDENT`, `DEDENT`, `EOF` | `None` (unchanged) |
| Layout-mode `VIRTUAL_*` tokens | `None` (unchanged) |
| Tokens emitted by an F04 callback (`ctx.emit`) | whatever the callback sets |
| The second half of a parser-split `>>` (see below) | `None` (it is adjacent) |

F10 modes, F04 groups and `disable-skip`/`enable-skip` need no special
handling: the lexer records what it *did* consume as trivia, whichever group
was active. With skipping disabled nothing is trivia, so nothing is flagged.

Indentation mode (Python/Starlark) records the same two bits with the same
rules: spaces and tabs inside a line, comments, bracketed newlines and
`NEWLINE` tokens. Leading indentation follows a `NEWLINE` token, so the first
token of a line is always `SPACE | NEWLINE`.

## `.grammar` syntax

A predicate is `@` followed by its name. It is an **element**: it can appear
anywhere an element can (in a sequence, alternation, `[ ]`, `{ }`, `( )`, and
under `&` / `!`). It consumes nothing and produces no AST children.

| Predicate | Succeeds when |
| --- | --- |
| `@space_before` | the next token is preceded by trivia |
| `@no_space_before` | the next token is directly adjacent to the previous token |
| `@newline_before` | a line terminator precedes the next token |
| `@no_newline_before` | no line terminator precedes the next token (ECMAScript `[no LineTerminator here]`) |
| `@space_after_next` | the token *after* the next token is preceded by trivia |
| `@no_space_after_next` | the token after the next token is directly adjacent to the next token |

"The next token" is the token the following element would try to match. So a
predicate is written *before* the token it tests, and a predicate written
after a token tests the gap that follows it:

```text
# ECMAScript restricted productions (LANG84 E6's examples, verbatim)
return_statement = "return" [ @no_newline_before expression ] SEMICOLON ;
postfix_expr     = lhs_expr [ @no_newline_before ( "++" | "--" ) ] ;

# Ruby `f -1`: space before the minus, none after it. Three equivalent forms:
command_arg = @space_before !@space_after_next MINUS primary ;
command_arg = @space_before @no_space_after_next MINUS primary ;
command_arg = @space_before MINUS @no_space_before primary ;
```

`!@p` is ordinary negative lookahead over a zero-width element, so it is
always equivalent to the `no_` form (`!@space_after_next` ≡
`@no_space_after_next`). Both spellings are accepted; the `no_` form is
preferred because it reads as a fact.

Predicate names are case-sensitive lowercase. Unknown names are a parse
error that lists the six valid names, and a bare `@` is an error too. The
reserved E6/E7 forms fail loudly rather than parsing into something else:

- `@asi(...)` gets its own error: not implemented yet (LANG84 E6 follow-up).
  It is raised while tokenizing, before the comma-separated argument list
  could produce a confusing "unexpected character ','".
- `NAME@declare`, `@scope(...)` and the other E7 binding forms are rejected as
  unknown predicates.

## Validation

Termination does not depend on validation: `GrammarParser::new` never
validates, so the engine stops any repetition after an empty iteration (see
[Parser semantics](#parser-semantics)). Validation exists to tell the author.

`validate_parser_grammar` adds a **nullability analysis**. A rule is nullable
when its body can succeed without consuming a token, computed as a least
fixpoint over the rules:

| Element | Nullable when |
| --- | --- |
| token, literal | never |
| `@p`, `&x`, `!x`, `[ x ]`, `{ x }`, `{ x // s }` | always |
| `( x )`, `{ x }+` | `x` is |
| `{ x // s }+` | `x` is |
| `a b c` | every element is |
| `a \| b \| c` | any choice is |
| rule reference | the rule is |

A repetition (`{ }`, `{ }+`, or `{ x // s }` when both `x` and `s` are
nullable) whose body is nullable is reported:

- as an **error** when the body contains an adjacency predicate, directly or
  through the rules it references (`{ @space_before | NAME }`,
  `{ @space_before [ NAME ] }`, `{ p }` with `p = @space_before`). That is new
  F11 syntax, so no existing grammar is affected;
- as a **`Warning:`** otherwise (`{ [ NAME ] }`). Twelve existing grammars,
  all `csharp*.grammar` (`switch_block = LBRACE { switch_section } RBRACE`
  with `switch_section = { switch_label } { statement }`), have such a body.
  Making it an error would make them invalid; the engine guard already makes
  them terminate.

Predicates in any other position
are valid, including as the last element of a rule (a trailing-context test)
and as the only element of an alternative.

Predicates contain no token or rule references, so they do not affect the
undefined-reference and unreachable-rule checks or the tokens/grammar
cross-validator.

## Compiled grammars

`GrammarElement` gains one variant:

```rust
AdjacencyPredicate { predicate: AdjacencyPredicate }

pub enum AdjacencyPredicate {
    SpaceBefore, NoSpaceBefore,
    NewlineBefore, NoNewlineBefore,
    SpaceAfterNext, NoSpaceAfterNext,
}
```

`compile_parser_grammar` emits it with a fully qualified path,

```rust
GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NoNewlineBefore }
```

so the generated `use` line is unchanged and grammars that do not use
predicates regenerate byte-identically.

## Parser semantics

`GrammarParser` evaluates a predicate at the current cursor without moving
it:

1. Find the next token the way token matching does. If the grammar is not
   newline-significant, skip `NEWLINE` tokens; each skipped `NEWLINE` counts
   as `SPACE | NEWLINE`. Past the end, the last token (`EOF`) is used.
2. The predicate's bits are those skipped-`NEWLINE` bits OR'd with the found
   token's `flags`. For the `_after_next` forms, repeat step 1 from the token
   after the one found, and use that token's bits.
3. Succeed or fail. On failure nothing is recorded in the furthest-failure
   set, as with `&`/`!` lookahead. The element that would have followed
   records its own expectation, so error messages and error token positions
   are unchanged.

Consequences:

- **Zero-width everywhere.** Inside `[ ]`, an alternative, a repetition body
  or `&`/`!`, a predicate behaves like a lookahead that consumed nothing.
- **Repetitions always terminate.** `{ }`, `{ }+` and `{ x // s }` stop after
  an iteration that succeeded but left the cursor where it was and performed
  no `>>` split (a split matches a `>` without moving the cursor, which is
  progress). That iteration is undone, so it adds no children. Matching is a
  pure function of the position, so before this guard such an iteration
  repeated forever, and through memo hits on a rule body grew the tree
  without bound. A parse that terminated before the guard never reaches it,
  so its result is unchanged; the guard only turns hangs into results (for
  example the C# `switch_block` above).
- **Packrat-safe.** A predicate reads only the token stream at the cursor (and
  at most one significant token further for `_after_next`). The memo key
  `(rule, position)` therefore still determines the result. The only mutation
  of the token stream is the existing `>>` split, whose invalidation is
  unchanged. Its remainder token is given `flags: None`, which is correct: it
  is directly adjacent to the `>` split off before it.
- **Newline significance is unchanged.** `@newline_before` does not reference
  `NEWLINE`, so using it does not make a grammar newline-significant. That is
  the point: a grammar can skip newlines as whitespace and still see them.
- **Hand-built token streams.** The predicates read `flags`. A token vector
  that was not produced by `GrammarLexer` (for example a hand-written test
  stream or another lexer) must set the bits itself. Absent flags read as
  "adjacent, same line".

## Backward compatibility

- A grammar with no `@` predicate parses into the same `ParserGrammar`,
  compiles to byte-identical `_grammar.rs`, and parses every input to the same
  tree with the same errors.
- `TOKEN_PRECEDED_BY_SPACE` appears on most tokens, so `flags` is now usually
  `Some(..)`. Code that tests a bit with `flags.unwrap_or(0) & BIT` is
  unaffected. Code that compares whole `Token`s against `flags: None` fixtures
  must account for the new bit.
- `TOKEN_PRECEDED_BY_NEWLINE` is additionally set on the token after a built-in
  `NEWLINE` token and in indentation mode. Before F11 the only reader was the
  JavaScript ASI pass, whose grammar skips newlines as whitespace and so never
  sees a `NEWLINE` token. Its behaviour is unchanged.

## Other grammar-tools ports

LANG84 requires the non-Rust ports to *reject* a grammar that uses an
extension they do not implement. All fourteen ports (Python, Ruby,
TypeScript, Go, Elixir, Lua, Perl, Java, Kotlin, C#, F#, Dart, Swift,
Haskell) already do: their `.grammar` tokenizers raise
`Unexpected character '@'`. No shared `.grammar` file uses predicates yet.
Follow-ups, in order of value:

1. Make that rejection name the extension ("token-adjacency predicates
   (LANG84 E6 / F11) are not supported by this port").
2. Port the element, its validation and the lexer flags, where a port's
   parser is used to parse a grammar that adopts predicates.

## Testing

- **grammar-tools:** each of the six predicates parses, also under `!`, `&`,
  `[ ]`, `{ }` with a consuming element, and after a token; unknown names and
  `@asi(...)` are rejected with clear messages; a bare `@` is rejected; the
  nullability fixpoint; nullable repetitions are errors with a predicate
  (including the security-review probes and nullability through rule
  references), warnings without one, and silent when the body consumes;
  predicates add no token or rule references; `compile_parser_grammar` emits
  the qualified path and the emitted text round-trips.
- **lexer:** space and newline bits from skip patterns, from multi-line block
  comments, from the built-in whitespace skip, after a built-in `NEWLINE`,
  across an F10 mode switch, in indentation mode, for suppressed tokens with
  and without an emitted replacement (excel-lexer style); none
  for adjacent tokens or across a newline inside a string.
- **parser:** Ruby-style `f -1` (command argument) versus `f - 1` and `f-1`
  (binary), in all three equivalent spellings; ECMAScript-style
  `return` [no LineTerminator here] with newlines skipped as whitespace and
  with newlines as (insignificant) `NEWLINE` tokens; postfix `++`; predicates
  under `&`, `!`, repetition and alternation; memoized rules reused at
  different gaps; error message and error token unchanged by a failing
  predicate; every nullable-repetition probe (`{ }`, `{ }+`, `{ x // s }`,
  through a rule) terminates with the right tree or error, and a `>>` split
  still counts as progress.
- **Regression:** existing grammar consumers' test suites pass unchanged, and
  regenerating every Rust compiled grammar gives the same output before and
  after F11.
