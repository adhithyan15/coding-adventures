use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::ARM1Gatelevel',
    VERSION_FROM     => 'lib/CodingAdventures/ARM1Gatelevel.pm',
    ABSTRACT         => 'ARM1 gate-level simulator — every ALU and barrel-shift operation routes through logic gate calls',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::LogicGates'    => 0,
        'CodingAdventures::Arithmetic'    => 0,
        'CodingAdventures::ARM1Simulator' => 0,
    },
    TEST_REQUIRES => {
        'Test2::V0' => 0,
    },
    META_MERGE => {
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
