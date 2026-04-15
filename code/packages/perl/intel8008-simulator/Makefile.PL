use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Intel8008Simulator',
    AUTHOR           => 'Coding Adventures',
    VERSION_FROM     => 'lib/CodingAdventures/Intel8008Simulator.pm',
    ABSTRACT         => 'Behavioral simulator for the Intel 8008 — the first 8-bit microprocessor and ancestor of x86',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.016',
    PREREQ_PM        => {},
    TEST_REQUIRES    => {
        'Test2::V0' => '0',
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/coding-adventures/coding-adventures.git',
            },
        },
    },
);
