package CodingAdventures::CompilerSourceMap::AstToIr;

# ============================================================================
# AstToIr — Segment 2: AST node IDs → IR instruction IDs
# ============================================================================
#
# A single AST node often produces multiple IR instructions.  For example,
# a Brainfuck "+" command produces four IR instructions:
#   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
#
# So the mapping is one-to-many: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].
#
# ## Entries
#
# Each entry is a hashref:
#   { ast_node_id => integer, ir_ids => arrayref of integers }
#
# ## Lookups
#
#   lookup_by_ast_node_id($id)  — get the IR IDs for an AST node
#   lookup_by_ir_id($id)        — get the AST node that produced a given IR ID
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new() — create an empty AstToIr segment.
sub new {
    my ($class) = @_;
    return bless { entries => [] }, $class;
}

# add($ast_node_id, $ir_ids_arrayref) — record an AST node → IR ID mapping.
#
# $ast_node_id is an integer.
# $ir_ids_arrayref is an arrayref of IR instruction ID integers.
sub add {
    my ($self, $ast_node_id, $ir_ids) = @_;
    push @{ $self->{entries} }, {
        ast_node_id => $ast_node_id,
        ir_ids      => $ir_ids,
    };
}

# lookup_by_ast_node_id($ast_node_id) — return IR IDs for a given AST node.
#
# Returns an arrayref if found, or undef if not found.
sub lookup_by_ast_node_id {
    my ($self, $ast_node_id) = @_;
    for my $entry (@{ $self->{entries} }) {
        if ($entry->{ast_node_id} == $ast_node_id) {
            return $entry->{ir_ids};
        }
    }
    return undef;
}

# lookup_by_ir_id($ir_id) — return the AST node ID that produced this IR ID.
#
# Returns an integer if found, or -1 if not found.
# Used for reverse lookups when tracing from IR back to source.
sub lookup_by_ir_id {
    my ($self, $ir_id) = @_;
    for my $entry (@{ $self->{entries} }) {
        for my $id (@{ $entry->{ir_ids} }) {
            return $entry->{ast_node_id} if $id == $ir_id;
        }
    }
    return -1;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::AstToIr - Segment 2: AST node IDs to IR instruction IDs

=head1 SYNOPSIS

  my $a2i = CodingAdventures::CompilerSourceMap::AstToIr->new;
  $a2i->add(42, [7, 8, 9, 10]);
  my $ir_ids  = $a2i->lookup_by_ast_node_id(42);  # [7, 8, 9, 10]
  my $node_id = $a2i->lookup_by_ir_id(8);         # 42

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
