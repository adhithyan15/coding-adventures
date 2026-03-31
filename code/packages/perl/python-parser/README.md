# CodingAdventures::PythonParser

A hand-written recursive-descent Python parser for the coding-adventures monorepo. It takes Python source text, tokenizes it with `CodingAdventures::PythonLexer`, and builds an Abstract Syntax Tree (AST) of `CodingAdventures::PythonParser::ASTNode` nodes.

## What it does

Given input `x = 5`, the parser produces:

```
program
└── statement
    └── assignment
        ├── token(NAME "x")
        ├── token(EQUALS "=")
        └── expression
            └── primary
                └── token(NUMBER "5")
```

## Supported Python constructs

- **Assignments**: `x = 5`  `name = "Alice"`
- **Function definitions**: `def add(a, b):\n    return a + b`
- **If/elif/else**: `if x == 0:\n    ...\nelif x == 1:\n    ...\nelse:\n    ...`
- **For loops**: `for i in range(10):\n    ...` (iterates using `in`)
- **While loops**: `while x == 0:\n    x = x + 1`
- **Return statements**: `return value`  `return 1 + 2`
- **Import statements**: `import math`  `import os as operating_system`
- **From-import**: `from math import sqrt`
- **Function calls**: `print("hello")`  `range(10)`
- **Expressions**: arithmetic with correct precedence, equality (`==`), unary minus
- **Parenthesized expressions**: `(a + b) * c`

## How it fits in the stack

```
CodingAdventures::PythonParser  ← this package
              ↓
CodingAdventures::PythonLexer
              ↓
CodingAdventures::GrammarTools (parse_token_grammar)
```

## Usage

```perl
use CodingAdventures::PythonParser;

# Object-oriented
my $parser = CodingAdventures::PythonParser->new("x = 5");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::PythonParser->parse_python("x = 1 + 2 * 3");
```

## AST Node format

```perl
$node->rule_name   # "assignment", "if_stmt", "binary_expr", etc.
$node->children    # arrayref of child nodes
$node->is_leaf     # 1 for token leaves, 0 for inner nodes
$node->token       # token hashref (leaf only): {type, value, line, col}
```

## AST node types

| `rule_name`       | Description                           |
|-------------------|---------------------------------------|
| `program`         | Root node, contains statements        |
| `statement`       | Wrapper for one statement             |
| `assignment`      | `NAME = expression`                   |
| `function_def`    | `def NAME(params): block`             |
| `if_stmt`         | `if expr: block [elif ...] [else ...]`|
| `for_stmt`        | `for NAME in expr: block`             |
| `while_stmt`      | `while expr: block`                   |
| `return_stmt`     | `return [expression]`                 |
| `import_stmt`     | `import NAME [as NAME]`               |
| `from_import_stmt`| `from NAME import NAME [as NAME]`     |
| `expression_stmt` | Stand-alone expression                |
| `block`           | INDENT/DEDENT delimited body          |
| `expression`      | Entry point for expression parsing    |
| `binary_expr`     | `left op right`                       |
| `unary_expr`      | `-expr`                               |
| `call_expr`       | `NAME(args)`                          |
| `primary`         | Literal, identifier, or grouped expr  |
| `param_list`      | Comma-separated parameter names       |
| `arg_list`        | Comma-separated argument expressions  |
| `token`           | Leaf node wrapping a single token     |

## Version

0.01
