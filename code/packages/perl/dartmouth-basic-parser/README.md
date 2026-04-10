# CodingAdventures::DartmouthBasicParser (Perl)

A hand-written recursive-descent parser for 1964 Dartmouth BASIC. Accepts
source text and returns an Abstract Syntax Tree (AST).

## What is Dartmouth BASIC?

Dartmouth BASIC was invented by John Kemeny and Thomas Kurtz at Dartmouth
College in 1964 — the world's first widely accessible programming language,
running on the GE-225 time-sharing mainframe. Designed for non-science
students who had never programmed before.

Key features of the 1964 specification:
- Every statement is **line-numbered** (`10 LET X = 5`)
- All source is **uppercase** (teletypes had no lowercase)
- Simple **scalar variables**: single letter (`X`) or letter+digit (`A1`)
- **Arrays** dimensioned with `DIM`; accessed as `A(I)`
- **17 statement types**: LET, PRINT, INPUT, IF/THEN, GOTO, GOSUB, RETURN,
  FOR/NEXT, END, STOP, REM, READ, DATA, RESTORE, DIM, DEF
- **11 built-in math functions**: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
  INT, RND, SGN
- **User-defined functions**: `DEF FNA(X) = ...` through `DEF FNZ(X) = ...`

## Why hand-written (not grammar-driven)?

The Perl `CodingAdventures::GrammarTools` only provides `parse_token_grammar`,
not a grammar-driven parser engine. So this module implements each grammar
rule as a Perl subroutine — the classic recursive-descent approach.

## How it fits in the stack

```
DartmouthBasicParser   ← this module
       ↓ uses
DartmouthBasicLexer    ← tokenizes source text (normalises case, relabels
                          LINE_NUM, suppresses REM content)
```

## Installation

```bash
cpanm --notest .
```

## Usage

```perl
use CodingAdventures::DartmouthBasicParser;

# Parse a complete BASIC program
my $ast = CodingAdventures::DartmouthBasicParser->parse(
    "10 LET X = 5\n20 PRINT X\n30 END\n"
);

print $ast->rule_name;     # "program"
print ref($ast->children); # "ARRAY"

# Walk the AST recursively
sub walk {
    my ($node, $depth) = @_;
    my $indent = '  ' x $depth;
    if ($node->is_leaf) {
        printf "%sToken(%s, '%s')\n",
            $indent, $node->token->{type}, $node->token->{value};
    } else {
        printf "%s%s\n", $indent, $node->rule_name;
        walk($_, $depth + 1) for @{ $node->children };
    }
}
walk($ast, 0);
```

## AST structure

```
program
└── line
    ├── token(LINE_NUM, "10")
    ├── statement
    │   └── let_stmt
    │       ├── token(KEYWORD, "LET")
    │       ├── variable
    │       │   └── token(NAME, "X")
    │       ├── token(EQ, "=")
    │       └── expr → term → power → unary → primary
    │           └── token(NUMBER, "5")
    └── token(NEWLINE, "\n")
```

ASTNode methods:
- `$node->rule_name` — grammar rule that produced this node
- `$node->children`  — arrayref of child ASTNodes
- `$node->is_leaf`   — 1 if wrapping a single token, 0 otherwise
- `$node->token`     — hashref with `type`, `value`, `line`, `col` (leaf only)

## Running tests

```bash
PERL5LIB=../grammar-tools/lib:../dartmouth-basic-lexer/lib prove -l -v t/
```

## Grammar implemented

    program      = { line }
    line         = LINE_NUM [ statement ] NEWLINE
    statement    = let_stmt | print_stmt | input_stmt | if_stmt | goto_stmt
                 | gosub_stmt | return_stmt | for_stmt | next_stmt | end_stmt
                 | stop_stmt | rem_stmt | read_stmt | data_stmt | restore_stmt
                 | dim_stmt | def_stmt
    variable     = NAME LPAREN expr RPAREN | NAME
    expr         = term { (PLUS|MINUS) term }
    term         = power { (STAR|SLASH) power }
    power        = unary [ CARET power ]
    unary        = MINUS primary | primary
    primary      = NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
    relop        = EQ | LT | GT | LE | GE | NE

## Version

0.01
