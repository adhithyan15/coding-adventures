package CodingAdventures::AlgolParser::ASTNode;

# ============================================================================
# CodingAdventures::AlgolParser::ASTNode — AST node for the ALGOL 60 parser
# ============================================================================
#
# An ASTNode represents a single node in the Abstract Syntax Tree produced
# by the ALGOL 60 parser.  Each node corresponds to either:
#
#   - An internal rule node: produced by a grammar rule (program, block,
#     statement, expression, etc.).  Has a rule_name and child ASTNodes.
#
#   - A leaf node: wraps a single lexer token (BEGIN, INTEGER_LIT, IDENT, etc.).
#     Has is_leaf == 1 and a token hashref.
#
# # AST structure for `begin integer x; x := 42 end`
#
#   ASTNode(rule_name="program")
#   └── ASTNode(rule_name="block")
#       ├── ASTNode(rule_name="token", is_leaf=1, token={type=BEGIN,  value="begin"})
#       ├── ASTNode(rule_name="declaration")
#       │   └── ASTNode(rule_name="type_decl")
#       │       ├── ASTNode(rule_name="token", is_leaf=1, token={type=INTEGER, ...})
#       │       └── ASTNode(rule_name="ident_list")
#       │           └── ASTNode(rule_name="token", is_leaf=1, token={type=IDENT, value="x"})
#       ├── ASTNode(rule_name="token", is_leaf=1, token={type=SEMICOLON, ...})
#       ├── ASTNode(rule_name="statement")
#       │   └── ASTNode(rule_name="assign_stmt")
#       │       ├── ASTNode(rule_name="left_part")
#       │       │   ├── ASTNode(rule_name="token", ..., token={type=IDENT, value="x"})
#       │       │   └── ASTNode(rule_name="token", ..., token={type=ASSIGN, value=":="})
#       │       └── ASTNode(rule_name="expression")
#       │           └── ASTNode(rule_name="arith_expr")
#       │               └── ...
#       └── ASTNode(rule_name="token", is_leaf=1, token={type=END, value="end"})
#
# # Why blessed hashrefs?
#
# We use Perl's `bless` mechanism to attach method dispatch to plain hashrefs.
# This is the idiomatic Perl OOP pattern for lightweight objects.  The
# alternative — using Moose or Moo — would add a heavy dependency for a
# simple data structure.

use strict;
use warnings;

our $VERSION = '0.01';

# --- new(%args) ---------------------------------------------------------------
#
# Create a new ASTNode.
#
# Required key:
#   rule_name (string) — the grammar rule or "token" for leaves
#
# Optional keys:
#   children  (arrayref) — child ASTNodes; defaults to []
#   is_leaf   (boolean)  — 1 for token-wrapping leaf nodes; defaults to 0
#   token     (hashref)  — the wrapped token; only valid when is_leaf is 1

sub new {
    my ($class, %args) = @_;
    $args{children} //= [];
    $args{is_leaf}  //= 0;
    return bless \%args, $class;
}

# --- rule_name() --------------------------------------------------------------
#
# Return the grammar rule name for this node.
# Examples: "program", "block", "statement", "assign_stmt", "token"

sub rule_name { $_[0]->{rule_name} }

# --- children() ---------------------------------------------------------------
#
# Return the arrayref of child ASTNodes.
# For leaf nodes this is always an empty arrayref.

sub children { $_[0]->{children} || [] }

# --- is_leaf() ----------------------------------------------------------------
#
# Return 1 if this node wraps a single token, 0 otherwise.

sub is_leaf { $_[0]->{is_leaf} || 0 }

# --- token() ------------------------------------------------------------------
#
# Return the wrapped token hashref (only meaningful when is_leaf() is true).
# Token fields: type, value, line, col.

sub token { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::AlgolParser::ASTNode - Abstract Syntax Tree node for the ALGOL 60 parser

=head1 SYNOPSIS

    use CodingAdventures::AlgolParser::ASTNode;

    # Internal rule node
    my $node = CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'block',
        children  => [$child],
    );

    # Leaf (token-wrapping) node
    my $leaf = CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'INTEGER_LIT', value => '42', line => 1, col => 1 },
    );

    print $node->rule_name;       # "block"
    print $leaf->is_leaf;         # 1
    print $leaf->token->{type};   # "INTEGER_LIT"

=head1 DESCRIPTION

A simple blessed-hashref AST node produced by L<CodingAdventures::AlgolParser>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
