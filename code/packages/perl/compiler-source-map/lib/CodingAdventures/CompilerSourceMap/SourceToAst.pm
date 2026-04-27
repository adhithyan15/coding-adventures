package CodingAdventures::CompilerSourceMap::SourceToAst;

# ============================================================================
# SourceToAst — Segment 1: source text positions → AST node IDs
# ============================================================================
#
# This segment is produced by the parser or by the language-specific
# frontend (e.g., brainfuck-ir-compiler).  It maps every meaningful
# source position to the AST node that represents it.
#
# ## Example
#
# The "+" character at line 1, column 3 of "hello.bf" maps to AST node #42
# (which is a command(INC) node in the parse tree).
#
# ## Entries
#
# Each entry is a hashref:
#   { pos => SourcePosition object, ast_node_id => integer }
#
# ## Lookup
#
#   lookup_by_node_id($id)  — returns the SourcePosition for a given AST node ID
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new() — create an empty SourceToAst segment.
sub new {
    my ($class) = @_;
    return bless { entries => [] }, $class;
}

# add($pos, $ast_node_id) — record a source position → AST node mapping.
#
# $pos must be a SourcePosition object.
# $ast_node_id is an integer.
sub add {
    my ($self, $pos, $ast_node_id) = @_;
    push @{ $self->{entries} }, {
        pos         => $pos,
        ast_node_id => $ast_node_id,
    };
}

# lookup_by_node_id($ast_node_id) — return the SourcePosition for a node.
#
# Returns the SourcePosition object if found, or undef if not found.
# (Used for reverse lookups when tracing from machine code back to source.)
sub lookup_by_node_id {
    my ($self, $ast_node_id) = @_;
    for my $entry (@{ $self->{entries} }) {
        if ($entry->{ast_node_id} == $ast_node_id) {
            return $entry->{pos};
        }
    }
    return undef;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::SourceToAst - Segment 1: source positions to AST node IDs

=head1 SYNOPSIS

  my $s2a = CodingAdventures::CompilerSourceMap::SourceToAst->new;
  $s2a->add($pos, 42);
  my $found_pos = $s2a->lookup_by_node_id(42);

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
