package CodingAdventures::TomlParser;

# ============================================================================
# CodingAdventures::TomlParser — Hand-written recursive-descent TOML parser
# ============================================================================
#
# This module parses TOML source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach because the Perl
# CodingAdventures::GrammarTools module only provides `parse_token_grammar`,
# not `parse_parser_grammar`.  There is no grammar-driven GrammarParser in
# the Perl layer.
#
# # TOML grammar (subset implemented here)
#
#   document           = { NEWLINE | expression }
#   expression         = array_table_header | table_header | keyval
#   keyval             = key EQUALS value
#   key                = simple_key { DOT simple_key }
#   simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING
#                      | TRUE | FALSE | INTEGER | FLOAT
#                      | OFFSET_DATETIME | LOCAL_DATETIME | LOCAL_DATE | LOCAL_TIME
#   table_header       = LBRACKET key RBRACKET
#   array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET
#   value              = BASIC_STRING | ML_BASIC_STRING | LITERAL_STRING
#                      | ML_LITERAL_STRING | INTEGER | FLOAT | TRUE | FALSE
#                      | OFFSET_DATETIME | LOCAL_DATETIME | LOCAL_DATE | LOCAL_TIME
#                      | array | inline_table
#   array              = LBRACKET array_values RBRACKET
#   array_values       = { NEWLINE } [ value { NEWLINE }
#                          { COMMA { NEWLINE } value { NEWLINE } }
#                          [ COMMA ] { NEWLINE } ]
#   inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE
#
# # Key TOML parsing considerations
#
# 1. **NEWLINEs are significant** — TOML key-value pairs are terminated by
#    newlines.  The parser explicitly checks for NEWLINE to skip blank lines
#    and to terminate keyval productions.
#
# 2. **Array-of-tables vs table header disambiguation** — Both start with
#    LBRACKET.  We peek one token ahead: if the next-next token is also
#    LBRACKET, it's an array_table_header.
#
# 3. **Multi-line arrays** — NEWLINE tokens inside a LBRACKET/RBRACKET pair
#    are allowed and consumed.
#
# 4. **Inline tables** — LBRACE starts an inline_table, not an object.
#    The distinction from JSON objects is that inline tables use `key = val`
#    syntax internally, not `"string": val`.
#
# # Parse state
#
# Two package-level variables hold the current parse state:
#   $tokens_ref — arrayref from CodingAdventures::TomlLexer->tokenize
#   $pos        — 0-based current index
#
# The parser is not re-entrant (package-level state), but TOML parsing is
# always synchronous so this is fine in practice.

use strict;
use warnings;

use CodingAdventures::TomlLexer;
use CodingAdventures::TomlParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state
my ($tokens_ref, $pos);

# ============================================================================
# Simple-key token types
# ============================================================================
#
# TOML bare keys can look like booleans, numbers, or dates.  Since the lexer
# tokenizes them as their "natural" type (TRUE, INTEGER, LOCAL_DATE, etc.),
# the parser must accept all of these in key position.
#
# This set is used by _parse_simple_key to decide whether the current token
# can start a key.

my %SIMPLE_KEY_TYPES = map { $_ => 1 } qw(
    BARE_KEY BASIC_STRING LITERAL_STRING
    TRUE FALSE INTEGER FLOAT
    OFFSET_DATETIME LOCAL_DATETIME LOCAL_DATE LOCAL_TIME
);

# Value token types (non-compound).  These are the token types that can
# appear as a bare scalar value in TOML.

my %SCALAR_VALUE_TYPES = map { $_ => 1 } qw(
    BASIC_STRING ML_BASIC_STRING LITERAL_STRING ML_LITERAL_STRING
    INTEGER FLOAT TRUE FALSE
    OFFSET_DATETIME LOCAL_DATETIME LOCAL_DATE LOCAL_TIME
);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a TOML source string and return the root ASTNode.
#
# @param  $source  string  The TOML text to parse.
# @return ASTNode          Root with rule_name "document".
# @die                     On any lexer or parser error.

sub parse {
    my ($class, $source) = @_;

    my $toks = CodingAdventures::TomlLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos = 0;

    my $ast = _parse_document();

    # Verify we consumed everything
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::TomlParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return $ast;
}

# ============================================================================
# Internal helpers
# ============================================================================

sub _peek    { $tokens_ref->[$pos] // $tokens_ref->[-1] }
sub _peek_at { $tokens_ref->[$pos + $_[0]] // $tokens_ref->[-1] }

sub _advance {
    my $t = _peek();
    $pos++;
    return $t;
}

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::TomlParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::TomlParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::TomlParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Recursive descent productions
# ============================================================================

# --- _parse_document() --------------------------------------------------------
#
# Grammar: document = { NEWLINE | expression }
#
# A TOML document is a sequence of expressions (key-value pairs, table
# headers, array-of-tables headers) and blank lines (represented as
# NEWLINE tokens).
#
# We keep consuming until we hit EOF.

sub _parse_document {
    my @children;

    while (_peek()->{type} ne 'EOF') {
        my $t = _peek();

        if ($t->{type} eq 'NEWLINE') {
            # Blank line: consume the NEWLINE and record it
            push @children, _leaf(_advance());
            next;
        }

        # Try to parse an expression (table_header, array_table_header, keyval)
        my $expr = _parse_expression();
        push @children, $expr;

        # After an expression, consume the terminating NEWLINE (if present)
        if (_peek()->{type} eq 'NEWLINE') {
            push @children, _leaf(_advance());
        }
    }

    return _node('document', \@children);
}

# --- _parse_expression() ------------------------------------------------------
#
# Grammar: expression = array_table_header | table_header | keyval
#
# Disambiguation:
#   LBRACKET LBRACKET …  → array_table_header
#   LBRACKET …           → table_header
#   anything else        → keyval

sub _parse_expression {
    my $t = _peek();

    if ($t->{type} eq 'LBRACKET') {
        # Look ahead: if pos+1 is also LBRACKET → array_table_header
        if (_peek_at(1)->{type} eq 'LBRACKET') {
            return _node('expression', [_parse_array_table_header()]);
        }
        return _node('expression', [_parse_table_header()]);
    }

    # Must be a key-value pair
    return _node('expression', [_parse_keyval()]);
}

# --- _parse_keyval() ----------------------------------------------------------
#
# Grammar: keyval = key EQUALS value

sub _parse_keyval {
    my @children;

    push @children, _parse_key();

    my $eq = _expect('EQUALS');
    push @children, _leaf($eq);

    push @children, _parse_value();

    return _node('keyval', \@children);
}

# --- _parse_key() -------------------------------------------------------------
#
# Grammar: key = simple_key { DOT simple_key }

sub _parse_key {
    my @children;

    push @children, _parse_simple_key();

    # Dotted keys: a.b.c
    while (_peek()->{type} eq 'DOT') {
        push @children, _leaf(_advance());    # DOT
        push @children, _parse_simple_key();  # next component
    }

    return _node('key', \@children);
}

# --- _parse_simple_key() ------------------------------------------------------
#
# Grammar: simple_key = BARE_KEY | BASIC_STRING | LITERAL_STRING
#                      | TRUE | FALSE | INTEGER | FLOAT
#                      | OFFSET_DATETIME | LOCAL_DATETIME | LOCAL_DATE | LOCAL_TIME
#
# TOML allows any of these token types as a key name.  The most common case
# is BARE_KEY (an unquoted identifier like `host` or `my-key`), but
# quoted strings and even "true" (the boolean) are valid key names.

sub _parse_simple_key {
    my $t = _peek();
    unless ($SIMPLE_KEY_TYPES{$t->{type}}) {
        die sprintf(
            "CodingAdventures::TomlParser: Expected a key, got %s ('%s') "
          . "at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _node('simple_key', [_leaf(_advance())]);
}

# --- _parse_table_header() ----------------------------------------------------
#
# Grammar: table_header = LBRACKET key RBRACKET

sub _parse_table_header {
    my @children;

    push @children, _leaf(_expect('LBRACKET'));
    push @children, _parse_key();
    push @children, _leaf(_expect('RBRACKET'));

    return _node('table_header', \@children);
}

# --- _parse_array_table_header() ----------------------------------------------
#
# Grammar: array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET

sub _parse_array_table_header {
    my @children;

    push @children, _leaf(_expect('LBRACKET'));
    push @children, _leaf(_expect('LBRACKET'));
    push @children, _parse_key();
    push @children, _leaf(_expect('RBRACKET'));
    push @children, _leaf(_expect('RBRACKET'));

    return _node('array_table_header', \@children);
}

# --- _parse_value() -----------------------------------------------------------
#
# Grammar: value = scalar_type | array | inline_table
#
# Decision table:
#   LBRACKET → array
#   LBRACE   → inline_table
#   scalar   → leaf node

sub _parse_value {
    my $t = _peek();

    if ($t->{type} eq 'LBRACKET') {
        return _node('value', [_parse_array()]);
    }

    if ($t->{type} eq 'LBRACE') {
        return _node('value', [_parse_inline_table()]);
    }

    if ($SCALAR_VALUE_TYPES{$t->{type}}) {
        return _node('value', [_leaf(_advance())]);
    }

    die sprintf(
        "CodingAdventures::TomlParser: Expected a value, got %s ('%s') "
      . "at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_array() -----------------------------------------------------------
#
# Grammar: array = LBRACKET array_values RBRACKET
#
# TOML arrays can span multiple lines.  Newlines are allowed between elements
# and after commas.

sub _parse_array {
    my @children;

    push @children, _leaf(_expect('LBRACKET'));
    push @children, _parse_array_values();
    push @children, _leaf(_expect('RBRACKET'));

    return _node('array', \@children);
}

# --- _parse_array_values() ----------------------------------------------------
#
# Grammar: array_values = { NEWLINE }
#                         [ value { NEWLINE }
#                           { COMMA { NEWLINE } value { NEWLINE } }
#                           [ COMMA ]
#                           { NEWLINE } ]
#
# Handles:
#   []                   — empty
#   [1, 2, 3]            — single line
#   [                    — multi-line (newlines between elements)
#     1,
#     2,
#     3,                 — trailing comma permitted
#   ]

sub _parse_array_values {
    my @children;

    # Leading newlines
    while (_peek()->{type} eq 'NEWLINE') {
        push @children, _leaf(_advance());
    }

    # Empty array (no values before RBRACKET)
    return _node('array_values', \@children)
        if _peek()->{type} eq 'RBRACKET';

    # First value
    push @children, _parse_value();
    while (_peek()->{type} eq 'NEWLINE') {
        push @children, _leaf(_advance());
    }

    # Additional values (each preceded by COMMA)
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());   # COMMA

        # Newlines after comma
        while (_peek()->{type} eq 'NEWLINE') {
            push @children, _leaf(_advance());
        }

        # Check for trailing comma (no more values before RBRACKET)
        last if _peek()->{type} eq 'RBRACKET';

        push @children, _parse_value();

        # Newlines after value
        while (_peek()->{type} eq 'NEWLINE') {
            push @children, _leaf(_advance());
        }
    }

    return _node('array_values', \@children);
}

# --- _parse_inline_table() ----------------------------------------------------
#
# Grammar: inline_table = LBRACE [ keyval { COMMA keyval } ] RBRACE
#
# Inline tables are compact, single-line table definitions like:
#   point = { x = 1, y = 2 }
#
# The TOML spec forbids newlines inside inline tables.  We don't enforce
# that here (the grammar layer doesn't); semantic validation would handle it.

sub _parse_inline_table {
    my @children;

    push @children, _leaf(_expect('LBRACE'));

    # Empty inline table: {}
    unless (_peek()->{type} eq 'RBRACE') {
        # First key-value pair
        push @children, _parse_keyval();

        # Additional pairs
        while (_peek()->{type} eq 'COMMA') {
            push @children, _leaf(_advance());   # COMMA
            push @children, _parse_keyval();
        }
    }

    push @children, _leaf(_expect('RBRACE'));

    return _node('inline_table', \@children);
}

1;

__END__

=head1 NAME

CodingAdventures::TomlParser - Hand-written recursive-descent TOML parser

=head1 SYNOPSIS

    use CodingAdventures::TomlParser;

    my $ast = CodingAdventures::TomlParser->parse(<<'TOML');
    [server]
    host = "localhost"
    port = 8080
    debug = true
    TOML

    print $ast->rule_name;  # "document"

=head1 DESCRIPTION

A hand-written recursive-descent parser for TOML (Tom's Obvious, Minimal
Language).  Tokenizes source text using C<CodingAdventures::TomlLexer> and
constructs an AST using C<CodingAdventures::TomlParser::ASTNode>.

Implements the TOML grammar rules: document, expression, keyval, key,
simple_key, table_header, array_table_header, value, array, array_values,
inline_table.

TOML is newline-sensitive: key-value pairs are terminated by newlines.
The parser explicitly handles NEWLINE tokens as significant.

=head1 METHODS

=head2 parse($source)

Parse a TOML string.  Returns the root C<ASTNode> with
C<rule_name == "document">.  Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
