package CodingAdventures::VerilogParser::ASTNode;

# ============================================================================
# CodingAdventures::VerilogParser::ASTNode — AST node for the Verilog parser
# ============================================================================
#
# Every node in the Verilog Abstract Syntax Tree is an instance of this class.
# There are two kinds of nodes:
#
#   Leaf nodes   — wrap a single token from the lexer.
#                  `is_leaf` returns 1, `token` returns the token hashref.
#
#   Inner nodes  — produced by grammar rules like module_declaration, always_construct.
#                  `is_leaf` returns 0, `children` holds an arrayref of
#                  child nodes (which may themselves be leaf or inner nodes).
#
# # Node format
#
# Inner node:
#
#   {
#     rule_name => "module_declaration",  # the grammar rule
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
#     token     => { type => "MODULE", value => "module", line => 1, col => 1 },
#   }
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

sub new {
    my ($class, $rule_name, $children) = @_;
    $children //= [];
    return bless {
        rule_name => $rule_name,
        children  => $children,
        is_leaf   => 0,
    }, $class;
}

sub new_leaf {
    my ($class, $token) = @_;
    return bless {
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $token,
    }, $class;
}

sub rule_name { $_[0]->{rule_name} }
sub children  { $_[0]->{children} }
sub is_leaf   { $_[0]->{is_leaf} }
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::VerilogParser::ASTNode - AST node for the Verilog parser

=head1 SYNOPSIS

    my $node = CodingAdventures::VerilogParser::ASTNode->new('module_declaration', []);
    my $leaf = CodingAdventures::VerilogParser::ASTNode->new_leaf($token);

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
