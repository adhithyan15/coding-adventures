# CodingAdventures::RubyParser

A hand-written recursive-descent Ruby parser for the coding-adventures monorepo. It takes Ruby source text, tokenizes it with `CodingAdventures::RubyLexer`, and builds an Abstract Syntax Tree (AST) of `CodingAdventures::RubyParser::ASTNode` nodes.

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

Given a method definition:

```ruby
def greet(name)
  puts name
end
```

The parser produces:

```
program
└── statement
    └── method_def
        ├── token(DEF "def")
        ├── token(NAME "greet")
        ├── token(LPAREN "(")
        ├── param_list → token(NAME "name")
        ├── token(RPAREN ")")
        ├── body
        │   └── statement → method_call_stmt → token(PUTS) + expression
        └── token(END "end")
```

## Supported Ruby constructs

- **Assignments**: `x = 5`  `name = "Alice"`
- **Method definitions**: `def greet(name) ... end`
- **Class definitions**: `class Dog ... end`
- **If/elsif/else**: `if x > 0 ... elsif x == 0 ... else ... end`
- **Unless**: `unless condition ... end`
- **While/until loops**: `while x > 0 ... end`  `until x == 0 ... end`
- **Return statements**: `return value`  `return 1 + 2`
- **Method calls with parens**: `puts("hello")`  `foo(a, b)`
- **Method calls without parens**: `puts "hello"`
- **Expressions**: arithmetic with correct precedence, equality (`==`, `!=`), comparison (`<`, `>`, `<=`, `>=`), unary minus

## How Ruby differs from Python

Ruby uses `end` keywords to close blocks (not indentation):

```ruby
def double(x)   # open
  return x + x
end             # close
```

This makes Ruby blocks straightforward to parse iteratively — we just keep
consuming statements until we see `end`, `else`, or `elsif`.

## How it fits in the stack

```
CodingAdventures::RubyParser  ← this package
             ↓
CodingAdventures::RubyLexer
             ↓
CodingAdventures::GrammarTools (parse_token_grammar)
```

## Usage

```perl
use CodingAdventures::RubyParser;

# Object-oriented
my $parser = CodingAdventures::RubyParser->new("x = 5");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::RubyParser->parse_ruby("x = 1 + 2 * 3");
```

## AST Node format

```perl
$node->rule_name   # "method_def", "if_stmt", "binary_expr", etc.
$node->children    # arrayref of child nodes
$node->is_leaf     # 1 for token leaves, 0 for inner nodes
$node->token       # token hashref (leaf only): {type, value, line, col}
```

## AST node types

| `rule_name`        | Description                                |
|--------------------|--------------------------------------------|
| `program`          | Root node, contains statements             |
| `statement`        | Wrapper for one statement                  |
| `assignment`       | `NAME = expression`                        |
| `method_def`       | `def NAME(params) body end`                |
| `class_def`        | `class NAME body end`                      |
| `if_stmt`          | `if expr body [elsif ...] [else body] end` |
| `unless_stmt`      | `unless expr body end`                     |
| `while_stmt`       | `while expr body end`                      |
| `until_stmt`       | `until expr body end`                      |
| `return_stmt`      | `return [expression]`                      |
| `method_call_stmt` | `puts expr` (keyword call without parens)  |
| `expression_stmt`  | Stand-alone expression                     |
| `body`             | Statements until `end`/`else`/`elsif`      |
| `expression`       | Entry point for expression parsing         |
| `binary_expr`      | `left op right`                            |
| `unary_expr`       | `-expr`                                    |
| `call_expr`        | `NAME(args)` or `KEYWORD(args)`            |
| `primary`          | Literal, identifier, or grouped expr       |
| `param_list`       | Comma-separated parameter names            |
| `arg_list`         | Comma-separated argument expressions       |
| `token`            | Leaf node wrapping a single token          |

## Version

0.01
