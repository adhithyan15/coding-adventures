# Grammar Tools

## Overview

`grammar-tools` provides parsers and validators for two declarative file formats used
throughout this repo to describe programming language syntax:

- **`.tokens` files** — define the lexical grammar (every token the lexer recognises)
- **`.grammar` files** — define the syntactic grammar in EBNF (how tokens combine into
  valid programs)

Together, these two files give a complete, language-agnostic description of a language's
surface syntax. All six language ports (Python, Go, Ruby, TypeScript, Rust, Elixir) share
the same `.tokens` and `.grammar` files, with thin wrapper packages in each language that
delegate to `GrammarLexer` and `GrammarParser`.

### Package role in the stack

```
lattice.tokens  ──parse──►  TokenGrammar  ──►  GrammarLexer
lattice.grammar ──parse──►  ParserGrammar ──►  GrammarParser
                                │
                         validate / cross_validate
                                │
                    errors reported before test time
```

---

## File Formats

### `.tokens` files

Each non-blank, non-comment (`#`) line has one of the forms below.

```
# Regex-based token
TOKEN_NAME = /regex_pattern/

# Literal-based token
TOKEN_NAME = "literal_string"

# Alias — lexer emits token type ALIAS instead of TOKEN_NAME
TOKEN_NAME = /regex/ -> ALIAS
TOKEN_NAME = "literal" -> ALIAS

# Lexer mode configuration
mode: indentation

# Escape processing mode
escape_mode: none

# Section headers
keywords:        — begin keywords list (one keyword per indented line)
reserved:        — begin reserved keywords list
skip:            — begin skip patterns (consumed without producing tokens)
errors:          — begin error-recovery patterns (stored, not yet used by GrammarLexer)
group NAME:      — begin a named pattern group for context-sensitive lexing
```

Lines starting with `#` are comments. Blank lines are ignored. Indented lines inside a
section belong to that section.

#### Token name convention

Token names MUST be `UPPER_CASE` (letters, digits, underscores; first char a letter).
Rule names in `.grammar` files MUST be `lower_case`. This case distinction is how the
parser infrastructure tells tokens and rules apart.

#### The `errors:` section

Error-recovery patterns use the same definition format as the `skip:` section. They are
stored on `TokenGrammar.error_definitions` and validated for regex correctness, but the
`GrammarLexer` does not apply them during normal tokenisation. They are reserved for
future graceful error-recovery work and for compatibility with grammar files authored with
the Python grammar-tools which supports them.

#### The `groups:` section

A `group NAME:` section defines a set of token patterns active during context-sensitive
lexing. The `GrammarLexer` maintains a group stack; only patterns from the top group (plus
global `skip:` patterns) are tried.

---

### `.grammar` files

Each non-blank, non-comment line defines one EBNF rule:

```
rule_name = body
```

**Body operators** (precedence low to high):

| Syntax | Meaning |
|--------|---------|
| `a \| b` | Alternation — match `a` or `b` |
| `a b` | Sequence — match `a` then `b` |
| `(a b)` | Group — explicit grouping |
| `[a]` | Optional — match `a` zero or one times |
| `{a}` | Repetition — match `a` zero or more times |
| `"literal"` | Literal token value |
| `UPPER_CASE` | Token reference (defined in `.tokens` file) |
| `lower_case` | Rule reference (defined elsewhere in this file) |

The **first rule** in the file is the start symbol.

---

## Public API

All six language implementations must expose the same surface. Function names follow each
language's naming convention (e.g., `parse_token_grammar` in Python/Ruby/Elixir,
`ParseTokenGrammar` in Go, `parseTokenGrammar` in TypeScript, `parse_token_grammar` in
Rust).

### Parsing

```python
# Parse a .tokens file from source text
parse_token_grammar(source: str) -> TokenGrammar
# raises TokenGrammarError on parse failure

# Parse a .grammar file from source text
parse_parser_grammar(source: str) -> ParserGrammar
# raises ParserGrammarError on parse failure
```

### Validation

```python
# Validate a parsed TokenGrammar
# Returns a list of warning/error strings; empty list = no issues
validate_token_grammar(grammar: TokenGrammar) -> list[str]

# Validate a parsed ParserGrammar
# token_names: optional set of valid token names from .tokens (enables cross-checking)
validate_parser_grammar(grammar: ParserGrammar, token_names=None) -> list[str]

# Cross-validate a token grammar and a parser grammar together
cross_validate(token_grammar: TokenGrammar, parser_grammar: ParserGrammar) -> list[str]
```

### Data structures

```python
@dataclass
class TokenDefinition:
    name: str          # UPPER_CASE token name
    pattern: str       # regex string or literal string
    is_regex: bool     # True → regex, False → literal
    alias: str | None  # If present, lexer emits ALIAS instead of name
    line_number: int

@dataclass
class TokenGrammar:
    definitions: list[TokenDefinition]         # Regular token definitions
    keywords: list[str]                        # keywords: section
    skip_definitions: list[TokenDefinition]    # skip: section
    error_definitions: list[TokenDefinition]   # errors: section
    reserved_keywords: list[str]               # reserved: section
    mode: str | None                           # e.g. "indentation"
    escape_mode: str | None                    # e.g. "none"
    groups: dict[str, PatternGroup]            # group NAME: sections

@dataclass
class ParserGrammar:
    rules: list[GrammarRule]

@dataclass
class GrammarRule:
    name: str           # lower_case rule name
    body: GrammarElement
    line_number: int

# GrammarElement variants (discriminated union / sealed interface):
# Sequence(elements), Alternation(elements), Repetition(element),
# Optional(element), Group(element), RuleReference(name), Literal(value),
# TokenReference(name)  ← note: some implementations use RuleReference
#                                 for both rule refs and token refs
```

---

## Validation Rules

### `validate_token_grammar`

| Check | Severity | Description |
|-------|----------|-------------|
| Duplicate token name | Error | Two definitions share the same `name` |
| Empty pattern | Error | `pattern` is the empty string |
| Invalid regex | Error | `is_regex=True` but `re.compile(pattern)` fails |
| Non-UPPER_CASE name | Error | Token name must be `UPPER_CASE` |
| Non-UPPER_CASE alias | Error | Alias must be `UPPER_CASE` |
| Unknown lexer mode | Error | Only `"indentation"` is supported |
| Unknown escape mode | Error | Only `"none"` is supported |
| Invalid group name | Error | Group names must be `lowercase_identifiers` |
| Empty group | Warning | A group with no definitions |

Same checks apply to `skip_definitions` and `error_definitions`.

### `validate_parser_grammar`

| Check | Severity | Description |
|-------|----------|-------------|
| Duplicate rule name | Error | Two rules share the same `name` |
| Non-lowercase rule name | Error | Rule names must be `lower_case` |
| Undefined rule reference | Error | `lower_case` name used but never defined |
| Undefined token reference | Error | `UPPER_CASE` name not in provided `token_names` set |
| Unreachable rule | Warning | Rule defined but never referenced (start rule exempt) |

### `cross_validate`

| Check | Severity | Description |
|-------|----------|-------------|
| Missing token definition | Error | Grammar references `TOKEN` but it is not in `.tokens` |
| Unused token | Warning | Token defined in `.tokens` but not referenced in grammar |

**Synthetic tokens** — always valid to reference in a grammar without a `.tokens` definition:
`NEWLINE`, `INDENT`, `DEDENT`, `EOF`

---

## CLI Validate Tool

Every language must provide a CLI entry point that implements this interface exactly:

```
grammar-tools validate <file.tokens> <file.grammar>
grammar-tools validate-tokens <file.tokens>
grammar-tools validate-grammar <file.grammar>
grammar-tools --help
```

**Exit codes:** `0` = all checks passed, `1` = at least one error found, `2` = usage error.

**Output format** (must match across all languages):

```
Validating lattice.tokens ... OK (39 tokens, 2 skip, 1 error)
Validating lattice.grammar ... OK (36 rules)
Cross-validating ... OK
All checks passed.
```

Or with errors:

```
Validating broken.tokens ... 2 error(s)
  Line 5: Duplicate token name 'IDENT' (first defined on line 3)
  Line 8: Invalid regex for token 'BAD': ...
Validating broken.grammar ... 1 error(s)
  Undefined rule reference: 'expresion'
Cross-validating ... 1 error(s)
  Error: Grammar references token 'SEMICOL' which is not defined ...

Found 4 error(s). Fix them and try again.
```

**Language-specific entry points:**

| Language | How to invoke |
|----------|--------------|
| Python | `python -m grammar_tools validate ...` *(already done)* |
| Go | `go run ./cmd/grammar-tools validate ...` |
| Ruby | `bundle exec bin/grammar-tools validate ...` |
| TypeScript | `npx ts-node src/cli.ts validate ...` |
| Rust | `cargo run --bin grammar-tools -- validate ...` |
| Elixir | `mix grammar_tools.validate ...` |

---

## GrammarParser Trace Mode

`GrammarParser` accepts an optional `trace` flag (default `false`/`off`). When `true`,
the parser emits one line to stderr for each rule attempt:

```
[TRACE] rule 'qualified_rule' at token 5 (IDENT "h1") → match
[TRACE] rule 'at_rule' at token 5 (IDENT "h1") → fail
[TRACE] rule 'declaration' at token 7 (IDENT "color") → match
```

Format: `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail`

This directly addresses the class of bug encountered during the Lattice cross-language
port, where a missing rule in the grammar caused silent wrong-AST failures that took
hours to diagnose. With trace mode, the developer sees exactly which rule failed at which
token position and can identify the missing production immediately.

**Constructor signatures:**

```python
# Python
GrammarParser(tokens: list[Token], grammar: ParserGrammar, *, trace: bool = False)

# Go
NewGrammarParser(tokens []Token, grammar *ParserGrammar, trace bool) *GrammarParser

# Ruby
GrammarParser.new(tokens, grammar, trace: false)

# TypeScript
new GrammarParser(tokens: Token[], grammar: ParserGrammar, options?: { trace?: boolean })

# Rust
GrammarParser::new(tokens: Vec<Token>, grammar: &ParserGrammar, trace: bool) -> Self

# Elixir
GrammarParser.new(tokens, grammar, trace: false)
```

---

## Token Type Naming Contract

The grammar infrastructure maps grammar token names to language-specific type values.
This table documents the canonical mapping to prevent the class of bug where `IDENT`
in the grammar is `TokenType::Name` in Rust but `TokenType.IDENT` in TypeScript.

| Grammar token name | Python enum | Go string | Ruby symbol | TypeScript enum | Rust variant | Elixir atom |
|---|---|---|---|---|---|---|
| `IDENT` | `TokenType.IDENT` | `"IDENT"` | `:IDENT` | `TokenType.IDENT` | `TokenType::Name` (see note) | `:IDENT` |
| `NUMBER` | `TokenType.NUMBER` | `"NUMBER"` | `:NUMBER` | `TokenType.NUMBER` | `TokenType::Number` | `:NUMBER` |
| `STRING` | `TokenType.STRING` | `"STRING"` | `:STRING` | `TokenType.STRING` | `TokenType::Str` | `:STRING` |
| Custom (`VARIABLE`, etc.) | `TokenType.VARIABLE` | `"VARIABLE"` | `:VARIABLE` | `TokenType.VARIABLE` | stored in `type_name` | `:VARIABLE` |

**Rust note**: The Rust `TokenType` enum has built-in variants (`Name`, `Number`, `Str`,
`EqualsEquals`, etc.) and a custom-token mechanism where `type_name` holds the grammar
name. The `get_token_type_name()` helper returns the grammar name for all tokens; always
use that helper rather than the `TokenType` variant name directly when comparing to
grammar-level token names.

---

## Compile-Time Grammar Embedding (Future)

Grammar files are currently read from disk at runtime using relative paths (e.g.,
`../../../../grammars/lattice.tokens`). This approach has two problems:

1. **Fragile paths** — the path breaks if a package is moved or published to a registry.
2. **Browser incompatibility** — no filesystem access in browser environments (required
   a special Vite `?raw` shim for the Lattice docs playground).

The planned fix is a `grammar-tools embed` subcommand that reads `.tokens` and `.grammar`
files and emits a source file with the content as a string constant:

```
grammar-tools embed lattice.tokens lattice.grammar --lang typescript --out src/grammars.ts
```

Output (TypeScript example):
```typescript
// Auto-generated by grammar-tools embed — do not edit
export const LATTICE_TOKENS = `...`;
export const LATTICE_GRAMMAR = `...`;
```

This turns a runtime disk read into a compile-time baked constant. The generated file is
committed alongside the package and regenerated whenever the grammar files change (via a
Makefile rule or CI step).

**Implementation**: To be done in a follow-up PR after the harmonisation work settles.

---

## Test Strategy

### Token grammar parsing tests
- Parse a minimal valid `.tokens` file → `TokenGrammar` with correct fields
- Parse `skip:` section → `skip_definitions` populated
- Parse `errors:` section → `error_definitions` populated (ignored by lexer)
- Parse `group NAME:` section → `groups` map populated
- Invalid regex → `TokenGrammarError` raised

### Parser grammar parsing tests
- Parse a simple rule → `GrammarRule` with correct EBNF body
- Parse `[optional]`, `{repetition}`, `(group)`, `a | b` alternation
- Invalid syntax → `ParserGrammarError` raised

### Validation tests
- Duplicate token name → error
- Invalid regex → error
- Unused token → warning (not error)
- Undefined rule reference → error
- Unreachable rule → warning

### Cross-validation tests
- Grammar references undefined token → error
- Token defined but never used → warning
- Synthetic tokens (`EOF`, `NEWLINE`) → no error even when not in `.tokens`

### CLI tests
- `validate lattice.tokens lattice.grammar` → exit 0, "All checks passed."
- `validate broken.tokens broken.grammar` → exit 1, error messages printed
- `validate-tokens` and `validate-grammar` subcommands
- Missing file → exit 1 with "File not found" message
- Unknown subcommand → exit 2 with usage message

### Trace mode tests
- Construct `GrammarParser` with `trace=true`
- Parse a simple grammar
- Verify stderr contains `[TRACE]` lines with correct rule names and token positions
- Verify trace mode does not affect the parse result
