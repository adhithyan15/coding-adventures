# CodingAdventures::TypescriptParser

Hand-written recursive-descent TypeScript parser for the coding-adventures monorepo.

## What it does

This module parses a subset of TypeScript into an Abstract Syntax Tree (AST)
using the recursive-descent technique. Each grammar rule is a Perl method that
calls sibling methods for sub-rules.

## Usage

```perl
use CodingAdventures::TypescriptParser;

# Object-oriented
my $parser = CodingAdventures::TypescriptParser->new("let x = 5;");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::TypescriptParser->parse_ts("const y = x + 1;");

# Walk the AST
sub walk {
    my ($node) = @_;
    if ($node->is_leaf) {
        print $node->token->{value}, "\n";
    } else {
        print $node->rule_name, "\n";
        walk($_) for @{ $node->children };
    }
}
walk($ast);
```

## Supported constructs

- Variable declarations: `var x = 5;`, `let y = "hello";`, `const z = true;`
- Assignments: `x = 10;`
- Expression statements: `42;`, `f(x);`
- Function declarations: `function add(a, b) { return a + b; }`
- If/else statements with chaining
- For loops with init/condition/update
- Return statements
- Arrow functions: `(x) => x + 1`, `(a, b) => a + b`
- Function calls: `f(a, b)`, `noop()`
- Binary expressions with precedence (equality > comparison > additive > multiplicative)
- Unary expressions: `-x`, `!flag`

## AST node format

```
$node->rule_name   # e.g. "var_declaration", "binary_expr"
$node->children    # arrayref of child nodes
$node->is_leaf     # 1 for token leaves, 0 for inner nodes
$node->token       # { type, value, line, col } (leaf only)
```

## Building and testing

```bash
cd code/packages/perl/typescript-parser
cat BUILD | bash
```

## Version

0.01
