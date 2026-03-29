package CodingAdventures::LatticeParser::ASTNode;

# ============================================================================
# CodingAdventures::LatticeParser::ASTNode — AST node for the Lattice parser
# ============================================================================
#
# An ASTNode represents one node in the Abstract Syntax Tree produced by the
# Lattice parser.  The structure mirrors CodingAdventures::TomlParser::ASTNode
# and CodingAdventures::JsonParser::ASTNode for consistency across the stack.
#
# Internal rule nodes (rule_name matches a grammar production):
#   rule_name => "stylesheet" | "rule" | "lattice_rule" | "qualified_rule"
#             | "variable_declaration" | "mixin_definition" | "mixin_params"
#             | "mixin_param" | "mixin_value_list" | "mixin_value"
#             | "include_directive" | "include_args" | "include_arg"
#             | "lattice_control" | "if_directive" | "for_directive"
#             | "each_directive" | "while_directive" | "each_list"
#             | "lattice_expression" | "lattice_or_expr" | "lattice_and_expr"
#             | "lattice_comparison" | "comparison_op"
#             | "lattice_additive" | "lattice_multiplicative"
#             | "lattice_unary" | "lattice_primary"
#             | "map_literal" | "map_entry"
#             | "function_definition" | "function_body" | "function_body_item"
#             | "return_directive" | "use_directive"
#             | "at_rule" | "at_prelude" | "at_prelude_token"
#             | "selector_list" | "complex_selector" | "compound_selector"
#             | "simple_selector" | "subclass_selector"
#             | "class_selector" | "id_selector" | "attribute_selector"
#             | "pseudo_class" | "pseudo_element" | "placeholder_selector"
#             | "block" | "block_contents" | "block_item"
#             | "lattice_block_item"
#             | "content_directive" | "extend_directive" | "at_root_directive"
#             | "declaration_or_nested" | "declaration"
#             | "property" | "priority"
#             | "value_list" | "value" | "function_call" | "function_args"
#   is_leaf   => 0
#   children  => arrayref of ASTNodes
#
# Leaf nodes (token wrappers):
#   rule_name => "token"
#   is_leaf   => 1
#   token     => hashref { type, value, line, col }
#   children  => []
#
# # Example: $primary: #4a90d9;
#
#   ASTNode(stylesheet)
#   └── ASTNode(rule)
#       └── ASTNode(lattice_rule)
#           └── ASTNode(variable_declaration)
#               ├── Leaf(VARIABLE "$primary")
#               ├── Leaf(COLON ":")
#               ├── ASTNode(value_list)
#               │   └── ASTNode(value)
#               │       └── Leaf(HASH "#4a90d9")
#               └── Leaf(SEMICOLON ";")

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
sub is_leaf   { $_[0]->{is_leaf}  || 0  }
sub token     { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::LatticeParser::ASTNode - Abstract Syntax Tree node for the Lattice parser

=head1 SYNOPSIS

    my $node = CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => 'variable_declaration',
        children  => [$var_leaf, $colon_leaf, $value_node, $semi_leaf],
    );

    my $leaf = CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'VARIABLE', value => '$primary', line => 1, col => 1 },
    );

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
