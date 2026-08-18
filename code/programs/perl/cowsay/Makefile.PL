use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'Cowsay',
    VERSION_FROM     => 'lib/CodingAdventures/Cowsay.pm',
    ABSTRACT         => 'A configurable ASCII cow that speaks or thinks, rendered through paint-vm-ascii',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    EXE_FILES        => ['cowsay.pl'],
    PREREQ_PM        => {
        'CodingAdventures::CliBuilder'       => 0,
        'CodingAdventures::PaintInstructions' => 0,
        'CodingAdventures::PaintVmAscii'      => 0,
        'JSON::PP'                            => 2.00,
    },
    TEST_REQUIRES    => {
        'Test2::V0'   => 0,
        'File::Temp'  => 0,
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/coding-adventures/coding-adventures',
            },
        },
    },
);
