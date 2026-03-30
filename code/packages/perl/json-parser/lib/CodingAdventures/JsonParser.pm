package CodingAdventures::JsonParser;

# ============================================================================
# CodingAdventures::JsonParser — Hand-written recursive-descent JSON parser
# ============================================================================
#
# This module parses JSON source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach rather than the grammar-
# driven infrastructure used by the Lua json_parser package.
#
# Why hand-written?
# -----------------
# The Perl CodingAdventures::GrammarTools module only provides
# `parse_token_grammar`, not `parse_parser_grammar`.  There is no grammar-
# driven GrammarParser in the Perl layer, so we implement the parser by
# hand, following the same grammar rules as the Lua implementation.
#
# # JSON grammar
#
#   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
#   object = LBRACE [ pair { COMMA pair } ] RBRACE
#   pair   = STRING COLON value
#   array  = LBRACKET [ value { COMMA value } ] RBRACKET
#
# This is a classic LL(1) grammar — at each step we only need to look at the
# current token to decide which production to apply:
#
#   LBRACE    → object
#   LBRACKET  → array
#   STRING    → string leaf
#   NUMBER    → number leaf
#   TRUE      → true leaf
#   FALSE     → false leaf
#   NULL      → null leaf
#
# # AST structure
#
# Internal nodes:
#   { rule_name => "value",  children => [...], is_leaf => 0 }
#   { rule_name => "object", children => [...], is_leaf => 0 }
#   { rule_name => "pair",   children => [...], is_leaf => 0 }
#   { rule_name => "array",  children => [...], is_leaf => 0 }
#
# Leaf nodes (token wrappers):
#   { rule_name => "token", children => [], is_leaf => 1,
#     token => { type => "NUMBER", value => "42", line => 1, col => 8 } }
#
# # Parse state
#
# The parser keeps two package-level variables:
#   $tokens_ref — arrayref of token hashrefs from CodingAdventures::JsonLexer
#   $pos        — current position (0-based index into @$tokens_ref)
#
# These are set by `parse()` on each call, so the parser is not re-entrant,
# but JSON parsing is synchronous so that's fine in practice.
#
# # Error messages
#
# When a token doesn't match what's expected, `_expect` raises a die with a
# message of the form:
#   "Expected TYPE, got ACTUAL_TYPE ('value') at line L col C"
#
# This makes it easy to pinpoint the bad input.

use strict;
use warnings;

use CodingAdventures::JsonLexer;
use CodingAdventures::JsonParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state.
# Set at the start of each `parse()` call.
my ($tokens_ref, $pos);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a JSON source string and return the root ASTNode.
#
# Steps:
#   1. Tokenize `$source` using CodingAdventures::JsonLexer.
#   2. Initialize parser state ($tokens_ref, $pos).
#   3. Parse a single `value` production.
#   4. Assert that we consumed all tokens (only EOF remains).
#   5. Return the root ASTNode.
#
# Dies on lexer errors or parse errors.
#
# @param  $source  string  The JSON text to parse.
# @return ASTNode          Root node with rule_name "value".

sub parse {
    my ($class, $source) = @_;

    # Tokenize
    my $toks = CodingAdventures::JsonLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos = 0;

    # Parse the top-level value
    my $ast = _parse_value();

    # Verify we consumed everything — only EOF should remain
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::JsonParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return $ast;
}

# ============================================================================
# Internal helpers
# ============================================================================

# --- _peek() ------------------------------------------------------------------
#
# Return the current token without consuming it.
# When past the end of the token list, returns the last token (EOF).

sub _peek {
    return $tokens_ref->[$pos] // $tokens_ref->[-1];
}

# --- _advance() ---------------------------------------------------------------
#
# Consume and return the current token, advancing the position.

sub _advance {
    my $t = _peek();
    $pos++;
    return $t;
}

# --- _expect($type) -----------------------------------------------------------
#
# Assert that the current token has type `$type`, consume it, and return it.
# Dies with a descriptive message if the type doesn't match.
#
# Example:
#   _expect('COLON')  — dies "Expected COLON, got NUMBER ('42') at line 1 col 7"

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::JsonParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _node($rule_name, $children_aref) ----------------------------------------
#
# Construct an internal (non-leaf) ASTNode.
#
# @param $rule_name  string    The grammar rule that produced this node.
# @param $children   arrayref  Child ASTNodes.
# @return ASTNode

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::JsonParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

# --- _leaf($token) ------------------------------------------------------------
#
# Construct a leaf ASTNode wrapping a single token.
#
# @param $token  hashref  A token from CodingAdventures::JsonLexer.
# @return ASTNode

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::JsonParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Recursive descent productions
# ============================================================================

# --- _parse_value() -----------------------------------------------------------
#
# Grammar: value = object | array | STRING | NUMBER | TRUE | FALSE | NULL
#
# Decision table (current token type → production):
#
#   LBRACE    → _parse_object()
#   LBRACKET  → _parse_array()
#   STRING    → leaf node wrapping the STRING token
#   NUMBER    → leaf node wrapping the NUMBER token
#   TRUE      → leaf node wrapping the TRUE token
#   FALSE     → leaf node wrapping the FALSE token
#   NULL      → leaf node wrapping the NULL token
#   anything else → die

sub _parse_value {
    my $t    = _peek();
    my $type = $t->{type};

    # Compound structures: delegated to their own sub-parsers
    return _node('value', [_parse_object()]) if $type eq 'LBRACE';
    return _node('value', [_parse_array()])  if $type eq 'LBRACKET';

    # Atomic values: consume the token and wrap it in a leaf
    if ($type =~ /^(STRING|NUMBER|TRUE|FALSE|NULL)$/) {
        my $tok = _advance();
        return _node('value', [_leaf($tok)]);
    }

    die sprintf(
        "CodingAdventures::JsonParser: Unexpected token %s ('%s') "
      . "at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_object() ----------------------------------------------------------
#
# Grammar: object = LBRACE [ pair { COMMA pair } ] RBRACE
#
# An object is a (possibly empty) comma-separated list of pairs enclosed in
# braces.  The `[ pair { COMMA pair } ]` idiom means:
#
#   - If the next token is RBRACE: empty object, skip the pair list entirely.
#   - Otherwise: parse the first pair, then loop consuming COMMA pair until
#     the next token is RBRACE.
#
# All tokens (braces, commas) are included in the children so the AST is
# a complete faithful representation of the source.

sub _parse_object {
    my @children;

    my $open = _expect('LBRACE');
    push @children, _leaf($open);

    # Empty object: {} → no pairs
    if (_peek()->{type} ne 'RBRACE') {
        # First pair
        push @children, _parse_pair();

        # Additional pairs, each preceded by a comma
        while (_peek()->{type} eq 'COMMA') {
            push @children, _leaf(_advance());   # consume COMMA
            push @children, _parse_pair();
        }
    }

    my $close = _expect('RBRACE');
    push @children, _leaf($close);

    return _node('object', \@children);
}

# --- _parse_pair() ------------------------------------------------------------
#
# Grammar: pair = STRING COLON value
#
# A key-value pair: a string key, a colon separator, and any JSON value.

sub _parse_pair {
    my @children;

    my $key   = _expect('STRING');
    push @children, _leaf($key);

    my $colon = _expect('COLON');
    push @children, _leaf($colon);

    push @children, _parse_value();

    return _node('pair', \@children);
}

# --- _parse_array() -----------------------------------------------------------
#
# Grammar: array = LBRACKET [ value { COMMA value } ] RBRACKET
#
# An array is a (possibly empty) comma-separated list of values enclosed in
# brackets.  Mirrors the object logic but uses values instead of pairs.

sub _parse_array {
    my @children;

    my $open = _expect('LBRACKET');
    push @children, _leaf($open);

    # Empty array: [] → no values
    if (_peek()->{type} ne 'RBRACKET') {
        # First value
        push @children, _parse_value();

        # Additional values, each preceded by a comma
        while (_peek()->{type} eq 'COMMA') {
            push @children, _leaf(_advance());   # consume COMMA
            push @children, _parse_value();
        }
    }

    my $close = _expect('RBRACKET');
    push @children, _leaf($close);

    return _node('array', \@children);
}

1;

__END__

=head1 NAME

CodingAdventures::JsonParser - Hand-written recursive-descent JSON parser

=head1 SYNOPSIS

    use CodingAdventures::JsonParser;

    my $ast = CodingAdventures::JsonParser->parse('{"key": 42}');
    print $ast->rule_name;    # "value"

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

=head1 DESCRIPTION

A hand-written recursive-descent parser for JSON.  Tokenizes source text
using C<CodingAdventures::JsonLexer> and constructs an AST using
C<CodingAdventures::JsonParser::ASTNode>.

Implements the four JSON grammar rules:

    value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
    object = LBRACE [ pair { COMMA pair } ] RBRACE
    pair   = STRING COLON value
    array  = LBRACKET [ value { COMMA value } ] RBRACKET

=head1 METHODS

=head2 parse($source)

Parse a JSON string.  Returns the root C<ASTNode> with C<rule_name == "value">.
Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
