use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::TypeCheckerProtocol',
    VERSION_FROM     => 'lib/CodingAdventures/TypeCheckerProtocol.pm',
    ABSTRACT         => 'Shared type-check diagnostics and hook dispatch for Perl compiler frontends',
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
