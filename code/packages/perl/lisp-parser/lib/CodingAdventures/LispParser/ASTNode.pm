package CodingAdventures::LispParser::ASTNode;

# ============================================================================
# CodingAdventures::LispParser::ASTNode — AST node for the Lisp parser
# ============================================================================
#
# An ASTNode represents a single node in the Abstract Syntax Tree produced
# by the Lisp parser.  Each node corresponds to either:
#
#   - An internal rule node: produced by a grammar rule (program, sexpr, atom,
#     list, list_body, quoted).  Has a rule_name and zero or more children.
#
#   - A leaf node: wraps a single lexer token (NUMBER, SYMBOL, STRING, etc.).
#     Has is_leaf == 1 and a token hashref.
#
# # AST structure for (define x 42)
#
#   ASTNode(rule_name="program")
#   └── ASTNode(rule_name="sexpr")
#       └── ASTNode(rule_name="list")
#           ├── ASTNode(rule_name="token", is_leaf=1, token={type=LPAREN,  value="("})
#           ├── ASTNode(rule_name="list_body")
#           │   ├── ASTNode(rule_name="sexpr")
#           │   │   └── ASTNode(rule_name="atom")
#           │   │       └── ASTNode(is_leaf=1, token={type=SYMBOL, value="define"})
#           │   ├── ASTNode(rule_name="sexpr")
#           │   │   └── ASTNode(rule_name="atom")
#           │   │       └── ASTNode(is_leaf=1, token={type=SYMBOL, value="x"})
#           │   └── ASTNode(rule_name="sexpr")
#           │       └── ASTNode(rule_name="atom")
#           │           └── ASTNode(is_leaf=1, token={type=NUMBER, value="42"})
#           └── ASTNode(rule_name="token", is_leaf=1, token={type=RPAREN, value=")"})
#
# # Why blessed hashrefs?
#
# We use Perl's `bless` mechanism to attach method dispatch to plain hashrefs.
# This is the idiomatic Perl OOP pattern for lightweight objects.  The
# alternative — using Moose or Moo — would add a heavy dependency for a
# simple data structure.
#
# Construction:
#   CodingAdventures::LispParser::ASTNode->new(
#       rule_name => "program",
#       children  => [$child1, $child2],
#   )
#
#   CodingAdventures::LispParser::ASTNode->new(
#       rule_name => "token",
#       is_leaf   => 1,
#       token     => { type => "NUMBER", value => "42", line => 1, col => 8 },
#   )

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
# Examples: "program", "sexpr", "atom", "list", "list_body", "quoted", "token"

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

CodingAdventures::LispParser::ASTNode - Abstract Syntax Tree node for the Lisp parser

=head1 SYNOPSIS

    use CodingAdventures::LispParser::ASTNode;

    # Internal rule node
    my $node = CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'program',
        children  => [$child],
    );

    # Leaf (token-wrapping) node
    my $leaf = CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'NUMBER', value => '42', line => 1, col => 1 },
    );

    print $node->rule_name;       # "program"
    print $leaf->is_leaf;         # 1
    print $leaf->token->{type};   # "NUMBER"

=head1 DESCRIPTION

A simple blessed-hashref AST node produced by
L<CodingAdventures::LispParser>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
