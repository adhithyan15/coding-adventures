use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::FPGA; 1 },               'CodingAdventures::FPGA loads' );
ok( eval { require CodingAdventures::FPGA::LUT; 1 },          'CodingAdventures::FPGA::LUT loads' );
ok( eval { require CodingAdventures::FPGA::Slice; 1 },        'CodingAdventures::FPGA::Slice loads' );
ok( eval { require CodingAdventures::FPGA::CLB; 1 },          'CodingAdventures::FPGA::CLB loads' );
ok( eval { require CodingAdventures::FPGA::SwitchMatrix; 1 }, 'CodingAdventures::FPGA::SwitchMatrix loads' );
ok( eval { require CodingAdventures::FPGA::IOBlock; 1 },      'CodingAdventures::FPGA::IOBlock loads' );
ok( eval { require CodingAdventures::FPGA::Fabric; 1 },       'CodingAdventures::FPGA::Fabric loads' );
ok( eval { require CodingAdventures::FPGA::Bitstream; 1 },    'CodingAdventures::FPGA::Bitstream loads' );

done_testing;
