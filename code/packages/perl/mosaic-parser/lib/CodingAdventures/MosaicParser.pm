package CodingAdventures::MosaicParser;

# ============================================================================
# CodingAdventures::MosaicParser — Parses Mosaic token stream into an ASTNode tree
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::MosaicParser;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::MosaicLexer;
use CodingAdventures::GrammarTools;
use CodingAdventures::Lexer;
use CodingAdventures::DirectedGraph;
use CodingAdventures::Parser;
use CodingAdventures::StateMachine;

# TODO: Implement MosaicParser

1;

__END__

=head1 NAME

CodingAdventures::MosaicParser - Parses Mosaic token stream into an ASTNode tree

=head1 SYNOPSIS

    use CodingAdventures::MosaicParser;

=head1 DESCRIPTION

Parses Mosaic token stream into an ASTNode tree

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
