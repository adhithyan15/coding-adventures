package CodingAdventures::MosaicVm;

# ============================================================================
# CodingAdventures::MosaicVm — Generic tree walker that drives Mosaic compiler backends
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::MosaicVm;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::MosaicAnalyzer;
use CodingAdventures::MosaicParser;
use CodingAdventures::MosaicLexer;
use CodingAdventures::GrammarTools;
use CodingAdventures::Lexer;
use CodingAdventures::DirectedGraph;
use CodingAdventures::Parser;
use CodingAdventures::StateMachine;

# TODO: Implement MosaicVm

1;

__END__

=head1 NAME

CodingAdventures::MosaicVm - Generic tree walker that drives Mosaic compiler backends

=head1 SYNOPSIS

    use CodingAdventures::MosaicVm;

=head1 DESCRIPTION

Generic tree walker that drives Mosaic compiler backends

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
