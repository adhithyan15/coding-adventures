# macsyma-grammar-extensions — Control Flow and Block Syntax

> **Status**: New spec. Extends `code/grammars/macsyma/macsyma.grammar`,
> `macsyma-parser`, and `macsyma-compiler` to support the
> control-flow forms missing from the current grammar.
> Parent: `symbolic-computation.md`.

## Why this spec exists

The current MACSYMA grammar covers expressions, comparisons, logical
operators, function calls, lists, and assignment. It does NOT cover
the control-flow forms a real Maxima session uses: `if/then/else`,
`for`, `while`, and `block`. Without these you cannot write a
non-trivial MACSYMA program — and you cannot port any of Maxima's
own library code.

This spec is the only entry that does NOT create a new package. It
modifies three existing packages.

## Scope

In — grammar additions:

```
statement      = (if_expr | for_expr | while_expr | block_expr | expression)
                 ( SEMI | DOLLAR ) ;

if_expr        = "if" expression "then" expression
                 { "elseif" expression "then" expression }
                 [ "else" expression ] ;

for_expr       = "for" NAME [ ":" expression ] [ "step" expression ]
                 ( "thru" | "while" | "unless" ) expression
                 "do" expression
               | "for" NAME "in" expression "do" expression ;

while_expr     = "while" expression "do" expression ;

block_expr     = "block" "(" [ list ] "," statement_list ")"
               | "(" statement_list ")" ;

statement_list = expression { "," expression } ;
```

In — keyword additions to the lexer: `if`, `then`, `elseif`, `else`,
`for`, `step`, `thru`, `in`, `do`, `while`, `unless`, `block`, `return`.

In — compiler mappings:

| AST                                    | IR                                      |
|----------------------------------------|-----------------------------------------|
| `if c then a else b`                   | `If(c, a, b)`                           |
| `for x: a step s thru b do body`       | `ForRange(x, a, s, b, body)`            |
| `for x in list do body`                | `ForEach(x, list, body)`                |
| `while c do body`                      | `While(c, body)`                        |
| `block([locals], s1, s2, ..., sN)`     | `Block(List(locals), s1, ..., sN)`      |
| `return(expr)` inside block            | `Return(expr)`                          |

Out:

- The `Block`/`While`/`For`/`Return` heads themselves — handler
  semantics live in `symbolic-vm` (basic) and in `macsyma-runtime`
  (MACSYMA-specific behavior such as `return` semantics in blocks).

## Heads added

| Head        | Arity | Meaning                                  |
|-------------|-------|------------------------------------------|
| `ForRange`  | 5     | `for x: a step s thru b do body`         |
| `ForEach`   | 3     | `for x in list do body`                  |
| `While`     | 2     | `while c do body`                        |
| `Block`     | 2+    | Local scope + statement sequence.        |
| `Return`    | 1     | Early exit from a block / for / while.   |

## Test strategy

- Grammar parses each new form.
- Compiler produces the expected IR shape.
- VM evaluates `if 1 < 2 then 3 else 4` → `3`.
- VM evaluates `for i: 1 thru 5 do i^2` returning the last value
  (`25`); `block([s: 0], for i: 1 thru 5 do s: s + i, s)` → `15`.
- `block([x], x: 5, x + 1)` → `6` and does not leak `x` outward.
- Coverage: hits each new grammar production at least once.

## Files modified

- `code/grammars/macsyma/macsyma.grammar` — production additions above.
- `code/grammars/macsyma/macsyma.tokens` — add new keywords.
- `code/packages/python/macsyma-lexer` — recognize new keywords.
- `code/packages/python/macsyma-parser` — exercise new productions.
- `code/packages/python/macsyma-compiler` — new AST → IR mappings.
- `code/packages/python/symbolic-vm` — `If` already exists; add
  `While`, `ForRange`, `ForEach`, `Block`, `Return` handlers.

## Phasing

- Phase G (after the rest of the substrate is in place). The basic
  REPL works without these — they're for porting library code and
  writing programs of more than one statement.
