use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::CompilerSourceMap',
    VERSION_FROM     => 'lib/CodingAdventures/CompilerSourceMap.pm',
    ABSTRACT         => 'Source map chain sidecar for the AOT compiler pipeline',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
            },
        },
    },
);
