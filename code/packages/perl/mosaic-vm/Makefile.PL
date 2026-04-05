use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::MosaicVm',
    VERSION_FROM     => 'lib/CodingAdventures/MosaicVm.pm',
    ABSTRACT         => 'Generic tree walker that drives Mosaic compiler backends',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::MosaicAnalyzer' => 0,
    },
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
