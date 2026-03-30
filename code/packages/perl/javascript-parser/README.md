# CodingAdventures::JavascriptParser

A hand-written recursive-descent JavaScript parser for the coding-adventures monorepo. It tokenizes JavaScript source with `CodingAdventures::JavascriptLexer` and builds an Abstract Syntax Tree (AST) of `CodingAdventures::JavascriptParser::ASTNode` nodes.

## What it does

Given input `function add(a, b) { return a + b; }`, the parser produces:

```
program
└── statement
    └── function_decl
        ├── FUNCTION token
        ├── NAME "add"
        ├── LPAREN token
        ├── param_list
        │   ├── NAME "a"
        │   ├── COMMA token
        │   └── NAME "b"
        ├── RPAREN token
        └── block
            ├── LBRACE token
            ├── statement
            │   └── return_stmt
            │       ├── RETURN token
            │       ├── expression
            │       │   └── binary_expr
            │       │       ├── primary → NAME "a"
            │       │       ├── PLUS "+"
            │       │       └── primary → NAME "b"
            │       └── SEMICOLON token
            └── RBRACE token
```

## Supported JavaScript constructs

- Variable declarations: `var x = 5;`  `let y = "hello";`  `const z = true;`
- Assignments: `x = 10;`
- Expression statements: `42;`  `f(x);`
- Function declarations: `function add(a, b) { return a + b; }`
- If/else statements: `if (x > 0) { … } else { … }`
- For loops: `for (let i = 0; i < 10; i = i + 1) { … }`
- Return statements: `return x + 1;`
- Arrow functions: `(x) => x + 1`  `(a, b) => a + b`
- Function calls: `f(a, b)`
- Binary expressions with correct precedence
- Unary expressions: `-x`  `!flag`

## Operator precedence

| Level | Operators |
|-------|-----------|
| equality | `===` `!==` `==` `!=` |
| comparison | `<` `>` `<=` `>=` |
| additive | `+` `-` |
| multiplicative | `*` `/` |
| unary | `!` `-` |
| primary | literals, names, calls, `(expr)`, arrow |

## How it fits in the stack

```
CodingAdventures::JavascriptParser  ← this package (hand-written)
              ↓ uses
CodingAdventures::JavascriptLexer   → tokenizes JavaScript
              ↓ uses
CodingAdventures::GrammarTools / Lexer
```

## Usage

```perl
use CodingAdventures::JavascriptParser;

# Object-oriented
my $parser = CodingAdventures::JavascriptParser->new("var x = 5;");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::JavascriptParser->parse_js(
    "function add(a, b) { return a + b; }"
);

# Walk the AST
sub find_node {
    my ($node, $rule_name) = @_;
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if $found;
    }
    return undef;
}

my $fn = find_node($ast, 'function_decl');
```

## AST Node Format

Each node is a `CodingAdventures::JavascriptParser::ASTNode`:

```perl
$node->rule_name   # e.g. "var_declaration", "if_stmt", "binary_expr"
$node->children    # arrayref of child ASTNode objects
$node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
$node->token       # token hashref (leaf only): {type, value, line, col}
```

## Version

0.01
