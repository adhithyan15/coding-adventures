# ALGOL 60 Parser

A Ruby gem that parses ALGOL 60 source text into Abstract Syntax Trees using the grammar-driven parser engine.

## Overview

This gem is a thin wrapper around `coding_adventures_parser`'s `GrammarDrivenParser`. Instead of writing an ALGOL-specific parser from scratch, it loads the `algol.grammar` grammar file and feeds it to the general-purpose parser engine.

The pipeline is:

```
ALGOL 60 source text
       |
       v
algol_lexer (tokenizes using algol.tokens)
       |
       v
grammar_tools (parses algol.grammar into ParserGrammar)
       |
       v
parser (GrammarDrivenParser produces AST)
       |
       v
algol_parser (this gem -- thin wrapper providing ALGOL 60 API)
```

## How It Fits in the Stack

```
algol.grammar (grammar file)
       |
       v
grammar_tools (parses .grammar into ParserGrammar)
       |
       v
parser (GrammarDrivenParser uses ParserGrammar to build AST)
       |
       v
algol_parser (this gem -- thin wrapper providing ALGOL 60 API)
```

## Usage

```ruby
require "coding_adventures_algol_parser"

source = <<~ALGOL
  begin
    integer n;
    integer fact;
    n := 5;
    fact := 1;
    for n := 1 step 1 until 5 do
      fact := fact * n
  end
ALGOL

ast = CodingAdventures::AlgolParser.parse(source)
# => ASTNode(rule_name: "program", children: [...])

puts ast.rule_name   # "program"
```

## Grammar Structure

The root node is always `"program"`. The grammar (algol.grammar) covers:

### Top-level
```
program = block
block   = BEGIN { declaration ; } statement { ; statement } END
```

### Declarations
| Rule | Description |
|------|-------------|
| `type_decl` | `integer x, y` — typed variable declaration |
| `array_decl` | `integer array A[1:10]` — dynamically-sized array |
| `switch_decl` | `switch s := label1, label2` — computed goto table |
| `procedure_decl` | `procedure p(x); integer x; begin ... end` |

### Statements
| Rule | Description |
|------|-------------|
| `assign_stmt` | `x := expression` |
| `cond_stmt` | `if bool then stmt [else stmt]` |
| `for_stmt` | `for i := 1 step 1 until 10 do stmt` |
| `goto_stmt` | `goto label` |
| `proc_stmt` | `write(x)` — procedure call as statement |
| `compound_stmt` | `begin stmt ; stmt end` — statement grouping |

### Expressions
Arithmetic precedence (highest to lowest):
1. `**` / `^` — exponentiation (LEFT-associative per ALGOL 60 report)
2. `*` `/` `div` `mod`
3. `+` `-` (including unary)

Boolean precedence (highest to lowest):
1. `not` (unary negation)
2. `and`
3. `or`
4. `impl` (implication: `a impl b` = `not a or b`)
5. `eqv` (equivalence: `a eqv b` = `a iff b`)

## Key Design Decisions

### Dangling Else Resolution
ALGOL 60 eliminates the dangling-else ambiguity at the grammar level:
```algol
if a then if b then c else d
```
In C, the `else` binds to the nearest `if` by convention. ALGOL makes this a grammar error — the `then`-branch must be `unlabeled_stmt` (which excludes conditionals). To nest conditionals, you must write:
```algol
if a then begin if b then c else d end
```

### Left-Associative Exponentiation
Per the ALGOL 60 report, `2**3**4` = `(2**3)**4` = `4096`, not `2**(3**4)` = `2417851639...`. This differs from most modern languages and mathematical convention. The grammar captures this with repetition rather than right-recursion.

### Call-By-Name Default
Procedure parameters not listed in a `value` declaration are passed by name. The argument expression is re-evaluated every time the parameter is used. The `value_part` and `spec_part` rules in procedure declarations capture this distinction.

## Dependencies

- `coding_adventures_grammar_tools` — reads the `.grammar` file
- `coding_adventures_parser` — the grammar-driven parser engine
- `coding_adventures_algol_lexer` — tokenizes ALGOL 60 source text

## Development

```bash
bundle install
bundle exec rake test
```
