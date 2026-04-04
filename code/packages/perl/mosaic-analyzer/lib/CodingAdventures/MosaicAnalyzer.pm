package CodingAdventures::MosaicAnalyzer;

# ============================================================================
# CodingAdventures::MosaicAnalyzer — Validates the Mosaic AST and produces a typed MosaicIR
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::MosaicAnalyzer;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::MosaicParser;
use CodingAdventures::MosaicLexer;
use CodingAdventures::GrammarTools;
use CodingAdventures::Lexer;
use CodingAdventures::DirectedGraph;
use CodingAdventures::Parser;
use CodingAdventures::StateMachine;

# TODO: Implement MosaicAnalyzer

1;

__END__

=head1 NAME

CodingAdventures::MosaicAnalyzer - Validates the Mosaic AST and produces a typed MosaicIR

=head1 SYNOPSIS

    use CodingAdventures::MosaicAnalyzer;

=head1 DESCRIPTION

Validates the Mosaic AST and produces a typed MosaicIR

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
