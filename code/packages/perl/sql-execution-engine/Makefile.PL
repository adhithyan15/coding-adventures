use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::SqlExecutionEngine',
    AUTHOR           => 'coding-adventures contributors',
    VERSION_FROM     => 'lib/CodingAdventures/SqlExecutionEngine.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/SqlExecutionEngine.pm',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.016',
    PREREQ_PM        => {
        'Scalar::Util' => 0,
        'POSIX'        => 0,
        'List::Util'   => 0,
    },
    TEST_REQUIRES => {
        'Test2::V0' => 0,
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
    dist  => { COMPRESS => 'gzip -9f', SUFFIX => 'gz' },
    clean => { FILES => 'CodingAdventures-SqlExecutionEngine-*' },
);
