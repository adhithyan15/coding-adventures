use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::SingleLayerNetwork',
    VERSION_FROM     => 'lib/CodingAdventures/SingleLayerNetwork.pm',
    ABSTRACT         => 'Single-layer multi-input multi-output neural network primitives',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
);
