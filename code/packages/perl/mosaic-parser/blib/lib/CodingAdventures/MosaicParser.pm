package CodingAdventures::MosaicParser;

# ============================================================================
# CodingAdventures::MosaicParser — Hand-written recursive-descent Mosaic parser
# ============================================================================
#
# This module parses Mosaic source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach, following the grammar
# rules in code/grammars/mosaic.grammar.
#
# Why hand-written?
# -----------------
# The task specifies: "IMPORTANT: Perl does NOT have grammar-tools. Lexer and
# parser are hand-written." The grammar-driven infrastructure is not available
# for Perl, so we implement the parser directly.
#
# Mosaic grammar (simplified)
# ---------------------------
#
#   file           = { import_decl } component_decl
#   import_decl    = "import" NAME [ "as" NAME ] "from" STRING ";"
#   component_decl = "component" NAME "{" { slot_decl } node_tree "}"
#   slot_decl      = "slot" NAME ":" slot_type [ "=" default_value ] ";"
#   slot_type      = KEYWORD | NAME | "list" "<" slot_type ">"
#   default_value  = STRING | NUMBER | DIMENSION | HEX_COLOR | KEYWORD
#   node_tree      = node_element
#   node_element   = NAME "{" { node_content } "}"
#   node_content   = property_assignment | child_node | slot_reference
#                  | when_block | each_block
#   property_assignment = (NAME | KEYWORD) ":" property_value ";"
#   property_value = "@" NAME | STRING | NUMBER | DIMENSION | HEX_COLOR
#                  | KEYWORD | NAME | NAME "." NAME
#   slot_reference = "@" NAME ";"
#   when_block     = "when" "@" NAME "{" { node_content } "}"
#   each_block     = "each" "@" NAME "as" NAME "{" { node_content } "}"
#
# AST node structure
# ------------------
#
# Every node is a hashref with at least a `rule_name` key. Internal nodes
# also have a `children` arrayref. Leaf nodes (token wrappers) have a
# `token` key. For example:
#
#   # Internal node
#   { rule_name => "component_decl", children => [...] }
#
#   # Token leaf
#   { rule_name => "token", is_leaf => 1,
#     token => { type => "NAME", value => "ProfileCard", line => 1, col => 11 } }
#
# Parse state
# -----------
#
# The parser uses two package-level variables set on each parse() call:
#   $tokens_ref — arrayref of token hashrefs from CodingAdventures::MosaicLexer
#   $pos        — current position (0-based)
#
# Public API
# ----------
#
#   my ($ast, $error) = CodingAdventures::MosaicParser->parse($source);
#
# On success: ($ast_hashref, undef)
# On failure: (undef, $error_string)

use strict;
use warnings;

use CodingAdventures::MosaicLexer;

our $VERSION = '0.01';

# Package-level parse state — set at the start of each parse() call.
my ($tokens_ref, $pos);

# ============================================================================
# Public API
# ============================================================================

sub parse {
    my ($class, $source) = @_;

    my ($toks, $lex_err) = CodingAdventures::MosaicLexer->tokenize($source);
    return (undef, $lex_err) if $lex_err;

    $tokens_ref = $toks;
    $pos = 0;

    my $ast = eval { _parse_file() };
    if ($@) {
        return (undef, $@);
    }

    # Verify we consumed all tokens except EOF
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        return (undef, sprintf(
            "CodingAdventures::MosaicParser: trailing content at line %d col %d: %s '%s'",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        ));
    }

    return ($ast, undef);
}

# ============================================================================
# Internal helpers
# ============================================================================

sub _peek  { $tokens_ref->[$pos] // $tokens_ref->[-1] }
sub _advance { my $t = _peek(); $pos++; $t }

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::MosaicParser: Expected %s, got %s ('%s') at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# Expect a KEYWORD token with a specific value (e.g., "component", "slot").
sub _expect_kw {
    my ($val) = @_;
    my $t = _peek();
    unless ($t->{type} eq 'KEYWORD' && $t->{value} eq $val) {
        die sprintf(
            "CodingAdventures::MosaicParser: Expected keyword '%s', got %s ('%s') at line %d col %d",
            $val, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# Helpers to peek at the current token type/value.
sub _peek_type  { _peek()->{type}  }
sub _peek_value { _peek()->{value} }

# Wrap a token as a leaf AST node.
sub _leaf { my ($tok) = @_; { rule_name => 'token', is_leaf => 1, token => $tok } }

# Build an internal AST node.
sub _node { my ($rule, $children) = @_; { rule_name => $rule, children => $children // [] } }

# ============================================================================
# Grammar productions
# ============================================================================

# file = { import_decl } component_decl
sub _parse_file {
    my @children;

    # Zero or more import declarations
    while (_peek_type() eq 'KEYWORD' && _peek_value() eq 'import') {
        push @children, _parse_import_decl();
    }

    # Exactly one component declaration
    unless (_peek_type() eq 'KEYWORD' && _peek_value() eq 'component') {
        die sprintf(
            "CodingAdventures::MosaicParser: Expected 'component', got %s ('%s') at line %d col %d",
            _peek_type(), _peek_value(), _peek()->{line}, _peek()->{col}
        );
    }
    push @children, _parse_component_decl();

    return _node('file', \@children);
}

# import_decl = "import" NAME [ "as" NAME ] "from" STRING ";"
sub _parse_import_decl {
    my @children;
    push @children, _leaf(_expect_kw('import'));
    push @children, _leaf(_expect('NAME'));

    # Optional: "as" NAME
    if (_peek_type() eq 'KEYWORD' && _peek_value() eq 'as') {
        push @children, _leaf(_advance());   # "as"
        push @children, _leaf(_expect('NAME'));
    }

    push @children, _leaf(_expect_kw('from'));
    push @children, _leaf(_expect('STRING'));
    push @children, _leaf(_expect('SEMICOLON'));

    return _node('import_decl', \@children);
}

# component_decl = "component" NAME "{" { slot_decl } node_tree "}"
sub _parse_component_decl {
    my @children;
    push @children, _leaf(_expect_kw('component'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('LBRACE'));

    # Zero or more slot declarations
    while (_peek_type() eq 'KEYWORD' && _peek_value() eq 'slot') {
        push @children, _parse_slot_decl();
    }

    # The node tree (exactly one root element)
    push @children, _parse_node_tree();

    push @children, _leaf(_expect('RBRACE'));

    return _node('component_decl', \@children);
}

# slot_decl = "slot" NAME ":" slot_type [ "=" default_value ] ";"
sub _parse_slot_decl {
    my @children;
    push @children, _leaf(_expect_kw('slot'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('COLON'));
    push @children, _parse_slot_type();

    # Optional default value
    if (_peek_type() eq 'EQUALS') {
        push @children, _leaf(_advance());   # "="
        push @children, _parse_default_value();
    }

    push @children, _leaf(_expect('SEMICOLON'));

    return _node('slot_decl', \@children);
}

# slot_type = KEYWORD | NAME | "list" "<" slot_type ">"
sub _parse_slot_type {
    if (_peek_type() eq 'KEYWORD' && _peek_value() eq 'list') {
        return _parse_list_type();
    }
    if (_peek_type() eq 'KEYWORD') {
        return _node('slot_type', [ _leaf(_advance()) ]);
    }
    if (_peek_type() eq 'NAME') {
        return _node('slot_type', [ _leaf(_advance()) ]);
    }
    die sprintf(
        "CodingAdventures::MosaicParser: Expected slot type, got %s ('%s') at line %d col %d",
        _peek_type(), _peek_value(), _peek()->{line}, _peek()->{col}
    );
}

# list_type = "list" "<" slot_type ">"
sub _parse_list_type {
    my @children;
    push @children, _leaf(_expect_kw('list'));
    push @children, _leaf(_expect('LANGLE'));
    push @children, _parse_slot_type();
    push @children, _leaf(_expect('RANGLE'));
    return _node('list_type', \@children);
}

# default_value = STRING | NUMBER | DIMENSION | HEX_COLOR | KEYWORD
sub _parse_default_value {
    my $t = _peek();
    my $type = $t->{type};
    if ($type =~ /^(STRING|NUMBER|DIMENSION|HEX_COLOR|KEYWORD)$/) {
        return _node('default_value', [ _leaf(_advance()) ]);
    }
    die sprintf(
        "CodingAdventures::MosaicParser: Expected default value, got %s ('%s') at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# node_tree = node_element
sub _parse_node_tree {
    return _node('node_tree', [ _parse_node_element() ]);
}

# node_element = NAME "{" { node_content } "}"
sub _parse_node_element {
    my @children;
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('LBRACE'));

    # node_content* — stop when we see the closing brace
    while (_peek_type() ne 'RBRACE' && _peek_type() ne 'EOF') {
        push @children, _parse_node_content();
    }

    push @children, _leaf(_expect('RBRACE'));
    return _node('node_element', \@children);
}

# node_content = property_assignment | child_node | slot_reference
#              | when_block | each_block
#
# Disambiguation:
#   AT followed by NAME → could be slot_reference (AT NAME SEMICOLON)
#                       → or start of a property_value (handled inside prop)
#   We peek 2 tokens ahead:
#     AT NAME SEMICOLON → slot_reference
#     NAME LBRACE       → child_node (another element)
#     NAME COLON        → property_assignment
#     KEYWORD COLON     → property_assignment (e.g., "color: #fff;")
#     KEYWORD(when) ...  → when_block
#     KEYWORD(each) ...  → each_block
sub _parse_node_content {
    my $t  = _peek();
    my $t2 = $tokens_ref->[$pos + 1] // $tokens_ref->[-1];

    # when block
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'when') {
        return _node('node_content', [ _parse_when_block() ]);
    }

    # each block
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'each') {
        return _node('node_content', [ _parse_each_block() ]);
    }

    # slot_reference: @name;
    if ($t->{type} eq 'AT' && $t2->{type} eq 'NAME') {
        my $t3 = $tokens_ref->[$pos + 2] // $tokens_ref->[-1];
        if ($t3->{type} eq 'SEMICOLON') {
            return _node('node_content', [ _parse_slot_reference() ]);
        }
        # Otherwise it's a property value (slot ref used as value) — fall through
    }

    # child_node: NAME LBRACE
    if ($t->{type} eq 'NAME' && $t2->{type} eq 'LBRACE') {
        return _node('node_content', [ _node('child_node', [ _parse_node_element() ]) ]);
    }

    # property_assignment: (NAME | KEYWORD) COLON ...
    if (($t->{type} eq 'NAME' || $t->{type} eq 'KEYWORD') && $t2->{type} eq 'COLON') {
        return _node('node_content', [ _parse_property_assignment() ]);
    }

    die sprintf(
        "CodingAdventures::MosaicParser: Unexpected token %s ('%s') in node content at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# property_assignment = (NAME | KEYWORD) ":" property_value ";"
sub _parse_property_assignment {
    my @children;
    my $t = _peek();
    unless ($t->{type} eq 'NAME' || $t->{type} eq 'KEYWORD') {
        die sprintf(
            "CodingAdventures::MosaicParser: Expected property name, got %s at line %d col %d",
            $t->{type}, $t->{line}, $t->{col}
        );
    }
    push @children, _leaf(_advance());
    push @children, _leaf(_expect('COLON'));
    push @children, _parse_property_value();
    push @children, _leaf(_expect('SEMICOLON'));
    return _node('property_assignment', \@children);
}

# property_value = slot_ref | STRING | NUMBER | DIMENSION | HEX_COLOR
#                | KEYWORD | enum_value (NAME "." NAME) | NAME
sub _parse_property_value {
    my $t  = _peek();
    my $t2 = $tokens_ref->[$pos + 1] // $tokens_ref->[-1];

    # slot_ref: @name
    if ($t->{type} eq 'AT') {
        return _node('property_value', [ _parse_slot_ref() ]);
    }

    # enum_value: NAME "." NAME
    if ($t->{type} eq 'NAME' && $t2->{type} eq 'DOT') {
        return _node('property_value', [ _parse_enum_value() ]);
    }

    # Literals and identifiers
    if ($t->{type} =~ /^(STRING|NUMBER|DIMENSION|HEX_COLOR|KEYWORD|NAME)$/) {
        return _node('property_value', [ _leaf(_advance()) ]);
    }

    die sprintf(
        "CodingAdventures::MosaicParser: Expected property value, got %s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# slot_ref = "@" NAME
sub _parse_slot_ref {
    my @children;
    push @children, _leaf(_expect('AT'));
    push @children, _leaf(_expect('NAME'));
    return _node('slot_ref', \@children);
}

# enum_value = NAME "." NAME
sub _parse_enum_value {
    my @children;
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('DOT'));
    push @children, _leaf(_expect('NAME'));
    return _node('enum_value', \@children);
}

# slot_reference = "@" NAME ";"   (used as a child, not a value)
sub _parse_slot_reference {
    my @children;
    push @children, _leaf(_expect('AT'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('SEMICOLON'));
    return _node('slot_reference', \@children);
}

# when_block = "when" "@" NAME "{" { node_content } "}"
sub _parse_when_block {
    my @children;
    push @children, _leaf(_expect_kw('when'));
    push @children, _parse_slot_ref();
    push @children, _leaf(_expect('LBRACE'));

    while (_peek_type() ne 'RBRACE' && _peek_type() ne 'EOF') {
        push @children, _parse_node_content();
    }

    push @children, _leaf(_expect('RBRACE'));
    return _node('when_block', \@children);
}

# each_block = "each" "@" NAME "as" NAME "{" { node_content } "}"
sub _parse_each_block {
    my @children;
    push @children, _leaf(_expect_kw('each'));
    push @children, _parse_slot_ref();
    push @children, _leaf(_expect_kw('as'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('LBRACE'));

    while (_peek_type() ne 'RBRACE' && _peek_type() ne 'EOF') {
        push @children, _parse_node_content();
    }

    push @children, _leaf(_expect('RBRACE'));
    return _node('each_block', \@children);
}

1;

__END__

=head1 NAME

CodingAdventures::MosaicParser - Hand-written recursive-descent Mosaic parser

=head1 SYNOPSIS

    use CodingAdventures::MosaicParser;

    my ($ast, $error) = CodingAdventures::MosaicParser->parse($source);
    die $error if $error;
    print $ast->{rule_name};  # "file"

=head1 DESCRIPTION

A hand-written recursive-descent parser for the Mosaic Component Description Language.
Produces a nested hashref AST. Each node has a C<rule_name> and C<children> arrayref.
Leaf nodes have C<is_leaf =E<gt> 1> and a C<token> hashref.

=head1 METHODS

=head2 parse($source)

Parse Mosaic source text. Returns C<($ast, undef)> on success or
C<(undef, $error_string)> on failure.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
