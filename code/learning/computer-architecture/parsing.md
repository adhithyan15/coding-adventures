# Parsing -- From Token Streams to Abstract Syntax Trees

## What is Parsing?

Parsing is the second stage of a compiler or interpreter pipeline. It takes
a flat list of tokens (produced by the lexer) and builds a tree structure
called an **Abstract Syntax Tree** (AST). The AST captures the *meaning* of
the code by encoding the relationships between tokens.

```
Token stream:   NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF

                                     |
                                     | Parsing
                                     v

AST:            Assignment
                +-- target: Name("x")
                +-- value: BinaryOp("+")
                           +-- left: NumberLiteral(1)
                           +-- right: NumberLiteral(2)
```

Think of it like diagramming a sentence in English class. Given the sentence
"The cat sat on the mat", you identify the subject ("The cat"), the verb
("sat"), and the prepositional phrase ("on the mat"). A parser does the same
thing for source code -- it identifies the assignments, expressions, function
calls, and how they nest together.

### Why not just use the token list directly?

A flat list of tokens has no structure. Consider `1 + 2 * 3`. The tokens are:

```
NUMBER("1")  PLUS("+")  NUMBER("2")  STAR("*")  NUMBER("3")
```

Should the answer be `(1 + 2) * 3 = 9` or `1 + (2 * 3) = 7`? The token
list doesn't tell us. The parser's job is to build a tree that encodes the
correct precedence: multiplication before addition.

---

## Abstract Syntax Trees

An AST is a **tree** data structure where:
- **Leaf nodes** are simple values (numbers, strings, variable names)
- **Interior nodes** are operations that combine their children

### Visualizing an AST

For `x = 1 + 2 * 3`:

```
            Assignment
           /          \
      Name("x")     BinaryOp("+")
                    /            \
             NumberLiteral(1)   BinaryOp("*")
                                /            \
                         NumberLiteral(2)  NumberLiteral(3)
```

The tree structure itself encodes operator precedence. Multiplication
is *deeper* in the tree than addition, which means it gets evaluated
*first*. When you evaluate a tree bottom-up:

```
Step 1:  Evaluate the deepest nodes first
         NumberLiteral(2) = 2,  NumberLiteral(3) = 3

Step 2:  Evaluate BinaryOp("*") on (2, 3)
         2 * 3 = 6

Step 3:  Evaluate BinaryOp("+") on (1, 6)
         1 + 6 = 7

Step 4:  Assign 7 to variable "x"
```

No parentheses needed in the tree -- the structure *is* the precedence.

### AST Node Types in Our Language

Our parser produces these node types (defined in
`code/packages/python/parser/src/lang_parser/parser.py`):

```
NumberLiteral(value: int)
    A numeric literal like 42. Leaf node.
    Example: "42" --> NumberLiteral(value=42)

StringLiteral(value: str)
    A string literal like "hello". Leaf node.
    Example: '"hello"' --> StringLiteral(value="hello")

Name(name: str)
    A variable reference like x. Leaf node.
    Example: "x" --> Name(name="x")

BinaryOp(left: Expression, op: str, right: Expression)
    A binary operation like 1 + 2. Interior node with two children.
    Example: "1 + 2" --> BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))

Assignment(target: Name, value: Expression)
    A variable assignment like x = 1 + 2. Interior node.
    Example: "x = 42" --> Assignment(Name("x"), NumberLiteral(42))

Program(statements: list[Statement])
    The root node. Contains all top-level statements.
    Example: "x = 1\ny = 2" --> Program([Assignment(...), Assignment(...)])
```

---

## Recursive Descent Parsing

Our parser uses **recursive descent** -- the simplest and most intuitive
parsing technique. The core idea:

> Each grammar rule becomes a method. Methods call each other
> following the grammar structure.

### The Grammar

A grammar is a set of rules that describe what valid programs look like.
Our grammar is:

```
program     = statement*
statement   = assignment | expression_stmt
assignment  = NAME EQUALS expression NEWLINE
expression  = term ((PLUS | MINUS) term)*
term        = factor ((STAR | SLASH) factor)*
factor      = NUMBER | STRING | NAME | LPAREN expression RPAREN
```

Reading the grammar:
- `*` means "zero or more"
- `|` means "or" (alternation)
- `UPPERCASE` means "match a token of this type"
- `lowercase` means "parse this other grammar rule"

### Mapping Grammar Rules to Methods

Each rule in the grammar maps directly to a Python method in the `Parser`
class:

```
Grammar Rule                           Python Method
====================================   ==================
program = statement*                   _parse_program()
statement = assignment | expr_stmt     _parse_statement()
assignment = NAME EQUALS expr NL       _parse_assignment()
expression = term ((+|-) term)*        _parse_expression()
term = factor ((*|/) factor)*          _parse_term()
factor = NUMBER | NAME | (expr)        _parse_factor()
```

### How the Parser Methods Work

**_parse_program()** -- Parse the whole program:
```
    Create an empty list of statements
    While not at EOF:
        Call _parse_statement() to get one statement
        Add it to the list
    Return Program(statements)
```

**_parse_statement()** -- Decide what kind of statement:
```
    Peek at current and next token
    If current is NAME and next is EQUALS:
        This is an assignment --> call _parse_assignment()
    Else:
        This is an expression statement --> call _parse_expression()
```

**_parse_expression()** -- Parse addition and subtraction:
```
    left = _parse_term()             # Parse left operand
    While current token is + or -:
        Consume the operator
        right = _parse_term()        # Parse right operand
        left = BinaryOp(left, op, right)
    Return left
```

**_parse_term()** -- Parse multiplication and division:
```
    left = _parse_factor()           # Parse left operand
    While current token is * or /:
        Consume the operator
        right = _parse_factor()      # Parse right operand
        left = BinaryOp(left, op, right)
    Return left
```

**_parse_factor()** -- Parse atomic values:
```
    If current token is NUMBER:
        Consume it, return NumberLiteral(value)
    If current token is STRING:
        Consume it, return StringLiteral(value)
    If current token is NAME:
        Consume it, return Name(name)
    If current token is LPAREN:
        Consume the (
        expr = _parse_expression()    # Recurse!
        Expect and consume )
        Return expr
    Else:
        Raise ParseError
```

The "recursive" part: `_parse_factor()` can call `_parse_expression()` for
parenthesized sub-expressions, which calls `_parse_term()`, which calls
`_parse_factor()` again. The call stack naturally handles any nesting depth.

---

## Operator Precedence Climbing

### The Problem

In math, `1 + 2 * 3` equals 7, not 9. Multiplication has higher precedence
than addition. How does the parser know this?

### The Solution: Grammar Layering

Operator precedence is encoded by the *depth* of grammar rules:

```
    expression  -->  handles + and -     (LOWEST precedence)
    term        -->  handles * and /     (HIGHER precedence)
    factor      -->  handles atoms       (HIGHEST -- numbers, names)
```

Lower-precedence operators are in *higher* (more general) rules. Higher-
precedence operators are in *lower* (more specific) rules. Since the parser
descends from general to specific, higher-precedence operators end up deeper
in the tree -- and deeper means "evaluated first".

### Walk Through: Parsing `x = 1 + 2 * 3`

Let's trace the parser step by step:

```
Tokens: NAME("x") EQUALS("=") NUMBER("1") PLUS("+") NUMBER("2") STAR("*") NUMBER("3") EOF
        ^
        Position 0
```

**Step 1: _parse_statement()**
```
Peek: NAME("x") at pos 0
Next: EQUALS("=") at pos 1
--> It's NAME followed by EQUALS, so this is an assignment.
Call _parse_assignment()
```

**Step 2: _parse_assignment()**
```
Consume NAME("x") --> target = Name("x")
Consume EQUALS("=")
Call _parse_expression() for the right-hand side
```

**Step 3: _parse_expression()**
```
Call _parse_term() to get the left operand
```

**Step 4: _parse_term()** (first call)
```
Call _parse_factor()
```

**Step 5: _parse_factor()** (first call)
```
Current token: NUMBER("1")
Consume it.
Return NumberLiteral(1)
```

**Step 6: Back in _parse_term()**
```
left = NumberLiteral(1)
Current token: PLUS("+") -- NOT * or /, so exit the loop
Return NumberLiteral(1)
```

**Step 7: Back in _parse_expression()**
```
left = NumberLiteral(1)
Current token: PLUS("+") -- YES, this is + or -!
Consume PLUS.
Call _parse_term() for the right operand
```

**Step 8: _parse_term()** (second call)
```
Call _parse_factor()
```

**Step 9: _parse_factor()** (second call)
```
Current token: NUMBER("2")
Consume it.
Return NumberLiteral(2)
```

**Step 10: Back in _parse_term()** (second call)
```
left = NumberLiteral(2)
Current token: STAR("*") -- YES, this is * or /!
Consume STAR.
Call _parse_factor() for the right operand
```

**Step 11: _parse_factor()** (third call)
```
Current token: NUMBER("3")
Consume it.
Return NumberLiteral(3)
```

**Step 12: Back in _parse_term()** (second call)
```
left was NumberLiteral(2), right is NumberLiteral(3)
Build: BinaryOp(NumberLiteral(2), "*", NumberLiteral(3))
No more * or / tokens. Return the BinaryOp.
```

**Step 13: Back in _parse_expression()**
```
left was NumberLiteral(1), right is BinaryOp(2, "*", 3)
Build: BinaryOp(NumberLiteral(1), "+", BinaryOp(2, "*", 3))
No more + or - tokens. Return.
```

**Step 14: Back in _parse_assignment()**
```
Assignment(target=Name("x"), value=BinaryOp(1, "+", BinaryOp(2, "*", 3)))
```

### The Resulting AST

```
            Assignment
           /          \
      Name("x")     BinaryOp("+")
                    /            \
             NumberLiteral(1)   BinaryOp("*")
                                /            \
                         NumberLiteral(2)  NumberLiteral(3)
```

The multiplication is deeper, so it's evaluated first. The parser correctly
encoded `x = 1 + (2 * 3)` without any parentheses in the source.

### Left Associativity

What about `1 + 2 + 3`? Should this be `(1 + 2) + 3` (left-associative)
or `1 + (2 + 3)` (right-associative)? For addition it doesn't matter, but
for subtraction it does: `10 - 5 - 3` should be `(10 - 5) - 3 = 2`, not
`10 - (5 - 3) = 8`.

The `while` loop in `_parse_expression()` naturally produces left-associative
trees:

```
Parsing "1 + 2 + 3":

    Iteration 1: left = 1
    See +, right = 2
    left = BinaryOp(1, "+", 2)

    Iteration 2: left = BinaryOp(1, "+", 2)
    See +, right = 3
    left = BinaryOp(BinaryOp(1, "+", 2), "+", 3)

Result:
          +
         / \
        +   3
       / \
      1   2

This is (1 + 2) + 3 -- left-associative!
```

---

## The .grammar File Format

The `.grammar` file uses EBNF (Extended Backus-Naur Form) notation to define
the syntactic grammar of a language. It is parsed by the `grammar_tools`
package.

**Location:** `code/grammars/` -- contains `python.grammar`, `ruby.grammar`,
`javascript.grammar`, `typescript.grammar`

### Format

```
# Comments (not part of the format, shown for clarity)
rule_name = definition ;

# UPPERCASE names reference tokens from the .tokens file
# lowercase names reference other grammar rules
# |       alternation (or)
# { x }   zero or more repetitions
# [ x ]   optional (zero or one)
# ( x )   grouping
# "lit"   literal token match
```

### Example: Python Grammar

```
program      = { statement } ;
statement    = assignment | expression_stmt ;
assignment   = NAME EQUALS expression ;
expression_stmt = expression ;
expression   = term { ( PLUS | MINUS ) term } ;
term         = factor { ( STAR | SLASH ) factor } ;
factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
```

### EBNF Elements

| Notation  | Name        | Meaning                                    |
|-----------|-------------|--------------------------------------------|
| `A B C`   | Sequence    | Match A, then B, then C (all must succeed) |
| `A \| B`  | Alternation | Try A; if it fails, try B                  |
| `{ A }`   | Repetition  | Match A zero or more times                 |
| `[ A ]`   | Optional    | Match A zero or one time                   |
| `( A )`   | Group       | Parenthesized sub-expression               |
| `"text"`  | Literal     | Match a token with this exact text value    |
| `NAME`    | Token ref   | Match a token with type NAME               |
| `expr`    | Rule ref    | Recursively parse rule "expr"              |

### How it maps to the parser

The `grammar_tools` package parses the `.grammar` file into a tree of
`GrammarElement` objects:

```
GrammarRule("expression",
    Sequence([
        RuleReference("term"),
        Repetition(
            Sequence([
                Group(Alternation([
                    RuleReference("PLUS"),
                    RuleReference("MINUS"),
                ])),
                RuleReference("term"),
            ])
        ),
    ])
)
```

The grammar-driven parser (`GrammarParser`) walks this tree at runtime,
matching each element against the token stream. This is described in more
detail below.

---

## Grammar-Driven Parsing

**Location:** `code/packages/python/parser/src/lang_parser/grammar_parser.py`

The grammar-driven parser takes a `ParserGrammar` (parsed from a `.grammar`
file) and a token list, then interprets the grammar rules at runtime with
backtracking.

### How Each EBNF Element is Interpreted

```
RuleReference (UPPERCASE, e.g., "NUMBER"):
    Match a token of that type. Consume it if it matches.

RuleReference (lowercase, e.g., "expression"):
    Recursively parse that grammar rule.

Sequence (A B C):
    Match A, then B, then C. All must succeed.
    If any fails, backtrack to where the sequence started.

Alternation (A | B | C):
    Try A first. If it fails, restore position and try B.
    If B fails, restore position and try C.
    This is backtracking.

Repetition ({ A }):
    Match A zero or more times. Keep matching until A fails.
    Always succeeds (zero matches is valid).

Optional ([ A ]):
    Try to match A. If it fails, that's fine -- just continue.

Literal ("text"):
    Match a token whose text value is exactly "text".

Group (( A )):
    Just a parenthesized sub-expression. Evaluate A.
```

### Backtracking

When an alternation tries one choice and it fails partway through, the
parser needs to undo any tokens it consumed:

```
Parsing "1 + 2" against rule: statement = assignment | expression_stmt

Try assignment = NAME EQUALS expression:
    Current token: NUMBER("1")
    Expected: NAME
    NUMBER is not NAME --> FAIL
    Restore position to before this attempt

Try expression_stmt = expression:
    Parse expression...
    Success!
```

The parser saves the position before each attempt and restores it on failure.

### Generic AST Nodes

The grammar-driven parser produces generic `ASTNode` objects (rather than
the specific `NumberLiteral`, `BinaryOp`, etc. from the hand-written parser).
Each `ASTNode` has a rule name and a list of children, which can be tokens
or other `ASTNode` objects.

---

## Walk Through: Parsing `x = 1 + 2 * 3` Showing Precedence

Here is the complete parse trace in a compact format, showing how the grammar
rules create the correct tree structure:

```
_parse_program()
  |
  +-- _parse_statement()
  |     |
  |     +-- Peek: NAME("x"), next: EQUALS("=")
  |     |   --> assignment
  |     |
  |     +-- _parse_assignment()
  |           |
  |           +-- Consume NAME("x") --> target = Name("x")
  |           +-- Consume EQUALS("=")
  |           +-- _parse_expression()
  |                 |
  |                 +-- _parse_term()
  |                 |     |
  |                 |     +-- _parse_factor()
  |                 |     |     +-- Consume NUMBER("1")
  |                 |     |     +-- Return NumberLiteral(1)
  |                 |     |
  |                 |     +-- Current: PLUS (not * or /)
  |                 |     +-- Return NumberLiteral(1)
  |                 |
  |                 +-- Current: PLUS (yes, + or -)
  |                 +-- Consume PLUS
  |                 +-- _parse_term()
  |                 |     |
  |                 |     +-- _parse_factor()
  |                 |     |     +-- Consume NUMBER("2")
  |                 |     |     +-- Return NumberLiteral(2)
  |                 |     |
  |                 |     +-- Current: STAR (yes, * or /)
  |                 |     +-- Consume STAR
  |                 |     +-- _parse_factor()
  |                 |     |     +-- Consume NUMBER("3")
  |                 |     |     +-- Return NumberLiteral(3)
  |                 |     |
  |                 |     +-- Build: BinaryOp(2, "*", 3)
  |                 |     +-- No more * or /
  |                 |     +-- Return BinaryOp(2, "*", 3)
  |                 |
  |                 +-- Build: BinaryOp(1, "+", BinaryOp(2, "*", 3))
  |                 +-- No more + or -
  |                 +-- Return BinaryOp(1, "+", BinaryOp(2, "*", 3))
  |
  +-- Final AST:
        Assignment(
            target = Name("x"),
            value  = BinaryOp(
                left  = NumberLiteral(1),
                op    = "+",
                right = BinaryOp(
                    left  = NumberLiteral(2),
                    op    = "*",
                    right = NumberLiteral(3)
                )
            )
        )
```

The critical moment: when `_parse_term()` (second call) sees `*`, it consumes
it and builds a sub-tree. By the time control returns to `_parse_expression()`,
the `2 * 3` is already wrapped in a single `BinaryOp` node. The `+` in
`_parse_expression()` treats it as a single unit -- the right operand.

---

## AST Node Types: A Catalog

Here is a catalog of AST node types, from simplest to most complex. Our
language currently implements the first five; the others are shown to
illustrate what a fuller language would need.

### Leaf Nodes (no children)

```
NumberLiteral(value=42)     -- a number
StringLiteral(value="hi")  -- a string
Name(name="x")             -- a variable reference
BooleanLiteral(value=True) -- a boolean (future)
NilLiteral()               -- null/None/nil (future)
```

### Expression Nodes (produce a value)

```
BinaryOp(left, "+", right)           -- arithmetic: 1 + 2
UnaryOp("-", operand)                -- negation: -x (future)
FunctionCall(name, [args])           -- f(1, 2) (future)
MemberAccess(object, "field")        -- obj.field (future)
IndexAccess(array, index)            -- arr[0] (future)
```

### Statement Nodes (perform an action)

```
Assignment(Name("x"), expr)          -- x = 42
IfStatement(cond, then, else)        -- if/else (future)
WhileStatement(cond, body)           -- while loop (future)
FunctionDef(name, params, body)      -- def f(x): ... (future)
ReturnStatement(expr)                -- return x (future)
```

### The Root Node

```
Program(statements=[...])           -- the entire program
```

---

## References

| File | Description |
|------|-------------|
| `code/packages/python/parser/src/lang_parser/parser.py` | Hand-written recursive descent parser |
| `code/packages/python/parser/src/lang_parser/grammar_parser.py` | Grammar-driven parser with backtracking |
| `code/packages/python/grammar-tools/` | Parses `.tokens` and `.grammar` files |
| `code/grammars/python.grammar` | Python grammar definition (EBNF) |
| `code/grammars/ruby.grammar` | Ruby grammar definition |
| `code/grammars/javascript.grammar` | JavaScript grammar definition |
| `code/grammars/typescript.grammar` | TypeScript grammar definition |
