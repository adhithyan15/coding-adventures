# @coding-adventures/algol-parser

Parses ALGOL 60 source text into abstract syntax trees (ASTs) using the grammar-driven parser infrastructure.

## What Is ALGOL 60?

ALGOL 60 (ALGOrithmic Language, 1960) was the first programming language with a formally specified grammar, using what became known as BNF (Backus-Naur Form). Every modern language's syntax specification descends from this work. ALGOL introduced:

- **Block structure** — `begin...end` creates lexical scopes (the foundation of every modern language)
- **BNF grammar** — the formal way to specify language syntax, still used today
- **Recursive descent parsing** — the grammar maps directly to recursive functions
- **Call by name** — re-evaluating arguments on each use (influencing lazy evaluation in Haskell)
- **Dynamic arrays** — bounds determined at runtime (rediscovered in C99 as VLAs)
- **Dangling else resolution** — solved at the grammar level, not by convention

## How It Fits in the Stack

```
algol.grammar (grammar definition)
       |
       v
grammar-tools (parseParserGrammar)
       |
       v
parser (GrammarParser)             <-- generic engine, language-agnostic
       |
       v
algol-lexer (tokenizeAlgol)        <-- produces token stream
       |
       v
algol-parser (parseAlgol)          <-- this package
```

This package is a thin wrapper. It:
1. Tokenizes the source with `tokenizeAlgol` from `@coding-adventures/algol-lexer`
2. Reads `algol.grammar` and parses it with `parseParserGrammar`
3. Runs the `GrammarParser` on the token stream to produce an AST

## Installation

```bash
npm install @coding-adventures/algol-parser
```

## Usage

```typescript
import { parseAlgol } from "@coding-adventures/algol-parser";

// Minimal program
const ast = parseAlgol("begin integer x; x := 42 end");
console.log(ast.ruleName); // "program"

// With conditionals
const ast2 = parseAlgol(`
  begin
    integer x;
    integer y;
    x := 5;
    y := 0;
    if x > 0 then y := 1 else y := 0
  end
`);

// With for loop
const ast3 = parseAlgol(`
  begin
    integer i;
    integer sum;
    sum := 0;
    for i := 1 step 1 until 10 do
      sum := sum + i
  end
`);
```

## AST Structure

The AST is a tree of `ASTNode` objects, each with:
- `ruleName` — the grammar rule that matched (e.g., `"program"`, `"block"`, `"assign_stmt"`)
- `children` — child `ASTNode` objects and/or leaf `Token` objects

Example: parsing `begin integer x; x := 42 end` produces roughly:

```
ASTNode("program")
  ASTNode("block")
    Token(begin, "begin")
    ASTNode("declaration")
      ASTNode("type_decl")
        ASTNode("type")
          Token(integer, "integer")
        ASTNode("ident_list")
          Token(IDENT, "x")
    Token(SEMICOLON, ";")
    ASTNode("statement")
      ASTNode("unlabeled_stmt")
        ASTNode("assign_stmt")
          ASTNode("left_part")
            ASTNode("variable") [Token(IDENT, "x")]
            Token(ASSIGN, ":=")
          ASTNode("expression")
            ... Token(INTEGER_LIT, "42") ...
    Token(end, "end")
```

## Grammar Highlights

**Block structure:**
```algol
begin
  integer x, y;   { declarations come first }
  x := 1;         { then statements }
  y := x + 1
end
```

**Assignment uses `:=`; equality uses `=`:**
```algol
x := 5;           { assignment }
if x = 5 then ... { equality test }
```

**For loop with step/until:**
```algol
for i := 1 step 1 until 10 do
  sum := sum + i
```

**If/then/else:**
```algol
if x > 0 then
  y := 1
else
  y := 0
```

**Boolean operators are words:**
```algol
if x > 0 and x < 10 then ...
if x < 0 or x > 100 then ...
if not flag then ...
```

**Comments:**
```algol
comment this is ignored up to the semicolon;
begin
  ...
end
```

## Key Grammar Design Decisions

**Dangling else resolution:** The `then` branch is `unlabeled_stmt` (which excludes `cond_stmt`). To nest conditionals in a then-branch, you must wrap them in `begin...end`. This resolves the "dangling else" ambiguity at the grammar level — no convention needed.

**Left-associative exponentiation:** Per the ALGOL 60 report, `2^3^4 = (2^3)^4 = 4096`. Most modern languages use right-associativity for exponentiation; ALGOL does not.

**Boolean operator precedence (low to high):** `eqv`, `impl`, `or`, `and`, `not`.
