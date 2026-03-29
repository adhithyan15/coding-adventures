package CodingAdventures::TomlParser::ASTNode;

# ============================================================================
# CodingAdventures::TomlParser::ASTNode — AST node for the TOML parser
# ============================================================================
#
# An ASTNode represents one node in the Abstract Syntax Tree produced by the
# TOML parser.  The structure mirrors CodingAdventures::JsonParser::ASTNode.
#
# Internal rule nodes (rule_name matches a grammar rule):
#   rule_name => "document" | "expression" | "keyval" | "key" | "simple_key"
#             | "table_header" | "array_table_header" | "value"
#             | "array" | "array_values" | "inline_table"
#   is_leaf   => 0
#   children  => arrayref of ASTNodes
#
# Leaf nodes (token wrappers):
#   rule_name => "token"
#   is_leaf   => 1
#   token     => hashref { type, value, line, col }
#   children  => []
#
# # Example: key = 42
#
#   ASTNode(document)
#   └── ASTNode(expression)
#       └── ASTNode(keyval)
#           ├── ASTNode(key)
#           │   └── ASTNode(simple_key)
#           │       └── Leaf(BARE_KEY "key")
#           ├── Leaf(EQUALS "=")
#           └── ASTNode(value)
#               └── Leaf(INTEGER "42")

use strict;
use warnings;

our $VERSION = '0.01';

# --- new(%args) ---------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    $args{children} //= [];
    $args{is_leaf}  //= 0;
    return bless \%args, $class;
}

sub rule_name { $_[0]->{rule_name} }
sub children  { $_[0]->{children} || [] }
sub is_leaf   { $_[0]->{is_leaf} || 0 }
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::TomlParser::ASTNode - Abstract Syntax Tree node for the TOML parser

=head1 SYNOPSIS

    my $node = CodingAdventures::TomlParser::ASTNode->new(
        rule_name => 'keyval',
        children  => [$key_node, $eq_leaf, $value_node],
    );

    my $leaf = CodingAdventures::TomlParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'INTEGER', value => '42', line => 1, col => 7 },
    );

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
