# CSS Parser — Full CSS3 Infrastructure Stress Test

## Overview

This specification defines a CSS3 parser built on the grammar-driven lexer/parser
infrastructure. Unlike JSON (trivially simple) and Starlark (moderate complexity),
CSS pushes the infrastructure hard with compound tokens, context-dependent
disambiguation, complex selectors, nested at-rules, and CSS-specific escape sequences.

**Goals:**
1. Validate the grammar-driven infrastructure handles a real-world, complex language
2. Identify and fix infrastructure gaps discovered during implementation
3. Produce a working CSS3 lexer and parser via thin wrappers

## Infrastructure Enhancements

Three infrastructure changes are required for full CSS3 support. All are
backward-compatible — existing JSON and Starlark grammars continue to work
without modification.

### Enhancement 1: Configurable Escape Processing

**Problem:** The lexer's `_process_escapes()` is hardcoded to process any
token named STRING with JSON-style escape semantics (`\n`, `\t`, `\\`, `\"`,
`\uXXXX`). CSS strings use different escape semantics — `\26` means character
U+0026 (`&`), with 1-6 hex digits and an optional trailing space.

**Solution:** Add an `escapes:` directive to `.tokens` files that controls
escape processing mode:
- No directive (default): current behavior (JSON-style escapes + quote stripping)
- `escapes: none`: strip surrounding quotes but do NOT process escape sequences

CSS uses `escapes: none` because CSS escape semantics are significantly different
from JSON/Starlark and are better handled as a post-parse semantic step.

**Files modified:**
- `grammar_tools/token_grammar.py` — parse `escapes:` directive, add `escape_mode` to `TokenGrammar`
- `lexer/grammar_lexer.py` — check `escape_mode` before processing

### Enhancement 2: Error Token Support

**Problem:** The lexer raises `LexerError` on malformed input. CSS requires
`BAD_STRING` and `BAD_URL` error tokens for graceful degradation.

**Solution:** Add an `errors:` section to `.tokens` files. When no normal
token or skip pattern matches, the lexer tries error patterns before raising
`LexerError`. Error tokens are emitted with an `is_error` flag.

**Files modified:**
- `grammar_tools/token_grammar.py` — parse `errors:` section, add `error_definitions` to `TokenGrammar`
- `lexer/grammar_lexer.py` — try error patterns as fallback

### Enhancement 3: Forgiving Selector Lists

**Problem:** CSS `:is()`, `:where()`, `:not()`, `:has()` use "forgiving"
selector lists — invalid selectors are skipped, not fatal.

**Solution:** Handle this at the CSS parser wrapper level (not in the grammar
engine). The wrapper catches parse errors at comma boundaries within these
pseudo-class functions and continues parsing.

## Token Design (`css.tokens`)

~35 token types with careful first-match-wins ordering:

### Priority Ordering Rationale

1. **Skip patterns first**: `COMMENT = /\/\*[\s\S]*?\*\//`, `WHITESPACE = /[ \t\r\n]+/`
2. **Strings before numbers**: different starting characters, no conflict
3. **DIMENSION > PERCENTAGE > NUMBER**: `10px` → one token, not `NUMBER(10) IDENT(px)`
4. **URL_TOKEN > FUNCTION > IDENT**: `url(path)` → one token; `rgb(` → one token; `color` → IDENT
5. **CUSTOM_PROPERTY before IDENT**: `--var-name` must not be `IDENT(-)` `IDENT(-var-name)`
6. **Multi-char operators before single-char**: `::` before `:`, `~=` before `~`

### Token Types

| Token | Pattern Type | Example |
|-------|-------------|---------|
| STRING | regex (DQ/SQ aliased) | `"hello"`, `'world'` |
| DIMENSION | regex | `10px`, `2em`, `100vh` |
| PERCENTAGE | regex | `50%`, `33.3%` |
| NUMBER | regex | `42`, `3.14`, `1e10` |
| HASH | regex | `#fff`, `#header` |
| AT_KEYWORD | regex | `@media`, `@import` |
| URL_TOKEN | regex | `url(path/to/file.png)` |
| FUNCTION | regex | `rgb(`, `calc(`, `var(` |
| CDO / CDC | literal | `<!--`, `-->` |
| CUSTOM_PROPERTY | regex | `--main-color` |
| IDENT | regex | `color`, `-webkit-transform` |
| UNICODE_RANGE | regex | `U+0025-00FF` |
| Operators | literal | `::`, `~=`, `|=`, `^=`, `$=`, `*=` |
| Delimiters | literal | `{`, `}`, `(`, `)`, `[`, `]`, `;`, `:`, etc. |
| AMPERSAND | literal | `&` (CSS nesting) |

### Error Tokens

| Token | Pattern | Purpose |
|-------|---------|---------|
| BAD_STRING | `/"[^"]*$/` | Unclosed double-quoted string |
| BAD_URL | `/url\([^)]*$/` | Unclosed URL function |

## Grammar Design (`css.grammar`)

~45-50 rules covering full CSS3 selector and rule syntax.

### Structure

```
stylesheet
├── rule (repeated)
│   ├── at_rule: AT_KEYWORD prelude (SEMICOLON | block)
│   └── qualified_rule: selector_list block
│       ├── selector_list: complex_selector { COMMA complex_selector }
│       │   └── complex_selector: compound_selector { combinator compound_selector }
│       │       └── compound_selector: type_selector? subclass_selector* pseudo_element*
│       └── block: LBRACE block_contents RBRACE
│           └── block_item: declaration | qualified_rule | at_rule
│               └── declaration: property COLON value_list [priority] SEMICOLON
│                   └── value: DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | function_call | ...
```

### Key Design Decisions

1. **Descendant combinator**: CSS uses whitespace as a combinator, but whitespace
   is skipped by the lexer. Adjacent `compound_selector` nodes in the parse tree
   imply descendant relationship.

2. **`!important`**: Parsed as `BANG "important"` — the `BANG` token followed by
   a literal match on the IDENT value `"important"`.

3. **At-rule prelude**: Unified as `{ at_prelude_token }` — any sequence of
   tokens. The grammar doesn't distinguish `@media` preludes from `@import`
   preludes. Semantic analysis differentiates them post-parse.

4. **CSS nesting**: `AMPERSAND` token (`&`) enables nested selector references.
   `block_contents` allows `qualified_rule` inside blocks.

5. **Calc expressions**: Operator precedence encoded in grammar structure
   (additive > multiplicative > primary), similar to Starlark's expression grammar.

6. **`:nth-child()` An+B**: Parsed via `an_plus_b` rule matching patterns like
   `2n+1`, `-n`, `odd`, `even`.

## Python Packages

### `css-lexer` (thin wrapper)

- `tokenize_css(source: str) -> list[Token]`
- Loads `css.tokens` from `code/grammars/`
- ~40 test cases targeting compound tokens, at-keywords, functions, comments, error tokens

### `css-parser` (thin wrapper)

- `parse_css(source: str) -> ASTNode`
- Loads `css.grammar` from `code/grammars/`
- Includes forgiving selector list logic for `:is()`, `:where()`, `:not()`, `:has()`
- ~30 test cases targeting selectors, at-rules, values, calc, nesting

## Test Strategy

### Lexer Tests (~40 cases)

1. **Basic tokens**: identifiers, numbers, strings, hash, at-keywords
2. **Compound tokens** (primary stress test): DIMENSION vs NUMBER+IDENT disambiguation
3. **Function tokens**: FUNCTION vs IDENT, URL_TOKEN vs FUNCTION
4. **Multi-line comments**: single-line, multi-line, between tokens
5. **Operators**: priority ordering (`::` vs `:`, `~=` vs `~`)
6. **Error tokens**: unclosed strings, unclosed URLs
7. **Vendor prefixes**: `-webkit-transform`, `-moz-user-select`
8. **Custom properties**: `--main-color`, `--bg`
9. **Complex inputs**: full CSS rules, selectors, declarations

### Parser Tests (~30 cases)

1. **Simple rules**: empty rules, single declaration, multiple declarations
2. **Selectors**: type, class, ID, attribute, pseudo-class, pseudo-element, combinators
3. **At-rules**: `@import`, `@charset`, `@media` (with nested rules), `@keyframes`, `@font-face`
4. **Values**: dimensions, percentages, functions (rgb, calc, var), `!important`
5. **Nesting**: CSS nesting with `&`
6. **Forgiving selectors**: `:is()`, `:where()` with mixed valid/invalid selectors
7. **Edge cases**: empty stylesheet, comments only, deeply nested structures

## Verification

```bash
# Infrastructure backward compatibility
cd code/packages/python/lexer && uv run pytest -v
cd code/packages/python/grammar-tools && uv run pytest -v
cd code/packages/python/json-lexer && uv run pytest -v
cd code/packages/python/json-parser && uv run pytest -v

# CSS packages
cd code/packages/python/css-lexer && uv run pytest -v --tb=short
cd code/packages/python/css-parser && uv run pytest -v --tb=short

# Build tool
./build-tool -dry-run
```
