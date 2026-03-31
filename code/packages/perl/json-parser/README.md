# CodingAdventures::JsonParser

A hand-written recursive-descent JSON parser for the coding-adventures monorepo. It tokenizes JSON source with `CodingAdventures::JsonLexer` and produces an Abstract Syntax Tree (AST) using `CodingAdventures::JsonParser::ASTNode`.

## Why hand-written (not grammar-driven)?

The Perl `CodingAdventures::GrammarTools` module only provides `parse_token_grammar`, not `parse_parser_grammar`. There is no grammar-driven `GrammarParser` in the Perl layer. The parser is therefore implemented by hand, following the same four grammar rules as the Lua `json_parser` package.

## Grammar

```
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
object = LBRACE [ pair { COMMA pair } ] RBRACE
pair   = STRING COLON value
array  = LBRACKET [ value { COMMA value } ] RBRACKET
```

## Usage

```perl
use CodingAdventures::JsonParser;

my $ast = CodingAdventures::JsonParser->parse('{"name": "Alice", "age": 30}');

print $ast->rule_name;  # "value"

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

## AST Node API

`CodingAdventures::JsonParser::ASTNode` provides:

- `rule_name()` — the grammar rule name (`"value"`, `"object"`, `"pair"`, `"array"`, `"token"`)
- `children()` — arrayref of child ASTNodes
- `is_leaf()` — 1 if this node wraps a single token, 0 otherwise
- `token()` — the wrapped token hashref (type, value, line, col); only valid when `is_leaf()` is true

## Error handling

`parse()` dies with a descriptive message on any lexer or parser error:

```
CodingAdventures::JsonParser: Expected COLON, got NUMBER ('42') at line 1 col 7
CodingAdventures::JsonParser: trailing content at line 1 col 4: unexpected RBRACE ('}')
```

## How it fits in the stack

```
CodingAdventures::JsonParser  ← this package
     ↓
CodingAdventures::JsonLexer
     ↓
CodingAdventures::Lexer + CodingAdventures::GrammarTools
```

## Version

0.01
