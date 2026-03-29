package CodingAdventures::CssParser::ASTNode;

# ============================================================================
# CodingAdventures::CssParser::ASTNode — AST node for the CSS parser
# ============================================================================
#
# An ASTNode represents one node in the Abstract Syntax Tree produced by the
# CSS parser. The structure mirrors CodingAdventures::TomlParser::ASTNode.
#
# Internal rule nodes (rule_name matches a grammar rule):
#   rule_name => "stylesheet" | "rule" | "at_rule" | "at_prelude"
#             | "qualified_rule" | "selector_list" | "complex_selector"
#             | "compound_selector" | "simple_selector" | "subclass_selector"
#             | "class_selector" | "id_selector" | "attribute_selector"
#             | "pseudo_class" | "pseudo_class_args" | "pseudo_element"
#             | "block" | "block_contents" | "block_item"
#             | "declaration_or_nested" | "declaration" | "property"
#             | "priority" | "value_list" | "value"
#             | "function_call" | "function_args"
#   is_leaf   => 0
#   children  => arrayref of ASTNodes
#
# Leaf nodes (token wrappers):
#   rule_name => "token"
#   is_leaf   => 1
#   token     => hashref { type, value, line, col }
#   children  => []
#
# # Example: h1 { color: red; }
#
#   ASTNode(stylesheet)
#   └── ASTNode(rule)
#       └── ASTNode(qualified_rule)
#           ├── ASTNode(selector_list)
#           │   └── ASTNode(complex_selector)
#           │       └── ASTNode(compound_selector)
#           │           └── ASTNode(simple_selector)
#           │               └── Leaf(IDENT "h1")
#           └── ASTNode(block)
#               ├── Leaf(LBRACE "{")
#               ├── ASTNode(block_contents)
#               │   └── ASTNode(block_item)
#               │       └── ASTNode(declaration_or_nested)
#               │           └── ASTNode(declaration)
#               │               ├── ASTNode(property)
#               │               │   └── Leaf(IDENT "color")
#               │               ├── Leaf(COLON ":")
#               │               ├── ASTNode(value_list)
#               │               │   └── ASTNode(value)
#               │               │       └── Leaf(IDENT "red")
#               │               └── Leaf(SEMICOLON ";")
#               └── Leaf(RBRACE "}")

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

CodingAdventures::CssParser::ASTNode - Abstract Syntax Tree node for the CSS parser

=head1 SYNOPSIS

    my $node = CodingAdventures::CssParser::ASTNode->new(
        rule_name => 'declaration',
        children  => [$prop_node, $colon_leaf, $value_list_node, $semi_leaf],
    );

    my $leaf = CodingAdventures::CssParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'IDENT', value => 'color', line => 1, col => 1 },
    );

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
