package CodingAdventures::FPGA;

# CodingAdventures::FPGA — FPGA (Field-Programmable Gate Array) simulation.
#
# This top-level module provides an overview and re-exports all sub-modules
# via convenient accessors.
#
# Key insight: a truth table is a program.
#
# A 4-input LUT loaded with [0,0,0,0, 0,0,0,1,...] is an AND gate.
# Load it with [0,1,1,0, 1,0,0,1,...] and it becomes an XOR gate.
# Same silicon, different function. This is what makes FPGAs programmable.
#
# Hierarchy:
#   LUT          — truth table lookup (the atom of programmable logic)
#   Slice        — 2 LUTs + 2 flip-flops + carry chain
#   CLB          — 2 slices (Configurable Logic Block)
#   SwitchMatrix — programmable routing crossbar
#   IOBlock      — external pin interface
#   Fabric       — complete FPGA (CLB grid + routing + I/O)
#   Bitstream    — configuration data parser
#
# Usage:
#
#   use CodingAdventures::FPGA;
#   # Sub-modules are auto-loaded; use them directly:
#   use CodingAdventures::FPGA::LUT;
#   use CodingAdventures::FPGA::Slice;
#   use CodingAdventures::FPGA::CLB;
#   use CodingAdventures::FPGA::SwitchMatrix;
#   use CodingAdventures::FPGA::IOBlock;
#   use CodingAdventures::FPGA::Fabric;
#   use CodingAdventures::FPGA::Bitstream;

use strict;
use warnings;

use CodingAdventures::FPGA::LUT;
use CodingAdventures::FPGA::Slice;
use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;
use CodingAdventures::FPGA::Fabric;
use CodingAdventures::FPGA::Bitstream;

our $VERSION = '0.1.0';

1;

__END__

=head1 NAME

CodingAdventures::FPGA - FPGA simulation library

=head1 DESCRIPTION

Models the key FPGA components: LUT (truth-table lookup), Slice (2 LUTs +
2 flip-flops + carry chain), CLB (2 slices), SwitchMatrix (programmable
routing crossbar), IOBlock (external pin interface), Fabric (complete FPGA
top-level), and Bitstream (configuration data parser).

=head1 SEE ALSO

=over 4

=item L<CodingAdventures::FPGA::LUT>

=item L<CodingAdventures::FPGA::Slice>

=item L<CodingAdventures::FPGA::CLB>

=item L<CodingAdventures::FPGA::SwitchMatrix>

=item L<CodingAdventures::FPGA::IOBlock>

=item L<CodingAdventures::FPGA::Fabric>

=item L<CodingAdventures::FPGA::Bitstream>

=back

=cut
