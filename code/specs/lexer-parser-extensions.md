# Lexer & Parser Infrastructure Extensions

## Overview

This spec extends the grammar-driven lexer and parser with eight generic capabilities
needed to support full parsing of complex languages (JavaScript, TypeScript, Rust, Go,
etc.). All extensions are language-agnostic — language-specific disambiguation logic
lives in language packages, not in the generic engines.

### Relationship to Existing Specs

- **02-lexer.md**: The base lexer spec. This extends it without breaking compatibility.
- **03-parser.md**: The base parser spec. This extends it without breaking compatibility.
- **F04-lexer-pattern-groups.md**: Pattern groups and on-token callbacks. This spec adds
  new methods to the same `LexerContext` interface.
- **lexer-parser-hooks.md**: Pre/post transform hooks. Unchanged by this spec.
- **grammar-tools.md**: Grammar file formats. This spec adds new constructs to both
  `.tokens` and `.grammar` formats.

## Design Principles

1. **Generic capabilities only** — no language-specific logic in engines.
2. **Callback-driven disambiguation** — language packages use callbacks and hooks for
   context-sensitive behavior.
3. **Full backward compatibility** — all existing grammars and packages work unchanged.
4. **Idiomatic per-language** — each language implementation uses its natural patterns.

---

## Extension 1: Token Lookbehind

### Problem

The `LexerContext` provides `peek()` and `peekStr()` for forward lookahead in the
source string, but there is no way to inspect the previously emitted token. Many
languages need this:

- JavaScript/Ruby: `/` is regex after `=`, `(`, `,` but division after `)`, `]`, names
- Rust/TypeScript: `<` is generics after type names but comparison after expressions
- C/C++: `*` is pointer declaration or multiplication depending on context

### Solution

Add `previousToken()` method to `LexerContext`.

### LexerContext API Addition

```typescript
/**
 * Return the most recently emitted token, or null at the start of input.
 * Suppressed tokens are not counted — this returns the last token that
 * actually made it into the output list.
 */
previousToken(): Token | null
```

### Implementation

- Add `_lastEmittedToken: Token | null` field to `GrammarLexer`.
- Update after each token push (including callback-emitted tokens).
- Pass to `LexerContext` constructor.
- Reset on `tokenize()` entry.

### Usage Example (JavaScript lexer package)

```typescript
const REGEX_PRECEDING = new Set([
  "EQUALS", "LPAREN", "LBRACKET", "LBRACE", "COMMA", "SEMICOLON",
  "COLON", "RETURN", "KEYWORD", "PLUS", "MINUS", "STAR", "BANG",
]);

lexer.setOnToken((token, ctx) => {
  if (token.type === "SLASH") {
    const prev = ctx.previousToken();
    if (!prev || REGEX_PRECEDING.has(prev.type)) {
      ctx.suppress();
      // Re-lex as regex literal using peekStr()...
    }
  }
});
```

---

## Extension 2: Bracket Depth Tracking

### Problem

Bracket depth tracking currently exists only in indentation mode (for implicit line
joining in Python). Standard mode needs it too:

- JavaScript: template literal `${expr}` — need to know when `}` at brace-depth 0
  closes the interpolation vs being part of a nested object literal
- Kotlin/Ruby: string interpolation with `#{expr}` or `${expr}`
- Shell: command substitution `$(cmd)`

### Solution

Track bracket depths for all three bracket types in standard mode, expose via
`LexerContext`.

### LexerContext API Addition

```typescript
/**
 * Return the current nesting depth for a specific bracket type, or the
 * total depth across all types if no argument is given.
 *
 * Depth starts at 0 and increments on openers, decrements on closers.
 * Never goes below 0 (unmatched closers are clamped).
 */
bracketDepth(kind?: "paren" | "bracket" | "brace"): number
```

### Implementation

- Add `_bracketDepths: { paren: number; bracket: number; brace: number }` to
  `GrammarLexer`.
- After each token match in `_tokenizeStandard()`, check token value against the
  six bracket characters and update the appropriate counter.
- Expose via `LexerContext`.
- Reset all to 0 on `tokenize()` entry.

---

## Extension 3: Token Metadata Flags

### Problem

Some token metadata is neither type nor value but affects parsing:

- JavaScript/Go: "preceded by newline" for automatic semicolon insertion (ASI)
- Many languages: context-sensitive keywords (`async`, `yield`, `await` in JS;
  `override` in C++; `get`/`set` in JS) that are sometimes identifiers

### Solution

Add an optional `flags` bitmask to the `Token` interface and a `context_keywords`
section to the `.tokens` format.

### Token Interface Change

```typescript
export interface Token {
  readonly type: string;
  readonly value: string;
  readonly line: number;
  readonly column: number;
  readonly flags?: number;  // NEW: optional bitmask for metadata
}

// Standard flag constants
export const TOKEN_PRECEDED_BY_NEWLINE = 1;
export const TOKEN_CONTEXT_KEYWORD = 2;
```

### LexerContext API Addition

```typescript
/**
 * Return true if a newline appeared between the previous token and
 * the current token (different line numbers).
 */
precededByNewline(): boolean
```

### `.tokens` Format Addition

```
# Context keywords: emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
# Language-specific callbacks can promote them to KEYWORD contextually.
context_keywords:
  async
  await
  yield
  get
  set
```

### TokenGrammar Addition

```typescript
interface TokenGrammar {
  // ... existing fields ...
  readonly contextKeywords: readonly string[];  // NEW
}
```

---

## Extension 4: Syntactic Predicates

### Problem

The `.grammar` format has no way to express "match only if followed by X" or "match
only if NOT followed by X" without consuming input. Many grammars need this:

- JavaScript: `(a, b) => body` — need positive lookahead for `=>` after `)` to
  distinguish from parenthesized expression
- JavaScript: `x++` — need negative lookahead for newline (ASI restriction)
- C: declaration vs expression statement disambiguation

### Solution

Add `&element` (positive lookahead) and `!element` (negative lookahead) prefix
operators to the `.grammar` format. These are standard PEG operators.

### `.grammar` Format Addition

```
# &element — positive lookahead: succeed if element matches, consume nothing
arrow_function = LPAREN [ param { COMMA param } ] RPAREN &ARROW arrow_body ;

# !element — negative lookahead: succeed if element does NOT match, consume nothing
postfix_expr = left_hand_side !NEWLINE ( "++" | "--" ) ;
```

### GrammarElement Type Additions

```typescript
export interface PositiveLookahead {
  readonly type: "positive_lookahead";
  readonly element: GrammarElement;
}

export interface NegativeLookahead {
  readonly type: "negative_lookahead";
  readonly element: GrammarElement;
}

export type GrammarElement =
  | RuleReference | TokenReference | Literal
  | Group | Optional | Repetition | Alternation | Sequence
  | PositiveLookahead    // NEW
  | NegativeLookahead;   // NEW
```

### Parser Implementation

```
positive_lookahead:
  save pos
  result = matchElement(inner)
  restore pos
  return result !== null ? [] : null

negative_lookahead:
  save pos
  result = matchElement(inner)
  restore pos
  return result === null ? [] : null
```

Lookahead predicates produce no AST children (they return empty array on success).

---

## Extension 5: AST Position Information

### Problem

`ASTNode` has only `ruleName` and `children`. Tokens have `line`/`column`, but
intermediate AST nodes do not. This makes error reporting, source maps, and IDE
integration impossible at the AST level.

### Solution

Compute position info automatically from child tokens.

### ASTNode Interface Change

```typescript
export interface ASTNode {
  readonly ruleName: string;
  readonly children: ReadonlyArray<ASTNode | Token>;
  readonly startLine?: number;    // NEW
  readonly startColumn?: number;  // NEW
  readonly endLine?: number;      // NEW
  readonly endColumn?: number;    // NEW
}
```

### Implementation

After constructing a successful parse result in `parseRule()`:
1. Walk `children` to find the first leaf token → `startLine`, `startColumn`
2. Walk `children` to find the last leaf token → `endLine`, `endColumn`
3. If no tokens (empty repetition), positions are undefined.

The `isASTNode()` type guard remains unchanged (checks `"ruleName"`).

---

## Extension 6: AST Walking Utility

### Problem

Cover grammar rewriting, tree transforms, and semantic analysis all require
depth-first traversal. Without a shared utility, every language package writes
its own tree walker.

### Solution

Export generic traversal functions from the parser package.

### API

```typescript
interface ASTVisitor {
  enter?(node: ASTNode, parent: ASTNode | null): ASTNode | void;
  leave?(node: ASTNode, parent: ASTNode | null): ASTNode | void;
}

/**
 * Depth-first walk of an AST tree. Visitor callbacks can return a
 * replacement node or void (keep original). Tokens are not visited.
 */
export function walkAST(node: ASTNode, visitor: ASTVisitor): ASTNode;

/**
 * Find all nodes matching a rule name (depth-first).
 */
export function findNodes(node: ASTNode, ruleName: string): ASTNode[];

/**
 * Collect all tokens in depth-first order, optionally filtered by type.
 */
export function collectTokens(node: ASTNode, type?: string): Token[];
```

---

## Extension 7: One-or-More Repetition

### Problem

The common pattern `element { element }` (at least one) appears in nearly every
grammar. EBNF traditionally supports this with `{ element }+`.

### Solution

Add `+` suffix to repetition syntax.

### `.grammar` Format Addition

```
# { element }+ — one or more (fails if zero matches)
statements = { statement }+ ;
```

### GrammarElement Type Addition

```typescript
export interface OneOrMoreRepetition {
  readonly type: "one_or_more";
  readonly element: GrammarElement;
}
```

### Parser Implementation

Match one required occurrence, then loop for zero-or-more additional. Fail if
the first match fails.

---

## Extension 8: Separator Repetition

### Problem

Comma-separated lists appear in every language: function arguments, array
literals, imports, destructuring, enum members. The current encoding:
```
param_list = expression { COMMA expression } ;
```
is verbose and duplicates the `expression` reference.

### Solution

Add `//` separator syntax inside repetition braces.

### `.grammar` Format Addition

```
# { element // separator } — zero or more separated
param_list = LPAREN [ { expression // COMMA } ] RPAREN ;

# { element // separator }+ — one or more separated
items = { expression // COMMA }+ ;
```

### GrammarElement Type Addition

```typescript
export interface SeparatedRepetition {
  readonly type: "separated_repetition";
  readonly element: GrammarElement;
  readonly separator: GrammarElement;
  readonly atLeastOne: boolean;  // true if + suffix present
}
```

### Parser Implementation

For zero-or-more: match `[ element { separator element } ]`
For one-or-more: match `element { separator element }`

---

## Extension 9: Opt-In Rich Source Preservation

### Problem

Formatter pipelines need more than just token types and coarse line/column
spans. To preserve comments, blank lines, explicit grouping, and precise source
ranges, the generic lexer and parser need to retain:

- exact source offsets
- end positions
- named skip matches such as whitespace and comments
- a stable mapping from AST nodes back to the token stream

The current lexer consumes skip patterns silently, and the parser only keeps
`ruleName`, `children`, and coarse start/end line-column information.

### Design Goal

Preserve formatter-grade source detail **without** slowing down ordinary
parsing. The feature must therefore be opt-in, enabled only by callers such as
formatter pipelines.

### Solution

Add an optional `preserveSourceInfo` flag to the grammar-driven lexer and
parser. When the flag is disabled, behavior and performance stay as they are
today. When enabled:

- the lexer enriches tokens with offsets, end positions, token indices, and
  leading trivia
- the parser propagates source ranges and token-index spans onto AST nodes

### Lexer API Additions

```typescript
export interface Trivia {
  readonly type: string;
  readonly value: string;
  readonly line: number;
  readonly column: number;
  readonly endLine: number;
  readonly endColumn: number;
  readonly startOffset: number;
  readonly endOffset: number;
}

export interface Token {
  readonly type: string;
  readonly value: string;
  readonly line: number;
  readonly column: number;
  readonly flags?: number;

  // Present only when preserveSourceInfo is enabled.
  readonly endLine?: number;
  readonly endColumn?: number;
  readonly startOffset?: number;
  readonly endOffset?: number;
  readonly tokenIndex?: number;
  readonly leadingTrivia?: readonly Trivia[];
}

export interface GrammarLexerOptions {
  readonly preserveSourceInfo?: boolean;
}

constructor(source: string, grammar: TokenGrammar, options?: GrammarLexerOptions)

export function grammarTokenize(
  source: string,
  grammar: TokenGrammar,
  options?: GrammarLexerOptions,
): Token[];
```

### Lexer Semantics

When `preserveSourceInfo` is enabled:

1. Every emitted token gains:
   - `startOffset` / `endOffset` as half-open source ranges
   - `endLine` / `endColumn` as exclusive end positions
   - `tokenIndex` as the token's stable position in the emitted token stream

2. Skip matches are preserved as `Trivia` values and attached to the next emitted
   token as `leadingTrivia`.
   - grammar-defined skip patterns use the skip definition name, such as
     `WHITESPACE` or `LINE_COMMENT`
   - default whitespace skipping uses synthetic trivia type `WHITESPACE`

3. Any final trailing trivia at end-of-file is attached to the `EOF` token's
   `leadingTrivia`.

When `preserveSourceInfo` is disabled:

- no extra fields are attached
- skip patterns remain fully discarded
- existing callers see no behavior change

### Parser API Additions

```typescript
export interface ASTNode {
  readonly ruleName: string;
  readonly children: ReadonlyArray<ASTNode | Token>;
  readonly startLine?: number;
  readonly startColumn?: number;
  readonly endLine?: number;
  readonly endColumn?: number;

  // Present only when preserveSourceInfo is enabled and source-aware tokens
  // were provided to the parser.
  readonly startOffset?: number;
  readonly endOffset?: number;
  readonly firstTokenIndex?: number;
  readonly lastTokenIndex?: number;
  readonly leadingTrivia?: readonly Trivia[];
}

export interface GrammarParserOptions {
  readonly trace?: boolean;
  readonly preserveSourceInfo?: boolean;
}
```

### Parser Semantics

When `preserveSourceInfo` is enabled and the parser receives tokens that carry
rich source metadata:

- every AST node computes `startOffset` and `endOffset` from its first and last
  leaf tokens
- every AST node computes `firstTokenIndex` and `lastTokenIndex` from those
  same leaf tokens
- every AST node exposes `leadingTrivia` from its first leaf token

Nodes with no leaf tokens keep these fields undefined, just as empty nodes
already leave coarse position fields undefined.

### Formatter Payoff

This extension does **not** attempt language-specific comment attachment.
Instead, it preserves the raw information formatters need so that language
packages can later decide whether a given trivia item is:

- leading
- trailing
- dangling
- blank-line separation

That keeps the generic lexer and parser generic while still making them useful
for `AST + trivia -> Doc` pipelines.

### Backward Compatibility

This extension is fully backward-compatible:

- all new fields are optional
- the default mode does not preserve trivia
- existing grammars and callers continue to work unchanged

---

## Implementation Scope

These extensions are implemented in:
1. **TypeScript** (source of truth in `code/src/typescript/`)
2. **All 7 other languages** with generic lexer/parser: Elixir, Python, Ruby, Go,
   Rust, Lua, Perl
3. **Swift** (new foundational packages built with all extensions from the start)

## Backward Compatibility

All extensions are additive:
- New Token `flags` field is optional (undefined by default)
- New ASTNode position fields are optional
- New grammar syntax (`&`, `!`, `+`, `//`) only activates when used
- New `.tokens` section (`context_keywords:`) is optional
- Existing grammars produce identical results
