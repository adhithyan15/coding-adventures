package CodingAdventures::VhdlParser::ASTNode;

# ============================================================================
# CodingAdventures::VhdlParser::ASTNode — AST node for the VHDL parser
# ============================================================================
#
# Every node in the VHDL Abstract Syntax Tree is an instance of this class.
# There are two kinds of nodes:
#
#   Leaf nodes   — wrap a single token from the lexer.
#                  `is_leaf` returns 1, `token` returns the token hashref.
#
#   Inner nodes  — produced by grammar rules like entity_declaration,
#                  architecture_body, process_statement, etc.
#                  `is_leaf` returns 0, `children` holds an arrayref of
#                  child nodes (which may themselves be leaf or inner nodes).
#
# # VHDL design structure
#
# VHDL programs are organized into "design units".  The two most important
# are:
#
#   entity — declares the external interface (ports, generics) of a component
#   architecture — describes the internal behavior or structure
#
# A typical VHDL source file looks like:
#
#   library ieee;
#   use ieee.std_logic_1164.all;
#
#   entity my_adder is
#     port (a, b : in std_logic; sum : out std_logic);
#   end entity my_adder;
#
#   architecture rtl of my_adder is
#   begin
#     sum <= a xor b;
#   end architecture rtl;
#
# The AST captures this structure faithfully.  The root node is `design_file`,
# which contains one or more `design_unit` children.
#
# # Node format
#
# Inner node:
#
#   {
#     rule_name => "entity_declaration",
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
#     token     => { type => "ENTITY", value => "entity", line => 3, col => 1 },
#   }
#
# # Tree walking pattern
#
#   sub walk {
#       my ($node, $depth) = @_;
#       $depth //= 0;
#       my $indent = "  " x $depth;
#       if ($node->is_leaf) {
#           printf "%s[%s %s]\n", $indent, $node->token->{type},
#                                           $node->token->{value};
#       } else {
#           printf "%s(%s\n", $indent, $node->rule_name;
#           walk($_, $depth + 1) for @{ $node->children };
#           printf "%s)\n", $indent;
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
#   $token  hashref  — a token as produced by CodingAdventures::VhdlLexer
#                      { type => 'ENTITY', value => 'entity', line => 1, col => 1 }

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

CodingAdventures::VhdlParser::ASTNode - AST node for the VHDL parser

=head1 SYNOPSIS

    use CodingAdventures::VhdlParser::ASTNode;

    # Inner node
    my $node = CodingAdventures::VhdlParser::ASTNode->new('entity_declaration', \@children);
    print $node->rule_name;   # "entity_declaration"
    print $node->is_leaf;     # 0

    # Leaf node
    my $leaf = CodingAdventures::VhdlParser::ASTNode->new_leaf($token);
    print $leaf->is_leaf;         # 1
    print $leaf->token->{value};  # e.g. "entity"

=head1 DESCRIPTION

Lightweight AST node class used by C<CodingAdventures::VhdlParser>.
Nodes are either inner nodes (produced by grammar rules) or leaf nodes
(wrapping a single lexer token).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
