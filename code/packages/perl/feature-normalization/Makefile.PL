use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::FeatureNormalization',
    VERSION_FROM     => 'lib/CodingAdventures/FeatureNormalization.pm',
    ABSTRACT         => 'Feature scaling utilities for machine learning examples',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
                web  => 'https://github.com/adhithyan15/coding-adventures',
            },
        },
    },
);
