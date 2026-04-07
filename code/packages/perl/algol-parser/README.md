# CodingAdventures::AlgolParser (Perl)

A hand-written recursive-descent ALGOL 60 parser. Tokenizes source text using `CodingAdventures::AlgolLexer` and constructs an Abstract Syntax Tree (AST) following the grammar in `algol.grammar`.

## What it does

Given `begin integer x; x := 42 end`, produces an AST rooted at `program`:

```
program
└── block
    ├── Token(BEGIN, begin)
    ├── type_decl
    │   ├── Token(INTEGER, integer)
    │   └── ident_list
    │       └── Token(IDENT, x)
    ├── Token(SEMICOLON, ;)
    ├── statement
    │   └── unlabeled_stmt
    │       └── assign_stmt
    │           ├── left_part
    │           │   ├── variable → Token(IDENT, x)
    │           │   └── Token(ASSIGN, :=)
    │           └── expression
    │               └── arith_expr → ... → Token(INTEGER_LIT, 42)
    └── Token(END, end)
```

## ALGOL 60 Grammar coverage

### Declarations

| Declaration   | Example                              |
|---------------|--------------------------------------|
| type_decl     | `integer x, y`                       |
| array_decl    | `array A[1:10]`, `integer array B[1:n]` |
| switch_decl   | `switch s := L1, L2`                 |
| procedure_decl| `real procedure f(x); value x; ...`  |

### Statements

| Statement    | Example                                    |
|--------------|--------------------------------------------|
| assign_stmt  | `x := 42`, `x := y := 0`                  |
| cond_stmt    | `if x = 0 then x := 1 else x := 2`        |
| for_stmt     | `for i := 1 step 1 until 10 do ...`       |
| goto_stmt    | `goto label`                               |
| proc_stmt    | `print(x)`, `init`                         |
| block        | `begin ... end`                            |

### Expressions

| Expression   | Example                   |
|--------------|---------------------------|
| arith_expr   | `x + y * z`, `a ** 2`     |
| bool_expr    | `x < 10 and y > 0`        |
| relation     | `x = 0`, `a != b`, `i <= n` |

## Usage

```perl
use CodingAdventures::AlgolParser;

my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := 42 end');
print $ast->rule_name;    # "program"

# Walk the tree recursively
sub walk {
    my ($node, $depth) = @_;
    my $indent = '  ' x $depth;
    if ($node->is_leaf) {
        printf "%sToken(%s, %s)\n", $indent, $node->token->{type}, $node->token->{value};
    } else {
        printf "%s%s\n", $indent, $node->rule_name;
        walk($_, $depth + 1) for @{ $node->children };
    }
}
walk($ast, 0);
```

## AST node structure

Internal nodes:
```
{ rule_name => "block", children => [...], is_leaf => 0 }
```

Leaf nodes:
```
{ rule_name => "token", children => [], is_leaf => 1,
  token => { type => "IDENT", value => "x", line => 1, col => 5 } }
```

## Design notes

- **Dangling else**: resolved at the grammar level. The `then`-branch is `unlabeled_stmt`
  (which cannot be a conditional), so `else` always belongs to the nearest `if`.
  Use `begin...end` to nest conditionals in the then-branch.

- **Left-associative exponentiation**: `2^3^4 = (2^3)^4 = 4096`, per the ALGOL 60 report.
  This differs from mathematical convention.

- **Case-insensitive keywords**: inherited from `AlgolLexer`. `BEGIN`, `Begin`, and `begin`
  all produce the same token type.

- **Call-by-name default**: ALGOL 60 passes parameters by name unless listed in a
  `value` declaration. This parser represents the syntax; call semantics are for the
  interpreter/evaluator layer.

## How it fits in the stack

```
algol.tokens / algol.grammar  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools (token grammar only)
CodingAdventures::AlgolLexer  (tokenizer)
    ↓  feeds
CodingAdventures::AlgolParser  ← you are here
    ↓  produces
AST (program → block → declarations + statements)
```

## Dependencies

- `CodingAdventures::AlgolLexer` — tokenizes ALGOL 60 source
- `CodingAdventures::GrammarTools` — transitive (used by lexer)

## Running tests

```bash
prove -l -v t/
```
