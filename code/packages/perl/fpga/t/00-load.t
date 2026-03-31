use strict;
use warnings;
use Test2::V0;

use_ok 'CodingAdventures::FPGA';
use_ok 'CodingAdventures::FPGA::LUT';
use_ok 'CodingAdventures::FPGA::Slice';
use_ok 'CodingAdventures::FPGA::CLB';
use_ok 'CodingAdventures::FPGA::SwitchMatrix';
use_ok 'CodingAdventures::FPGA::IOBlock';
use_ok 'CodingAdventures::FPGA::Fabric';
use_ok 'CodingAdventures::FPGA::Bitstream';

done_testing;
