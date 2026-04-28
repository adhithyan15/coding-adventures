use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME => 'CodingAdventures::TwoLayerNetwork',
    VERSION_FROM => 'lib/CodingAdventures/TwoLayerNetwork.pm',
    ABSTRACT => 'Two-layer neural network primitives for hidden-layer examples',
    AUTHOR => 'Adhithya Rajasekaran',
    LICENSE => 'mit',
    PREREQ_PM => {},
    TEST_REQUIRES => { 'Test2::V0' => 0 },
);
