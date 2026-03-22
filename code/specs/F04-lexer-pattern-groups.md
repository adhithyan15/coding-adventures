# F04: Lexer Pattern Groups & Callback Hooks

## Overview

This spec extends the grammar-driven lexer infrastructure with **pattern groups**
and **on-token callback hooks**. Pattern groups are named sets of token definitions
in `.tokens` files. Callbacks are user code that fires on every token match and can
push/pop groups on a stack, enabling context-sensitive lexing.

This is the foundation for XML and HTML lexing, where the same character has
different meaning depending on position (e.g., `=` is a delimiter inside tags but
text content outside them).

### Relationship to Existing Specs

- **02-lexer.md**: The base lexer spec. This extends it without breaking compatibility.
- **lexer-parser-hooks.md**: Pre/post tokenize transforms (str→str, tokens→tokens).
  Those are *batch* transforms on the full source or token list. This spec adds
  *streaming* hooks that fire per-token during tokenization. Complementary, not
  overlapping.

## Design Principles

1. **Grammar files are declarative** — they define pattern groups (named sets of
   patterns) but contain zero transition logic.
2. **Callbacks are imperative** — user code decides when to switch groups, written
   in each language's native code.
3. **Stackable groups** — push a group to enter it, pop to return. The stack
   supports nesting (e.g., CDATA inside default mode in XML).
4. **Full backward compatibility** — no groups defined = flat pattern list. No
   callback registered = no overhead. All existing lexers unchanged.

## Extended `.tokens` Format

### `group NAME:` sections

```
# Patterns outside any group belong to the implicit "default" group.
TEXT        = /[^<&]+/
TAG_OPEN    = "<"

# Named group — active only when pushed onto the stack.
group tag:
  TAG_NAME  = /[a-zA-Z_][\w.-]*/
  EQUALS    = "="
  TAG_CLOSE = ">"

group cdata:
  CDATA_TEXT = /([^\]]|\](?!\]>))+/
  CDATA_END  = "]]>"
```

### Rules

- **Implicit default**: Patterns outside any `group:` section belong to the
  `default` group. You never write `group default:` (the parser rejects it).
- **Group names**: Lowercase identifiers (`[a-z_][a-z0-9_]*`). Distinguishes
  from UPPER_CASE token names.
- **Content**: Indented lines within a group section are token definitions,
  following the same `NAME = /pattern/` or `NAME = "literal"` syntax. Aliases
  (`-> TYPE`) work inside groups.
- **Global sections**: `skip:`, `errors:`, `keywords:`, `reserved:` remain
  global — they are not per-group. Skip patterns are tried before the active
  group's patterns at every position.
- **No transition logic**: The grammar file never says "when you see TOKEN_X,
  switch to group Y." That logic lives in the callback.
- **Reserved names**: Group names cannot be `default`, `skip`, `keywords`,
  `reserved`, `errors`.

## Data Structures

### PatternGroup (new)

```python
@dataclass(frozen=True)
class PatternGroup:
    name: str
    definitions: list[TokenDefinition]
```

### TokenGrammar (extended)

```python
@dataclass
class TokenGrammar:
    definitions: list[TokenDefinition]         # Default group patterns
    keywords: list[str]
    mode: str | None
    skip_definitions: list[TokenDefinition]
    reserved_keywords: list[str]
    escape_mode: str | None
    error_definitions: list[TokenDefinition]
    groups: dict[str, PatternGroup]            # NEW — named groups
```

When `groups` is empty, the lexer uses `definitions` as the flat list — identical
to current behavior. When groups are defined, `definitions` becomes the `default`
group's patterns.

## LexerContext API

The context is passed to the callback. It provides controlled access to group
stack manipulation and token emission.

```python
class LexerContext:
    def push_group(self, group_name: str) -> None:
        """Push a group onto the stack. Active for the NEXT token match.
        Raises ValueError if group_name not defined."""

    def pop_group(self) -> None:
        """Pop current group. No-op if only default remains."""

    def active_group(self) -> str:
        """Name of the currently active group."""

    def group_stack_depth(self) -> int:
        """Stack depth (always >= 1)."""

    def emit(self, token: Token) -> None:
        """Inject a synthetic token after the current one.
        Emitted tokens do NOT trigger the callback (prevents loops)."""

    def suppress(self) -> None:
        """Swallow the current token (don't add to output)."""

    def peek(self, offset: int = 1) -> str:
        """Peek at source character at offset past current token.
        Returns '' past EOF."""

    def peek_str(self, length: int) -> str:
        """Peek at next `length` characters past current token."""

    def set_skip_enabled(self, enabled: bool) -> None:
        """Toggle skip pattern processing. Persists until changed.
        Useful for groups where whitespace is significant (CDATA, comments)."""
```

### Elixir Variant

Elixir's functional style uses return values instead of mutation:

```elixir
@type action :: {:push_group, String.t()}
              | :pop_group
              | {:emit, Token.t()}
              | :suppress
              | {:set_skip_enabled, boolean()}

@type callback :: (Token.t(), LexerContext.t() -> [action()])
```

## Callback Registration

```python
class GrammarLexer:
    def set_on_token(self, callback: Callable[[Token, LexerContext], None] | None) -> None:
        """Register a callback for every token match.

        Only one callback. Pass None to clear.

        NOT invoked for:
        - Skip pattern matches (no token produced)
        - Tokens from ctx.emit() (prevents infinite loops)
        - The EOF token
        """
```

## GrammarLexer Changes

### New state

- `_group_stack: list[str]` — bottom is always `"default"`, top is active
- `_group_patterns: dict[str, list[compiled]]` — compiled patterns per group
- `_on_token` — optional callback
- `_skip_enabled: bool` — togglable by callback

### Modified main loop

```python
# 1. Skip patterns (global, unless disabled)
if self._skip_enabled and self._try_skip():
    continue

# 2. Try active group's patterns
active = self._group_stack[-1]
token = self._try_match_token_in_group(active)

if token and self._on_token:
    ctx = LexerContext(self)
    self._on_token(token, ctx)
    if not ctx._suppressed:
        tokens.append(token)
    tokens.extend(ctx._emitted)
    # Apply group stack changes
    for action in ctx._actions:
        ...
    self._skip_enabled = ctx._skip_enabled
elif token:
    tokens.append(token)
```

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Pop at bottom (only default) | No-op |
| Emitted tokens trigger callback? | No — prevents infinite loops |
| Skip patterns per-group? | No, global. Use `set_skip_enabled(false)` for significant whitespace |
| Error patterns per-group? | No, global fallback |
| Indentation mode + groups? | Orthogonal. In practice, not used together |
| Suppress + emit | Current token suppressed, emitted tokens still output |
| Multiple pushes in one callback | Applied in order |
| Push group with no matching pattern | Next iteration fails to match → falls to error patterns → LexerError |

## Validation Changes

New checks in `validate_token_grammar`:

1. Duplicate group names → error
2. `group default:` → error (reserved)
3. Group name format → must match `[a-z_][a-z0-9_]*`
4. Group name conflicts with section names (`skip`, `keywords`, `reserved`, `errors`) → error
5. Empty group → warning

## XML Example (Proving the Design)

### xml.tokens

```
escapes: none

skip:
  WHITESPACE = /[ \t\r\n]+/

# Default group: content between tags
TEXT             = /[^<&]+/
ENTITY_REF       = /&[a-zA-Z]+;/
CHAR_REF         = /&#[0-9]+;|&#x[0-9a-fA-F]+;/
OPEN_TAG_START   = "<"
CLOSE_TAG_START  = "</"
COMMENT_START    = "<!--"
CDATA_START      = "<![CDATA["
PI_START         = "<?"

group tag:
  TAG_NAME       = /[a-zA-Z_][a-zA-Z0-9_:.-]*/
  ATTR_EQUALS    = "="
  ATTR_VALUE_DQ  = /"[^"]*"/ -> ATTR_VALUE
  ATTR_VALUE_SQ  = /'[^']*'/ -> ATTR_VALUE
  TAG_CLOSE      = ">"
  SELF_CLOSE     = "/>"
  SLASH          = "/"

group comment:
  COMMENT_TEXT   = /([^-]|-(?!->))+/
  COMMENT_END    = "-->"

group cdata:
  CDATA_TEXT     = /([^\]]|\](?!\]>))+/
  CDATA_END      = "]]>"

group pi:
  PI_TEXT        = /([^?]|\?(?!>))+/
  PI_END         = "?>"
```

### XML callback (Python)

```python
def xml_on_token(token: Token, ctx: LexerContext) -> None:
    match token.type:
        case "OPEN_TAG_START" | "CLOSE_TAG_START":
            ctx.push_group("tag")
        case "TAG_CLOSE" | "SELF_CLOSE":
            ctx.pop_group()
        case "COMMENT_START":
            ctx.push_group("comment")
            ctx.set_skip_enabled(False)
        case "COMMENT_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)
        case "CDATA_START":
            ctx.push_group("cdata")
            ctx.set_skip_enabled(False)
        case "CDATA_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)
        case "PI_START":
            ctx.push_group("pi")
            ctx.set_skip_enabled(False)
        case "PI_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)
```

### Trace: `<div class="main">Hello</div>`

```
Pos  Group    Match              Callback
---  -------  ---------------    --------
0    default  OPEN_TAG_START     push("tag")
1    tag      TAG_NAME(div)      —
4    tag      (skip WS)          —
5    tag      TAG_NAME(class)    —
10   tag      ATTR_EQUALS        —
11   tag      ATTR_VALUE         —
17   tag      TAG_CLOSE          pop()
18   default  TEXT(Hello)        —
23   default  CLOSE_TAG_START    push("tag")
25   tag      TAG_NAME(div)      —
28   tag      TAG_CLOSE          pop()
29   default  EOF
```

## Implementation Order

| Step | What | Commit |
|------|------|--------|
| 0 | This spec (F04) | `spec(lexer): add F04 pattern groups and callback hooks` |
| 1 | Python grammar-tools: PatternGroup, group parsing, validation | `feat(grammar-tools): pattern group parsing and validation` |
| 2 | Python lexer: LexerContext, group stack, callbacks | `feat(lexer): on-token callbacks and pattern group stack` |
| 3 | Port grammar-tools to Ruby, TS, Go, Rust, Elixir | `feat(grammar-tools): pattern groups (all languages)` |
| 4 | Port lexer to Ruby, TS, Go, Rust, Elixir | `feat(lexer): on-token callbacks (all languages)` |
| 5 | xml.tokens grammar + xml-lexer (all 6 languages) | `feat(xml-lexer): XML tokenizer with group callbacks` |

## Testing Strategy

### Grammar-tools tests
- Parse `.tokens` with groups → correct PatternGroup objects
- Parse existing `.tokens` files → empty groups (backward compat)
- Validation: duplicate groups, reserved `default`, bad names, section name conflicts

### Lexer tests
- LexerContext unit tests: push/pop/emit/suppress in isolation
- No groups + no callback → identical to current behavior
- Groups + callback → correct group transitions
- Edge cases: pop at bottom, multiple pushes, suppress+emit, set_skip_enabled

### XML lexer tests
- Basic tags: `<div>text</div>`
- Attributes: `<a href="url">` with single/double quotes
- Self-closing: `<br/>`
- Comments: `<!-- comment -->`
- CDATA: `<![CDATA[raw]]>`
- Processing instructions: `<?xml version="1.0"?>`
- Nested structures: tags within tags
- Entity/character references: `&amp;`, `&#65;`
- Mixed content: text interspersed with elements

### Backward compatibility
- All existing lexer test suites pass unchanged (JSON, TOML, CSS, Starlark)
