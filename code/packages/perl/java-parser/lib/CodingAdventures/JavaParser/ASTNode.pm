package CodingAdventures::JavaParser::ASTNode;

# ============================================================================
# CodingAdventures::JavaParser::ASTNode — AST node for the Java parser
# ============================================================================
#
# Every node in the Java Abstract Syntax Tree is an instance of this
# class.  There are two kinds of nodes:
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
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# --- new($rule_name, $children) -----------------------------------------------
#
# Create an inner (non-leaf) AST node.

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

CodingAdventures::JavaParser::ASTNode - AST node for the Java parser

=head1 SYNOPSIS

    use CodingAdventures::JavaParser::ASTNode;

    # Inner node
    my $node = CodingAdventures::JavaParser::ASTNode->new('if_stmt', \@children);
    print $node->rule_name;   # "if_stmt"
    print $node->is_leaf;     # 0

    # Leaf node
    my $leaf = CodingAdventures::JavaParser::ASTNode->new_leaf($token);
    print $leaf->is_leaf;         # 1
    print $leaf->token->{value};  # e.g. "int"

=head1 DESCRIPTION

Lightweight AST node class used by C<CodingAdventures::JavaParser>.
Nodes are either inner nodes (produced by grammar rules) or leaf nodes
(wrapping a single lexer token).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
