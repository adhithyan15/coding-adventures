use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Hamurabi',
    VERSION_FROM     => 'lib/CodingAdventures/Hamurabi.pm',
    ABSTRACT         => 'Mainframe Turn-Based Resource Allocation Simulation',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'Math::BigInt' => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
        'JSON::PP'  => 0,
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
