use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::GpuCore; 1 },
    'CodingAdventures::GpuCore loads' );

ok( eval { require CodingAdventures::GpuCore::GPUCore; 1 },
    'GPUCore subpackage loads' );

ok( eval { require CodingAdventures::GpuCore::GenericISA; 1 },
    'GenericISA subpackage loads' );

ok( eval { require CodingAdventures::GpuCore::FPRegisterFile; 1 },
    'FPRegisterFile subpackage loads' );

ok( eval { require CodingAdventures::GpuCore::LocalMemory; 1 },
    'LocalMemory subpackage loads' );

ok( CodingAdventures::GpuCore->VERSION, 'GpuCore has a VERSION' );

done_testing;
