# CodingAdventures::LispParser

Perl module — hand-written recursive-descent Lisp/Scheme parser.

## What it does

Parses Lisp/Scheme source text into an AST using the six grammar rules:

```
program   = { sexpr }
sexpr     = atom | list | quoted
atom      = NUMBER | SYMBOL | STRING
list      = LPAREN list_body RPAREN
list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
quoted    = QUOTE sexpr
```

```perl
use CodingAdventures::LispParser;

my $ast = CodingAdventures::LispParser->parse('(define x 42)');
print $ast->rule_name;    # "program"

# Walk the tree
sub walk {
    my ($node, $depth) = @_;
    my $indent = '  ' x $depth;
    if ($node->is_leaf) {
        printf "%sToken(%s, %s)\n",
            $indent, $node->token->{type}, $node->token->{value};
    } else {
        printf "%s%s\n", $indent, $node->rule_name;
        walk($_, $depth + 1) for @{ $node->children };
    }
}
walk($ast, 0);
```

## Dependencies

- `CodingAdventures::LispLexer` — tokenizes the source
- `CodingAdventures::LispParser::ASTNode` — internal AST node class

## How it fits in the stack

```
LispLexer ──→ LispParser   ← this module
                   ↓
          (evaluator, macro expander, …)
```
