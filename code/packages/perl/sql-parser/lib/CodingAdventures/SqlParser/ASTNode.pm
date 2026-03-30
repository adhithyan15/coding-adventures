package CodingAdventures::SqlParser::ASTNode;

# ============================================================================
# CodingAdventures::SqlParser::ASTNode — AST node for the SQL parser
# ============================================================================
#
# Every node in the SQL Abstract Syntax Tree is represented as an instance
# of this class.  There are two kinds of nodes:
#
#   Leaf nodes   — wrap a single token from the lexer.
#                  `is_leaf` returns 1, `token` returns the token hashref.
#
#   Inner nodes  — produced by grammar rules like select_stmt, where_clause.
#                  `is_leaf` returns 0, `children` holds an arrayref of
#                  child nodes (which may themselves be leaf or inner nodes).
#
# # Node structure
#
#   {
#     rule_name => "select_stmt",    # string — the grammar rule that made this
#     children  => [ $node, ... ],   # arrayref — child ASTNode objects
#     is_leaf   => 0,                # 0 = inner node
#   }
#
# For leaf nodes:
#
#   {
#     rule_name => "token",          # always "token" for leaves
#     children  => [],
#     is_leaf   => 1,
#     token     => { type => "SELECT", value => "SELECT", line => 1, col => 1 },
#   }
#
# # Design rationale
#
# Keeping the leaf/inner distinction explicit makes tree-walking code simple:
#
#   if ($node->is_leaf) {
#       print $node->token->{value};
#   } else {
#       for my $child (@{ $node->children }) {
#           # recurse
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
# Create a leaf AST node that wraps a single token.
#
# Arguments:
#   $token  hashref  — a token as produced by CodingAdventures::SqlLexer

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

# Return the children arrayref (may be empty for leaves).
sub children  { $_[0]->{children} }

# Return 1 if this is a leaf node, 0 otherwise.
sub is_leaf   { $_[0]->{is_leaf} }

# Return the wrapped token (only valid when is_leaf() returns 1).
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::SqlParser::ASTNode - AST node for the SQL parser

=head1 SYNOPSIS

    use CodingAdventures::SqlParser::ASTNode;

    # Inner node
    my $node = CodingAdventures::SqlParser::ASTNode->new('select_stmt', \@children);
    print $node->rule_name;   # "select_stmt"
    print $node->is_leaf;     # 0

    # Leaf node
    my $leaf = CodingAdventures::SqlParser::ASTNode->new_leaf($token);
    print $leaf->is_leaf;     # 1
    print $leaf->token->{value};

=head1 DESCRIPTION

Lightweight AST node class used by C<CodingAdventures::SqlParser>.  Nodes are
either inner nodes (produced by grammar rules) or leaf nodes (wrapping a single
lexer token).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
