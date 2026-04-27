package CodingAdventures::BrainfuckIrCompiler;

# ============================================================================
# CodingAdventures::BrainfuckIrCompiler — Brainfuck AOT compiler frontend
# ============================================================================
#
# This package compiles Brainfuck ASTs into the general-purpose IR defined
# by CodingAdventures::CompilerIr.  It is the Perl port of the Go package
# at code/packages/go/brainfuck-ir-compiler/.
#
# ## Pipeline position
#
#   Brainfuck source
#        ↓  CodingAdventures::Brainfuck::Parser
#      AST
#        ↓  CodingAdventures::BrainfuckIrCompiler  ← this package
#      IR + SourceMapChain (Segments 1 & 2)
#        ↓  codegen-riscv  (future)
#   RISC-V machine code
#
# ## Quick start
#
#   use CodingAdventures::BrainfuckIrCompiler qw(compile);
#   use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
#   use CodingAdventures::Brainfuck::Parser;
#   use CodingAdventures::CompilerIr qw(print_ir);
#
#   my $ast    = CodingAdventures::Brainfuck::Parser->parse('++[-].');
#   my $cfg    = CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;
#   my $result = compile($ast, 'hello.bf', $cfg);
#
#   print print_ir($result->{program});
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(compile);

use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
use CodingAdventures::BrainfuckIrCompiler::Compiler qw(compile);

1;

__END__

=head1 NAME

CodingAdventures::BrainfuckIrCompiler - Brainfuck AOT compiler frontend (AST → IR)

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::BrainfuckIrCompiler qw(compile);
  use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
  use CodingAdventures::Brainfuck::Parser;
  use CodingAdventures::CompilerIr qw(print_ir);

  my $ast    = CodingAdventures::Brainfuck::Parser->parse('++[-].');
  my $cfg    = CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;
  my $result = compile($ast, 'hello.bf', $cfg);

  print print_ir($result->{program});

=head1 DESCRIPTION

Perl port of the Go C<brainfuck-ir-compiler> package.  Compiles a Brainfuck
AST (from C<CodingAdventures::Brainfuck::Parser>) into IR instructions
(C<CodingAdventures::CompilerIr::IrProgram>) and populates the first two
segments of the source map chain.

=head1 LICENSE

MIT

=cut
