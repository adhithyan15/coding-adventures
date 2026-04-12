package CodingAdventures::CSharpParser::ASTNode;

# ============================================================================
# CodingAdventures::CSharpParser::ASTNode — AST node for the C# parser
# ============================================================================
#
# Every node in the C# Abstract Syntax Tree is an instance of this class.
# There are two kinds of nodes:
#
#   Leaf nodes   — wrap a single token from the lexer.
#                  `is_leaf` returns 1, `token` returns the token hashref.
#
#   Inner nodes  — produced by grammar rules like var_declaration, if_stmt.
#                  `is_leaf` returns 0, `children` holds an arrayref of
#                  child nodes (which may themselves be leaf or inner nodes).
#
# # Node format
#
# Inner node:
#
#   {
#     rule_name => "var_declaration",  # the grammar rule that produced this
#     children  => [ $node, ... ],
#     is_leaf   => 0,
#   }
#
# Leaf node:
#
#   {
#     rule_name => "token",
#     children  => [],
#     is_leaf   => 1,
#     token     => { type => "NAME", value => "int", line => 1, col => 1 },
#   }
#
# # Tree walking pattern
#
# The classic way to process an AST is to recurse depth-first:
#
#   sub walk {
#       my ($node) = @_;
#       if ($node->is_leaf) {
#           print $node->token->{value}, "\n";
#       } else {
#           print $node->rule_name, "\n";
#           walk($_) for @{ $node->children };
#       }
#   }
#
# # Why separate leaf and inner nodes?
#
# Leaf nodes hold source token metadata (line, col, original text).
# Inner nodes hold structural meaning (which grammar rule produced them).
# By keeping them distinct, tree-walking code can always ask `is_leaf`
# and know exactly what fields are available — no defensive checks needed.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# --- new($rule_name, $children) -----------------------------------------------
#
# Create an inner (non-leaf) AST node.
#
# $rule_name — a string identifying the grammar rule, e.g. "if_stmt".
# $children  — an arrayref of child ASTNode objects (default: []).

sub new {
    my ($class, $rule_name, $children) = @_;
    $children //= [];
    return bless {
        rule_name => $rule_name,
        children  => $children,
        is_leaf   => 0,
    }, $class;
}

# --- new_leaf($token) ---------------------------------------------------------
#
# Create a leaf AST node wrapping a single lexer token.
#
# $token — a hashref with keys: type, value, line, col.

sub new_leaf {
    my ($class, $token) = @_;
    return bless {
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $token,
    }, $class;
}

# --- Accessors ----------------------------------------------------------------

sub rule_name { $_[0]->{rule_name} }
sub children  { $_[0]->{children} }
sub is_leaf   { $_[0]->{is_leaf} }
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::CSharpParser::ASTNode - AST node for the C# parser

=head1 SYNOPSIS

    use CodingAdventures::CSharpParser::ASTNode;

    # Inner node
    my $node = CodingAdventures::CSharpParser::ASTNode->new('if_stmt', \@children);
    print $node->rule_name;   # "if_stmt"
    print $node->is_leaf;     # 0

    # Leaf node
    my $leaf = CodingAdventures::CSharpParser::ASTNode->new_leaf($token);
    print $leaf->is_leaf;         # 1
    print $leaf->token->{value};  # e.g. "int"

=head1 DESCRIPTION

Lightweight AST node class used by C<CodingAdventures::CSharpParser>.
Nodes are either inner nodes (produced by grammar rules) or leaf nodes
(wrapping a single lexer token).

=head1 METHODS

=head2 new($rule_name, $children)

Create an inner node. C<$children> defaults to C<[]>.

=head2 new_leaf($token)

Create a leaf node wrapping a lexer token hashref.

=head2 rule_name

Returns the grammar rule name string, e.g. C<"var_declaration">.
Leaf nodes return C<"token">.

=head2 children

Returns an arrayref of child ASTNode objects. Empty for leaf nodes.

=head2 is_leaf

Returns 1 for leaf nodes, 0 for inner nodes.

=head2 token

Returns the wrapped lexer token hashref (defined for leaf nodes only).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
