package CodingAdventures::CompilerSourceMap;

# ============================================================================
# CodingAdventures::CompilerSourceMap — Source map chain for the AOT pipeline
# ============================================================================
#
# This package provides the source-mapping "sidecar" that flows through
# every stage of the AOT compiler pipeline.  It is a Perl port of the Go
# package at code/packages/go/compiler-source-map/.
#
# ## Why a chain instead of a flat table?
#
# A flat table (machine-code offset → source position) works for the final
# consumer — a debugger, profiler, or error reporter.  But it doesn't help
# when you're debugging the *compiler itself*:
#
#   - "Why did the optimiser delete instruction #42?"
#     → Look at the IrToIr segment for that pass.
#
#   - "Which AST node produced this IR instruction?"
#     → Look at AstToIr.
#
#   - "The machine code for this instruction seems wrong — what IR produced it?"
#     → Look at IrToMachineCode in reverse.
#
# The chain makes the compiler pipeline **transparent and debuggable at
# every stage**.
#
# ## Segment overview
#
#   Segment 1: SourceToAst      — source text position → AST node ID
#   Segment 2: AstToIr          — AST node ID → IR instruction IDs
#   Segment 3: IrToIr           — IR instruction ID → optimised IR IDs
#                                  (one segment per optimiser pass)
#   Segment 4: IrToMachineCode  — IR instruction ID → MC byte offset+length
#
#   Composite: source position ↔ machine code offset  (forward and reverse)
#
# ## Modules
#
#   SourcePosition    — a span of characters in a source file
#   SourceToAstEntry  — one source position → AST node ID mapping
#   SourceToAst       — Segment 1 container
#   AstToIrEntry      — one AST node → IR instruction IDs mapping
#   AstToIr           — Segment 2 container
#   IrToIrEntry       — one original IR ID → new IR IDs mapping
#   IrToIr            — Segment 3 container (one per optimiser pass)
#   IrToMachineCodeEntry — one IR ID → MC offset+length mapping
#   IrToMachineCode   — Segment 4 container
#   SourceMapChain    — the full pipeline sidecar
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::CompilerSourceMap::SourcePosition;
use CodingAdventures::CompilerSourceMap::SourceToAst;
use CodingAdventures::CompilerSourceMap::AstToIr;
use CodingAdventures::CompilerSourceMap::IrToIr;
use CodingAdventures::CompilerSourceMap::IrToMachineCode;
use CodingAdventures::CompilerSourceMap::SourceMapChain;

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap - Source map chain for the AOT compiler

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::CompilerSourceMap;
  use CodingAdventures::CompilerSourceMap::SourceMapChain;
  use CodingAdventures::CompilerSourceMap::SourcePosition;

  my $chain = CodingAdventures::CompilerSourceMap::SourceMapChain->new_chain;

  # Frontend fills segment 1 and 2
  $chain->{source_to_ast}->add(
      CodingAdventures::CompilerSourceMap::SourcePosition->new(
          file => 'hello.bf', line => 1, column => 1, length => 1
      ),
      42   # AST node ID
  );

=head1 DESCRIPTION

Perl port of the Go C<compiler-source-map> package.  Provides the full
source-map chain that flows through every stage of the AOT compiler pipeline.

=head1 LICENSE

MIT

=cut
