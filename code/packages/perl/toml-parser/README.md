# CodingAdventures::TomlParser

A hand-written recursive-descent TOML parser for the coding-adventures monorepo. It tokenizes TOML source with `CodingAdventures::TomlLexer` and produces an Abstract Syntax Tree (AST) using `CodingAdventures::TomlParser::ASTNode`.

## Why hand-written (not grammar-driven)?

The Perl `CodingAdventures::GrammarTools` module only provides `parse_token_grammar`, not `parse_parser_grammar`. There is no grammar-driven `GrammarParser` in the Perl layer, so this parser is implemented by hand.

## TOML-specific notes

**Newlines are significant** in TOML — key-value pairs are terminated by newlines. The parser explicitly handles NEWLINE tokens (unlike JSON, where whitespace including newlines is insignificant).

**Array-of-tables disambiguation** — Both `[table]` and `[[array]]` start with LBRACKET. The parser peeks one token ahead to distinguish them.

**Multi-line arrays** — NEWLINE tokens inside `[...]` are allowed and consumed.

## Grammar (implemented rules)

```
document           = { NEWLINE | expression }
expression         = array_table_header | table_header | keyval
keyval             = key EQUALS value
key                = simple_key { DOT simple_key }
simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING | TRUE | FALSE
                   | INTEGER | FLOAT | OFFSET_DATETIME | LOCAL_DATETIME
                   | LOCAL_DATE | LOCAL_TIME
table_header       = LBRACKET key RBRACKET
array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET
value              = scalar | array | inline_table
array              = LBRACKET array_values RBRACKET
array_values       = (with optional newlines between elements, trailing comma)
inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE
```

## Usage

```perl
use CodingAdventures::TomlParser;

my $ast = CodingAdventures::TomlParser->parse(<<'TOML');
[server]
host = "localhost"
port = 8080
debug = true
TOML

print $ast->rule_name;  # "document"

# Find all keyval nodes
sub find_all {
    my ($node, $rule, $results) = @_;
    $results //= [];
    return $results unless ref $node && $node->can('rule_name');
    push @$results, $node if $node->rule_name eq $rule;
    find_all($_, $rule, $results) for @{ $node->children };
    return $results;
}

my $kvs = find_all($ast, 'keyval');
printf "Found %d key-value pairs\n", scalar @$kvs;
```

## Error handling

`parse()` dies with a descriptive message on any error:

```
CodingAdventures::TomlParser: Expected EQUALS, got BARE_KEY ('value') at line 1 col 5
CodingAdventures::TomlParser: Expected RBRACKET, got EOF ('') at line 1 col 8
```

## How it fits in the stack

```
CodingAdventures::TomlParser  ← this package
     ↓
CodingAdventures::TomlLexer
     ↓
CodingAdventures::Lexer + CodingAdventures::GrammarTools
```

## Version

0.01
