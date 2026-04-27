package CodingAdventures::CompilerSourceMap::SourceMapChain;

# ============================================================================
# SourceMapChain — the full pipeline sidecar
# ============================================================================
#
# This is the central data structure that flows through every stage of the
# compiler pipeline.  Each stage reads the existing segments and appends
# its own:
#
#   1. Frontend (brainfuck-ir-compiler) → fills source_to_ast + ast_to_ir
#   2. Optimiser (compiler-ir-optimizer) → appends ir_to_ir segments
#   3. Backend (codegen-riscv) → fills ir_to_machine_code
#
# ## Fields
#
#   source_to_ast     — SourceToAst segment (Segment 1)
#   ast_to_ir         — AstToIr segment (Segment 2)
#   ir_to_ir          — arrayref of IrToIr segments (one per optimiser pass)
#   ir_to_machine_code — IrToMachineCode segment (undef until backend runs)
#
# ## Composite queries
#
#   source_to_mc($pos)   — trace source position → machine code offset(s)
#   mc_to_source($offset) — trace machine code offset → source position
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::CompilerSourceMap::SourceToAst;
use CodingAdventures::CompilerSourceMap::AstToIr;

# new_chain() — create an empty source map chain ready for use.
sub new_chain {
    my ($class) = @_;
    return bless {
        source_to_ast      => CodingAdventures::CompilerSourceMap::SourceToAst->new,
        ast_to_ir          => CodingAdventures::CompilerSourceMap::AstToIr->new,
        ir_to_ir           => [],
        ir_to_machine_code => undef,
    }, $class;
}

# add_optimizer_pass($ir_to_ir_segment) — append an IrToIr segment.
#
# Call once per optimiser pass in the order the passes were applied.
sub add_optimizer_pass {
    my ($self, $segment) = @_;
    push @{ $self->{ir_to_ir} }, $segment;
}

# ============================================================================
# Composite queries
# ============================================================================

# source_to_mc($pos) — trace a source position to machine code entries.
#
# Algorithm:
#   1. SourceToAst: source position → AST node ID
#   2. AstToIr: AST node ID → IR instruction IDs
#   3. IrToIr (each pass): follow IR IDs through each optimiser pass
#   4. IrToMachineCode: final IR IDs → machine code offsets
#
# Returns an arrayref of { ir_id, mc_offset, mc_length } hashrefs,
# or undef if the chain is incomplete or no mapping exists.
sub source_to_mc {
    my ($self, $pos) = @_;

    return undef unless defined $self->{ir_to_machine_code};

    # Step 1: source position → AST node ID
    my $ast_node_id = -1;
    for my $entry (@{ $self->{source_to_ast}{entries} }) {
        if ($entry->{pos}{file}   eq $pos->{file}
            && $entry->{pos}{line}   == $pos->{line}
            && $entry->{pos}{column} == $pos->{column})
        {
            $ast_node_id = $entry->{ast_node_id};
            last;
        }
    }
    return undef if $ast_node_id == -1;

    # Step 2: AST node ID → IR IDs
    my $ir_ids = $self->{ast_to_ir}->lookup_by_ast_node_id($ast_node_id);
    return undef unless defined $ir_ids;

    # Step 3: follow through optimiser passes
    my @current_ids = @$ir_ids;
    for my $pass (@{ $self->{ir_to_ir} }) {
        my @next_ids;
        for my $id (@current_ids) {
            next if $pass->{deleted}{$id};  # instruction was optimised away
            my $new_ids = $pass->lookup_by_original_id($id);
            push @next_ids, @$new_ids if defined $new_ids;
        }
        @current_ids = @next_ids;
    }
    return undef unless @current_ids;

    # Step 4: IR IDs → machine code entries
    my @results;
    for my $id (@current_ids) {
        my ($offset, $length) = $self->{ir_to_machine_code}->lookup_by_ir_id($id);
        if ($offset >= 0) {
            push @results, {
                ir_id     => $id,
                mc_offset => $offset,
                mc_length => $length,
            };
        }
    }
    return \@results;
}

# mc_to_source($mc_offset) — trace a machine code offset back to source.
#
# Algorithm (reverse of source_to_mc):
#   1. IrToMachineCode: MC offset → IR instruction ID
#   2. IrToIr (each pass, in reverse): follow IR ID back through passes
#   3. AstToIr: IR ID → AST node ID
#   4. SourceToAst: AST node ID → source position
#
# Returns a SourcePosition object, or undef if the chain is incomplete
# or no mapping exists.
sub mc_to_source {
    my ($self, $mc_offset) = @_;

    return undef unless defined $self->{ir_to_machine_code};

    # Step 1: MC offset → IR ID
    my $ir_id = $self->{ir_to_machine_code}->lookup_by_mc_offset($mc_offset);
    return undef if $ir_id == -1;

    # Step 2: follow back through optimiser passes (in reverse order)
    my $current_id = $ir_id;
    for my $pass (((reverse @{ $self->{ir_to_ir} }))) {
        my $orig = $pass->lookup_by_new_id($current_id);
        return undef if $orig == -1;
        $current_id = $orig;
    }

    # Step 3: IR ID → AST node ID
    my $ast_node_id = $self->{ast_to_ir}->lookup_by_ir_id($current_id);
    return undef if $ast_node_id == -1;

    # Step 4: AST node ID → source position
    return $self->{source_to_ast}->lookup_by_node_id($ast_node_id);
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::SourceMapChain - full pipeline source map sidecar

=head1 SYNOPSIS

  use CodingAdventures::CompilerSourceMap::SourceMapChain;
  use CodingAdventures::CompilerSourceMap::SourcePosition;

  my $chain = CodingAdventures::CompilerSourceMap::SourceMapChain->new_chain;

  # Frontend fills segments 1 and 2
  $chain->{source_to_ast}->add($pos, $ast_node_id);
  $chain->{ast_to_ir}->add($ast_node_id, \@ir_ids);

  # Optimiser appends a pass
  $chain->add_optimizer_pass($ir_to_ir_segment);

  # Backend fills segment 4
  $chain->{ir_to_machine_code} = $mc_segment;

  # Composite queries
  my $mc_entries = $chain->source_to_mc($pos);
  my $source_pos = $chain->mc_to_source(0x14);

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
