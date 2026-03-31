package CodingAdventures::FPGA;
use strict;
use warnings;
our $VERSION = '0.01';

use CodingAdventures::FPGA::LUT;
use CodingAdventures::FPGA::Slice;
use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;
use CodingAdventures::FPGA::Bitstream;
use CodingAdventures::FPGA::Fabric;

1;

__END__

=head1 NAME

CodingAdventures::FPGA - FPGA fabric simulation

=head1 SYNOPSIS

  use CodingAdventures::FPGA::LUT;
  my $lut = CodingAdventures::FPGA::LUT->new(2);
  $lut->configure([0, 0, 0, 1]);  # AND
  print $lut->evaluate([1, 1]);   # 1

=head1 DESCRIPTION

FPGA fabric simulation: LUT, Slice, CLB, SwitchMatrix, IOBlock, Bitstream, Fabric.
