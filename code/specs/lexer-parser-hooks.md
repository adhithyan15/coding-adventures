# Lexer & Parser Transform Hooks

## Overview

Many languages require processing steps that don't fit cleanly into the lexer-then-parser pipeline. COBOL needs column stripping before tokenization. C needs macro expansion between lexing and parsing. Lisp needs macro expansion after parsing. Rather than building these into each language's thin wrapper package, we add **pluggable transform hooks** to the base `lexer` and `parser` packages.

Each hook point accepts a chain of transform functions. Languages that don't need transforms (Lisp, JSON, Starlark) ignore them entirely — zero overhead, no API changes for existing code. Languages that do need transforms register them on the lexer or parser before calling `tokenize()` or `parse()`.

## Layer Position

```
Source Text
    │
    ▼
┌──────────────────────────────────────┐
│  Lexer                               │
│                                      │
│  pre_tokenize hooks: str → str       │──── COBOL column strip, C #include,
│           │                          │     LaTeX \input, line splicing
│           ▼                          │
│  tokenize() — existing logic         │
│           │                          │
│  post_tokenize hooks:                │──── C #define expansion, token
│      list[Token] → list[Token]       │     filtering, conditional compilation
│                                      │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│  Parser                              │
│                                      │
│  pre_parse hooks:                    │──── Token normalization,
│      list[Token] → list[Token]       │     disambiguation
│           │                          │
│           ▼                          │
│  parse() — existing logic            │
│           │                          │
│  post_parse hooks:                   │──── Lisp macros, Rust proc macros,
│      ASTNode → ASTNode               │     desugaring, AST rewrites
│                                      │
└──────────────────────────────────────┘
    │
    ▼
  Compiler
```

## Design Principles

### 1. Transforms are pure functions

Every hook is a function with a clear signature. No side effects. No state. The lexer/parser passes data through the chain and gets data back.

```python
TextTransform   = Callable[[str], str]
TokenTransform  = Callable[[list[Token]], list[Token]]
ASTTransform    = Callable[[ASTNode], ASTNode]
```

### 2. Hooks compose left-to-right

If you register three pre-tokenize hooks A, B, C, the source flows through A → B → C before tokenization. Order matters — column stripping must happen before line splicing.

### 3. Zero hooks = zero cost

When no hooks are registered, the lexer and parser behave exactly as they do today. The existing `tokenize()` and `parse()` methods keep their current signatures and return types. No performance penalty for languages that don't use hooks.

### 4. Hooks are optional, not required

Languages register hooks only when they need them. The thin wrapper pattern stays thin:

```python
# lisp-lexer — no hooks needed, nothing changes
def create_lisp_lexer():
    grammar = load_token_grammar("lisp.tokens")
    return GrammarLexer(source, grammar)

# cobol-lexer — adds a pre-tokenize hook
def create_cobol_lexer(source):
    grammar = load_token_grammar("cobol.tokens")
    lexer = GrammarLexer(source, grammar)
    lexer.add_pre_tokenize(strip_cobol_columns)
    return lexer
```

## Public API Changes

### Lexer Package (`lexer`)

#### GrammarLexer additions

```python
class GrammarLexer:
    # ... existing __init__ and tokenize unchanged ...

    def add_pre_tokenize(self, hook: Callable[[str], str]) -> None:
        """Register a text transform to run before tokenization.

        The hook receives the raw source string and returns a
        (possibly modified) source string. Multiple hooks are
        composed left-to-right: A(B(C(source))).

        Use cases:
        - COBOL/FORTRAN column stripping
        - C #include file insertion
        - LaTeX \\input{} expansion
        - Line continuation / splicing (C backslash-newline)
        - Encoding normalization

        Args:
            hook: A function str → str.
        """

    def add_post_tokenize(self, hook: Callable[[list[Token]], list[Token]]) -> None:
        """Register a token transform to run after tokenization.

        The hook receives the full token list (including EOF) and
        returns a (possibly modified) token list. Multiple hooks
        are composed left-to-right.

        Use cases:
        - C #define macro expansion (substitute token sequences)
        - C #ifdef conditional compilation (filter tokens)
        - Token pasting (C ## operator)
        - Inserting synthetic tokens
        - Stripping or reclassifying tokens

        Args:
            hook: A function list[Token] → list[Token].
        """
```

#### Updated `tokenize()` flow

```python
def tokenize(self) -> list[Token]:
    # 1. Run pre-tokenize hooks on source text
    source = self._source
    for hook in self._pre_tokenize_hooks:
        source = hook(source)

    # 2. Tokenize (existing logic, using transformed source)
    tokens = self._do_tokenize(source)

    # 3. Run post-tokenize hooks on token list
    for hook in self._post_tokenize_hooks:
        tokens = hook(tokens)

    return tokens
```

### Parser Package (`lang_parser`)

#### GrammarParser additions

```python
class GrammarParser:
    # ... existing __init__ and parse unchanged ...

    def add_pre_parse(self, hook: Callable[[list[Token]], list[Token]]) -> None:
        """Register a token transform to run before parsing.

        The hook receives the token list and returns a (possibly
        modified) token list. Runs after all lexer hooks.

        Use cases:
        - Token-level disambiguation
        - Injecting synthetic tokens for parser guidance
        - Filtering or merging tokens

        Note: For most token-level transforms, prefer
        GrammarLexer.add_post_tokenize(). Use pre_parse only
        when the transform depends on the parser grammar context.

        Args:
            hook: A function list[Token] → list[Token].
        """

    def add_post_parse(self, hook: Callable[[ASTNode], ASTNode]) -> None:
        """Register an AST transform to run after parsing.

        The hook receives the root ASTNode and returns a (possibly
        modified) ASTNode. Multiple hooks are composed left-to-right.

        Use cases:
        - Lisp defmacro expansion (rewrite s-expressions)
        - Rust macro_rules! expansion
        - Desugaring (transform syntactic sugar into core forms)
        - AST optimization passes
        - Annotation processing

        Args:
            hook: A function ASTNode → ASTNode.
        """
```

#### Updated `parse()` flow

```python
def parse(self) -> ASTNode:
    # 1. Run pre-parse hooks on token list
    tokens = self._tokens
    for hook in self._pre_parse_hooks:
        tokens = hook(tokens)

    # 2. Parse (existing logic, using transformed tokens)
    ast = self._do_parse(tokens)

    # 3. Run post-parse hooks on AST
    for hook in self._post_parse_hooks:
        ast = hook(ast)

    return ast
```

## Data Flow

### Without hooks (status quo — no change)

```
"(+ 1 2)" ──→ GrammarLexer.tokenize() ──→ [LPAREN, SYMBOL, NUMBER, ...] ──→ GrammarParser.parse() ──→ ASTNode
```

### With hooks (COBOL example)

```
"000100 IDENTIFICATION DIVISION.\n000200..."
    │
    ▼ pre_tokenize: strip_cobol_columns
"IDENTIFICATION DIVISION.\n..."
    │
    ▼ tokenize()
[IDENTIFICATION, DIVISION, DOT, ...]
    │
    ▼ parse()
ASTNode(program, [...])
```

### With hooks (C preprocessor example)

```
"#include <stdio.h>\nint main() {...}"
    │
    ▼ pre_tokenize: resolve_includes
"/* contents of stdio.h */\nint main() {...}"
    │
    ▼ tokenize()
[INT, MAIN, LPAREN, RPAREN, LBRACE, ...]
    │
    ▼ post_tokenize: expand_macros
[INT, MAIN, LPAREN, RPAREN, LBRACE, ...] (with macros expanded)
    │
    ▼ parse()
ASTNode(program, [...])
```

### With hooks (Lisp macros example)

```
"(defmacro when (test body) `(if ,test ,body)) (when (> x 0) (print x))"
    │
    ▼ tokenize() → parse()
ASTNode(program, [sexpr(defmacro ...), sexpr(when ...)])
    │
    ▼ post_parse: expand_lisp_macros
ASTNode(program, [sexpr(defmacro ...), sexpr(if (> x 0) (print x))])
```

## Concrete Transform Examples

### COBOL Column Stripper

```python
def strip_cobol_columns(source: str) -> str:
    """Strip COBOL fixed-format columns.

    - Column 1-6: sequence number (discarded)
    - Column 7: indicator (* = comment, - = continuation, D = debug)
    - Columns 8-72: source code (kept)
    - Columns 73-80: identification (discarded)
    """
    lines = []
    for line in source.split("\n"):
        if len(line) < 7:
            lines.append("")
            continue
        indicator = line[6] if len(line) > 6 else " "
        if indicator == "*":
            continue  # comment line
        # Extract columns 7-72
        code = line[7:72] if len(line) > 7 else ""
        if indicator == "-":
            # Continuation — append to previous line
            if lines:
                lines[-1] = lines[-1].rstrip() + code.lstrip()
                continue
        lines.append(code)
    return "\n".join(lines)
```

### C Include Resolver

```python
def resolve_includes(source: str) -> str:
    """Expand #include directives by inlining file contents."""
    import re
    result = []
    for line in source.split("\n"):
        match = re.match(r'#include\s+"([^"]+)"', line)
        if match:
            path = match.group(1)
            with open(path) as f:
                result.append(f.read())
        else:
            result.append(line)
    return "\n".join(result)
```

### Lisp Macro Expander (post-parse)

```python
def expand_lisp_macros(ast: ASTNode) -> ASTNode:
    """Walk the AST and expand any known macro invocations.

    First pass: collect defmacro definitions.
    Second pass: rewrite invocations using the macro templates.
    """
    macros = collect_macro_definitions(ast)
    return rewrite_macro_invocations(ast, macros)
```

## Test Strategy

### Lexer Hook Tests

1. **Pre-tokenize: identity** — A no-op hook `lambda s: s` produces identical tokens to no hook.
2. **Pre-tokenize: strip** — A column-stripping hook on COBOL source produces valid tokens.
3. **Pre-tokenize: composition** — Two hooks compose left-to-right (A then B, not B then A).
4. **Post-tokenize: identity** — A no-op hook `lambda ts: ts` produces identical tokens.
5. **Post-tokenize: filter** — A hook that removes comment tokens works correctly.
6. **Post-tokenize: expand** — A hook that replaces one token with multiple works correctly.
7. **No hooks baseline** — Existing tokenization still works with no hooks registered.

### Parser Hook Tests

8. **Pre-parse: identity** — No-op hook produces identical AST.
9. **Pre-parse: filter** — Removing tokens before parsing changes the AST correctly.
10. **Post-parse: identity** — No-op hook produces identical AST.
11. **Post-parse: rewrite** — A desugaring hook transforms the AST as expected.
12. **Post-parse: composition** — Multiple hooks compose left-to-right.
13. **No hooks baseline** — Existing parsing still works with no hooks registered.

### Integration Tests

14. **COBOL column strip + tokenize** — Full pipeline with pre-tokenize hook.
15. **Token filtering + parse** — Post-tokenize hook feeds cleaned tokens to parser.
16. **Parse + macro expand** — Post-parse hook rewrites AST correctly.

## Implementation Notes

### Storage

Hooks are stored as ordered lists on the lexer/parser instance:

```python
class GrammarLexer:
    def __init__(self, source, grammar):
        # ... existing init ...
        self._pre_tokenize_hooks: list[Callable[[str], str]] = []
        self._post_tokenize_hooks: list[Callable[[list[Token]], list[Token]]] = []
```

### Existing API Compatibility

The `Lexer` class (hand-written lexer) gets the same hook API as `GrammarLexer`. Both use `tokenize()` as the entry point. The hooks wrap the internal tokenization logic.

### Error Handling

If a hook raises an exception, it propagates uncaught. No special error wrapping — the hook author is responsible for clear error messages. A column-stripping hook that encounters a malformed line should raise a descriptive error.

### Performance Consideration

Hook dispatch is a simple list iteration — O(n) where n is the number of hooks (typically 0-3). No reflection, no dynamic dispatch, no overhead when no hooks are registered.

## What This Enables

| Language | Hook Type | Transform |
|----------|-----------|-----------|
| COBOL | lexer pre_tokenize | Column stripping |
| FORTRAN 90+ | lexer pre_tokenize | Free-form line continuation |
| C | lexer pre_tokenize | `#include` resolution |
| C | lexer post_tokenize | `#define` macro expansion |
| C | lexer post_tokenize | `#ifdef` conditional compilation |
| LaTeX | lexer pre_tokenize | `\input{}` file inclusion |
| Lisp | parser post_parse | `defmacro` expansion |
| Rust | parser post_parse | `macro_rules!` expansion |
| Any | lexer post_tokenize | Debug logging, token counting |
| Any | parser post_parse | Desugaring, optimization |
