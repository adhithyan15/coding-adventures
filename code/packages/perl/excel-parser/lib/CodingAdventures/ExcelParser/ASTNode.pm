package CodingAdventures::ExcelParser::ASTNode;

# ============================================================================
# CodingAdventures::ExcelParser::ASTNode — AST node for the Excel parser
# ============================================================================
#
# An ASTNode represents one node in the Abstract Syntax Tree produced by the
# Excel formula parser.  The structure mirrors CodingAdventures::TomlParser::ASTNode.
#
# # Internal rule nodes (rule_name matches a parser production):
#
#   rule_name => "formula" | "binop" | "unop" | "postfix" | "call"
#             | "range" | "ref_prefix" | "cell" | "number" | "string"
#             | "bool" | "error" | "name" | "array" | "group"
#   is_leaf   => 0
#   children  => arrayref of ASTNodes
#
# # Leaf nodes (token wrappers):
#
#   rule_name => "token"
#   is_leaf   => 1
#   token     => hashref { type, value, line, col }
#   children  => []
#
# # Additional fields per node kind:
#
#   binop:     left, right, op (all ASTNodes or token hashrefs)
#   unop:      operand, op
#   postfix:   operand, op
#   call:      name (token hashref), args (arrayref of ASTNodes)
#   range:     start_ref, end_ref (ASTNodes)
#   ref_prefix:prefix (token hashref), ref (ASTNode or undef)
#   cell:      token (hashref)
#   number:    token (hashref)
#   string:    token (hashref)
#   bool:      token (hashref)
#   error:     token (hashref)
#   name:      token (hashref)
#   array:     rows (arrayref of arrayrefs of ASTNodes)
#   group:     expr (ASTNode)
#   formula:   eq (token hashref or undef), body (ASTNode)
#
# # Example: =A1+B2
#
#   ASTNode(formula)
#     eq   → { type=>"EQUALS", value=>"=", ... }
#     body → ASTNode(binop)
#               op    → { type=>"PLUS", value=>"+" }
#               left  → ASTNode(cell) { token => { type=>"CELL", value=>"a1" } }
#               right → ASTNode(cell) { token => { type=>"CELL", value=>"b2" } }

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

CodingAdventures::ExcelParser::ASTNode - Abstract Syntax Tree node for the Excel parser

=head1 SYNOPSIS

    my $node = CodingAdventures::ExcelParser::ASTNode->new(
        rule_name => 'binop',
        op        => { type => 'PLUS', value => '+', line => 1, col => 4 },
        left      => $cell_a1,
        right     => $cell_b2,
    );

    my $leaf = CodingAdventures::ExcelParser::ASTNode->new(
        rule_name => 'cell',
        token     => { type => 'CELL', value => 'a1', line => 1, col => 2 },
    );

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
