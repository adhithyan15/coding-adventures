# 07 — Parser

## Overview

The parser takes a flat stream of tokens from the lexer and builds an Abstract Syntax Tree (AST) — a tree structure that represents the grammatical structure of the source code. The parser enforces the language's grammar rules: it knows that `1 + 2` is valid but `+ 1 2` is not.

The long-term goal is to make this parser grammar-driven: provide a grammar definition (like BNF or PEG), and the parser generates itself — like a simplified ANTLR or Bison.

This is Layer 7 of the computing stack. It depends on the lexer package.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → [YOU ARE HERE] → Compiler → VM
```

**Input from:** Lexer (provides the token stream).
**Output to:** Bytecode compiler (walks the AST to generate bytecode).

## Concepts

### Abstract Syntax Tree (AST)

The AST strips away syntactic sugar (parentheses, commas, semicolons) and represents only the meaningful structure:

```python
# Source: x = 1 + 2 * 3

# AST:
Assignment
├── target: Name("x")
└── value: BinaryOp
    ├── op: Add
    ├── left: Number(1)
    └── right: BinaryOp
        ├── op: Multiply
        ├── left: Number(2)
        └── right: Number(3)
```

Note: `2 * 3` is deeper in the tree than `1 +`, which means multiplication is evaluated first. The tree structure *encodes* operator precedence.

### Recursive Descent Parsing

The simplest parsing technique. Each grammar rule becomes a function:

```
expression  → term ((PLUS | MINUS) term)*
term        → factor ((STAR | SLASH) factor)*
factor      → NUMBER | NAME | LPAREN expression RPAREN
```

Maps directly to:

```python
def parse_expression(self):
    left = self.parse_term()
    while self.current_token.type in (PLUS, MINUS):
        op = self.consume()
        right = self.parse_term()
        left = BinaryOp(left, op, right)
    return left
```

### Operator Precedence

Precedence is encoded by the grammar rule nesting depth:
1. Deepest rules = highest precedence (evaluated first)
2. `factor` (numbers, names, parentheses) — highest
3. `term` (multiplication, division) — medium
4. `expression` (addition, subtraction) — lowest

### Grammar (MVP)

```
program     → statement*
statement   → assignment | expression_stmt | if_stmt | while_stmt | print_stmt
assignment  → NAME EQUALS expression NEWLINE
expression  → term ((PLUS | MINUS) term)*
term        → factor ((STAR | SLASH) factor)*
factor      → NUMBER | NAME | LPAREN expression RPAREN
if_stmt     → IF expression COLON NEWLINE statement*
while_stmt  → WHILE expression COLON NEWLINE statement*
print_stmt  → PRINT LPAREN expression RPAREN NEWLINE
```

## Public API

```python
# AST node types
@dataclass
class Number:
    value: int

@dataclass
class Name:
    id: str

@dataclass
class BinaryOp:
    left: Expression
    op: str          # "+", "-", "*", "/"
    right: Expression

@dataclass
class Assignment:
    target: Name
    value: Expression

@dataclass
class Print:
    value: Expression

@dataclass
class If:
    condition: Expression
    body: list[Statement]

@dataclass
class While:
    condition: Expression
    body: list[Statement]

@dataclass
class Program:
    statements: list[Statement]

# Type aliases
Expression = Number | Name | BinaryOp
Statement = Assignment | Print | If | While | ExpressionStatement

# Parser
class Parser:
    def __init__(self, tokens: list[Token]) -> None: ...

    def parse(self) -> Program: ...
        # Parse the token stream into an AST

    @property
    def errors(self) -> list[ParseError]: ...

@dataclass
class ParseError:
    message: str
    token: Token    # The token where the error was detected
```

## Data Flow

```
Input:  List of Token objects (from the lexer)
Output: AST (Program node) + list of errors
```

## Test Strategy

- Parse single number: `42` → Number(42)
- Parse single name: `x` → Name("x")
- Parse binary operation: `1 + 2` → BinaryOp(Number(1), "+", Number(2))
- Parse operator precedence: `1 + 2 * 3` → BinaryOp(Number(1), "+", BinaryOp(Number(2), "*", Number(3)))
- Parse parentheses override: `(1 + 2) * 3` → BinaryOp(BinaryOp(...), "*", Number(3))
- Parse assignment: `x = 1 + 2` → Assignment(Name("x"), BinaryOp(...))
- Parse if statement with body
- Parse while loop with body
- Parse print statement
- Verify error messages for: missing operand, unclosed parenthesis, unexpected token
- End-to-end with lexer: source string → tokens → AST

## Token-adjacency predicates (grammar-driven parser)

The grammar-driven `GrammarParser` supports zero-width elements that test the
gap before the next token, using the lexer's token flags (see 02-lexer):
`@space_before`, `@no_space_before`, `@newline_before`, `@no_newline_before`,
`@space_after_next`, `@no_space_after_next`.

```text
return_statement = "return" [ @no_newline_before expression ] SEMICOLON ;
command_arg      = @space_before MINUS @no_space_before primary ;   # Ruby `f -1`
```

They consume nothing, add no AST children, and record no expectation for
error messages. They work inside alternation, repetition, optional groups and
`&`/`!` lookahead, and they keep packrat memoization exact because they read
only the token stream at the cursor. Using `@newline_before` does not make a
grammar newline-significant.

Because a predicate (like `[ x ]` or a lookahead) can succeed without
consuming, every repetition (`{ }`, `{ }+`, `{ x // sep }`) stops after an
iteration that matched nothing, so the engine terminates on any grammar,
validated or not. Full semantics:
[F11](F11-grammar-token-adjacency-predicates.md).

## Bounded recursion, start rules and compiled grammars (Go engine)

The packrat engine recurses once per rule level, so deeply nested input could
exhaust the stack. In Go that is a fatal, unrecoverable process crash, which
makes unbounded recursion a denial of service for every consumer. The Go
`GrammarParser` therefore counts rule-recursion depth and refuses to exceed
`Options.MaxDepth` (default `DefaultMaxDepth` = 10,000 rule levels, ceiling
`HardMaxDepth` = 50,000), returning an error that satisfies
`errors.Is(err, ErrMaxDepth)`. The refusal is sticky: it is reported even if
an enclosing optional or repetition would have backtracked past the failed rule
and "succeeded" on a prefix. Other implementations should do the same and keep
a hostile-nesting test that asserts the error category.

Each memoised result records the height of its subtree; reusing or growing it
(left recursion) is refused if depth plus height would pass `MaxDepth`, so a
successful parse returns a tree of at most `MaxDepth` levels; matcher
recursion through nested groups, optionals and lookaheads is charged to a
global frame budget (`HardMaxFrames`) so `HardMaxDepth` really bounds the
stack. Tree walkers (`WalkAST`, `FindNodes`, `CollectTokens`) are iterative.
`Options.MaxTokens` and `Options.MaxMemoSlots` optionally cap memory. At the
default limit the Go wrappers refuse JSON arrays at about 5,000 nested levels,
Python parentheses at about 3,300 and TypeScript call arguments at about 500.

The Go engine builds the tree during a parse in pooled flat `int32` arrays
(token indexes and node indexes) and materialises `*ASTNode`s only for what is
reachable from the root, once the parse has succeeded. This is invisible
except for properties that are kept on purpose: a packrat memo hit yields a
distinct node that shares the first parse's `Children` slice and has zero
`StartLine`/`EndLine` (positions are computed only for rules that actually
parsed); `Children` is never nil; and each node's `Children` is
capacity-clipped, so appending to one does not overwrite a sibling.

**Compact API (Go).** `(*Compiled).ParseInto(*Tree, tokens, Options) error`
runs the same engine as `Parse` but leaves the flat result in a reusable
`Tree` instead of building `*ASTNode`s; with one `Tree` reused, a parse
allocates nothing in steady state. `Tree` exposes `Root`, `Rule`/`RuleID`,
`NumChildren`, `Child(n, i)` (a `Child` that is a token index or a `NodeID`),
`Token(i)`, `Span` (first and last leaf token index) and `Position` (the
`ASTNode` position fields), and `ToAST()` rebuilds exactly the tree `Parse`
returns. The memo-hit rule is the same: a replayed result is a distinct
`NodeID` (`IsReplay`) sharing the replayed node's children, its `Position` is
zero, and its `Span` is the real token range. Errors, limits and refusals are
`Parse`'s, and on any error the `Tree` is empty (never partial). A `Tree`
owns its arrays, keeps a reference to (does not copy) the caller's tokens,
retains at most `maxPooledFlat` entries per buffer between parses, and is not
safe for concurrent use (distinct Trees are independent). `PostParse` hooks
transform `*ASTNode`s and are refused by `ParseInto`.

Also in the Go engine: `Options.StartRule` parses from any named rule (an
unknown name is an error); `CompileGrammar` builds an immutable, goroutine-safe
`Compiled` once so many inputs can be parsed without redoing per-grammar work;
reading past the end of the token list yields a synthetic EOF rather than
repeating the last token (so `p = A B EOF` accepts `[A, B]`); an EOF token in
the middle of the list ends the parse and consumers that cannot allow that
must reject it themselves.

**No error recovery.** The grammar-driven parser has none and must not gain
any silently: a parse returns a complete tree for the whole input or an error,
never a partial tree, so consumers can fail closed. (The "Error recovery"
future extension below is a separate, opt-in feature for the hand-written
parser and would not change this guarantee.)

## Future Extensions

- **Grammar-driven parser generator**: Define grammar in BNF/PEG, generate the parser
- **Functions**: def, call, return, arguments
- **Comparison operators**: ==, !=, <, >, <=, >=
- **Boolean operators**: and, or, not
- **Error recovery**: Skip to next statement on error, continue parsing
- **Pretty printer**: AST → formatted source code
