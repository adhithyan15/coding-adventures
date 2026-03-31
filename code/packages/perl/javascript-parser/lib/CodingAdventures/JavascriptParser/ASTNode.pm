package CodingAdventures::JavascriptParser::ASTNode;

# ============================================================================
# CodingAdventures::JavascriptParser::ASTNode — AST node for the JS parser
# ============================================================================
#
# Every node in the JavaScript Abstract Syntax Tree is an instance of this
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
#     token     => { type => "KEYWORD", value => "var", line => 1, col => 1 },
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
#
# Arguments:
#   $rule_name  string    — the grammar rule that produced this node
#   $children   arrayref  — child ASTNode objects (default: empty arrayref)

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
# Arguments:
#   $token  hashref  — a token as produced by CodingAdventures::JavascriptLexer

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

# Return the rule name string.
sub rule_name { $_[0]->{rule_name} }

# Return the children arrayref.
sub children  { $_[0]->{children} }

# Return 1 if this is a leaf node, 0 otherwise.
sub is_leaf   { $_[0]->{is_leaf} }

# Return the wrapped token (only valid when is_leaf() returns 1).
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::JavascriptParser::ASTNode - AST node for the JavaScript parser

=head1 SYNOPSIS

    use CodingAdventures::JavascriptParser::ASTNode;

    # Inner node
    my $node = CodingAdventures::JavascriptParser::ASTNode->new('if_stmt', \@children);
    print $node->rule_name;   # "if_stmt"
    print $node->is_leaf;     # 0

    # Leaf node
    my $leaf = CodingAdventures::JavascriptParser::ASTNode->new_leaf($token);
    print $leaf->is_leaf;         # 1
    print $leaf->token->{value};  # e.g. "var"

=head1 DESCRIPTION

Lightweight AST node class used by C<CodingAdventures::JavascriptParser>.
Nodes are either inner nodes (produced by grammar rules) or leaf nodes
(wrapping a single lexer token).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
