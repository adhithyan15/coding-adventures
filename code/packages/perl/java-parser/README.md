# CodingAdventures::JavaParser

A hand-written recursive-descent Java parser for the coding-adventures monorepo. It tokenizes Java source with `CodingAdventures::JavaLexer` and builds an Abstract Syntax Tree (AST) of `CodingAdventures::JavaParser::ASTNode` nodes.

## What it does

Given input `int x = 5;`, the parser produces:

```
program
└── statement
    └── var_declaration
        ├── NAME "int"
        ├── NAME "x"
        ├── EQUALS "="
        ├── expression
        │   └── primary → NUMBER "5"
        └── SEMICOLON ";"
```

## Supported Java constructs

- Variable declarations: `int x = 5;`  `String y = "hello";`
- Assignments: `x = 10;`
- Expression statements: `42;`  `f(x);`
- If/else statements: `if (x > 0) { … } else { … }`
- For loops: `for (int i = 0; i < 10; i = i + 1) { … }`
- Return statements: `return x + 1;`
- Method calls: `f(a, b)`
- Binary expressions with correct precedence
- Unary expressions: `-x`  `!flag`

## Operator precedence

| Level | Operators |
|-------|-----------|
| equality | `==` `!=` |
| comparison | `<` `>` `<=` `>=` |
| additive | `+` `-` |
| multiplicative | `*` `/` |
| unary | `!` `-` |
| primary | literals, names, calls, `(expr)` |

## How it fits in the stack

```
CodingAdventures::JavaParser  ← this package (hand-written)
              ↓ uses
CodingAdventures::JavaLexer   → tokenizes Java
              ↓ uses
CodingAdventures::GrammarTools / Lexer
```

## Usage

```perl
use CodingAdventures::JavaParser;

# Object-oriented
my $parser = CodingAdventures::JavaParser->new("int x = 5;");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::JavaParser->parse_java("int y = x + 1;");
```

## AST Node Format

Each node is a `CodingAdventures::JavaParser::ASTNode`:

```perl
$node->rule_name   # e.g. "var_declaration", "if_stmt", "binary_expr"
$node->children    # arrayref of child ASTNode objects
$node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
$node->token       # token hashref (leaf only): {type, value, line, col}
```

## Version

0.01
