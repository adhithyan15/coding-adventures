# CodingAdventures::CssParser (Perl)

A hand-written recursive-descent CSS3 parser. Tokenizes source text with `CodingAdventures::CssLexer` and constructs an Abstract Syntax Tree using `CodingAdventures::CssParser::ASTNode`.

## What it does

Given the input `h1 { color: red; }`, the parser produces:

```
ASTNode(stylesheet)
└── ASTNode(rule)
    └── ASTNode(qualified_rule)
        ├── ASTNode(selector_list)
        │   └── ASTNode(complex_selector)
        │       └── ASTNode(compound_selector)
        │           └── ASTNode(simple_selector)
        │               └── Leaf(IDENT "h1")
        └── ASTNode(block)
            ├── Leaf(LBRACE "{")
            ├── ASTNode(block_contents)
            │   └── ASTNode(block_item)
            │       └── ASTNode(declaration_or_nested)
            │           └── ASTNode(declaration)
            │               ├── ASTNode(property) → Leaf(IDENT "color")
            │               ├── Leaf(COLON ":")
            │               ├── ASTNode(value_list)
            │               │   └── ASTNode(value) → Leaf(IDENT "red")
            │               └── Leaf(SEMICOLON ";")
            └── Leaf(RBRACE "}")
```

## Supported CSS features

- **Selectors**: type (`h1`), class (`.active`), ID (`#header`), attribute (`[disabled]`), pseudo-class (`:hover`, `:nth-child(2n+1)`), pseudo-element (`::before`), combinators (`>`, `+`, `~`), CSS nesting (`&`)
- **At-rules**: `@import "file.css";`, `@charset "UTF-8";`, `@media screen { }`, `@keyframes`, `@font-face`
- **Declarations**: `color: red;`, `font-size: 16px;`, `--var: value;`, `margin: 10px 20px !important;`
- **Values**: DIMENSION, PERCENTAGE, NUMBER, STRING, IDENT, HASH, CUSTOM_PROPERTY, function calls (`rgba()`, `calc()`, `var()`), URL tokens

## Usage

```perl
use CodingAdventures::CssParser;

my $ast = CodingAdventures::CssParser->parse(<<'CSS');
h1 { color: red; }
@media screen { p { font-size: 16px; } }
CSS

print $ast->rule_name;  # "stylesheet"

# Walk the tree
sub walk {
    my ($node, $depth) = @_;
    my $indent = '  ' x $depth;
    if ($node->is_leaf) {
        printf "%sToken(%s %s)\n", $indent, $node->token->{type}, $node->token->{value};
    } else {
        printf "%s%s\n", $indent, $node->rule_name;
        walk($_, $depth + 1) for @{ $node->children };
    }
}
walk($ast, 0);
```

## CSS parsing challenges

**Declaration vs. nested rule disambiguation**: both can start with IDENT. The parser peeks one token ahead — IDENT followed by COLON is a declaration; otherwise it's a nested qualified rule.

**At-rule preludes**: `@media screen and (min-width: 768px) { }` — the prelude is consumed as a flexible token sequence with paren-depth tracking.

**Function arguments**: FUNCTION tokens include the opening paren (`rgba(`), so args are collected until the matching RPAREN with nesting support for `calc(100% - var(--x, 20px))`.

## Running tests

```bash
prove -l -v t/
```
