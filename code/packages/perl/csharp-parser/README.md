# CodingAdventures::CSharpParser (Perl)

A hand-written recursive-descent C# parser for the coding-adventures monorepo. It tokenizes C# source with `CodingAdventures::CSharpLexer` and builds an Abstract Syntax Tree (AST) of `CodingAdventures::CSharpParser::ASTNode` nodes.

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

## Supported C# constructs

- Variable declarations: `int x = 5;`  `string y = "hello";`
- Assignments: `x = 10;`
- Expression statements: `42;`  `f(x);`
- If/else statements: `if (x > 0) { … } else { … }`
- For loops: `for (int i = 0; i < 10; i = i + 1) { … }`
- Foreach loops: `foreach (string item in list) { … }` ← C# only
- Return statements: `return x + 1;`
- Method calls: `f(a, b)`
- Binary expressions with correct precedence
- Unary expressions: `-x`  `!flag`
- Null-coalescing: `a ?? b` ← C# only (since 2.0)

## Operator precedence

| Level | Operators |
|-------|-----------|
| null-coalescing | `??` |
| equality | `==` `!=` |
| comparison | `<` `>` `<=` `>=` |
| additive | `+` `-` |
| multiplicative | `*` `/` |
| unary | `!` `-` |
| primary | literals, names, calls, `(expr)` |

## C#-exclusive features

### Null-coalescing operator `??`

Available since C# 2.0. Returns the left operand if it is non-null; otherwise
evaluates and returns the right operand.

```perl
my $ast = CodingAdventures::CSharpParser->parse_csharp('a ?? b;', '2.0');
# AST contains a null_coalesce node
```

### Foreach loop

C# provides a dedicated `foreach` keyword (Java uses an enhanced `for`).

```perl
my $ast = CodingAdventures::CSharpParser->parse_csharp(
    'foreach (string item in list) { }');
# AST contains a foreach_stmt node
```

## How it fits in the stack

```
CodingAdventures::CSharpParser  ← this package (hand-written)
              ↓ uses
CodingAdventures::CSharpLexer   → tokenizes C#
              ↓ uses
CodingAdventures::GrammarTools / Lexer
```

## Usage

```perl
use CodingAdventures::CSharpParser;

# Object-oriented
my $parser = CodingAdventures::CSharpParser->new("int x = 5;");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::CSharpParser->parse_csharp("int y = x + 1;");

# Version-specific (C# 8.0 for null-coalescing assignment)
my $ast = CodingAdventures::CSharpParser->parse_csharp('a ?? b;', '8.0');
```

## AST Node Format

Each node is a `CodingAdventures::CSharpParser::ASTNode`:

```perl
$node->rule_name   # e.g. "var_declaration", "if_stmt", "null_coalesce"
$node->children    # arrayref of child ASTNode objects
$node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
$node->token       # token hashref (leaf only): {type, value, line, col}
```

## Version

0.01
