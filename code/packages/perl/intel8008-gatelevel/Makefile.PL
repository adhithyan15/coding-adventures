use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Intel8008GateLevel',
    AUTHOR           => 'Coding Adventures',
    VERSION_FROM     => 'lib/CodingAdventures/Intel8008GateLevel.pm',
    ABSTRACT         => 'Gate-level Intel 8008 simulator — all arithmetic through AND/OR/XOR/NOT gates',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.016',
    PREREQ_PM        => {
        'CodingAdventures::LogicGates'  => '0',
        'CodingAdventures::Arithmetic'  => '0',
    },
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
