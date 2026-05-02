use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::SqlCsvSource',
    AUTHOR           => 'coding-adventures contributors',
    VERSION_FROM     => 'lib/CodingAdventures/SqlCsvSource.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/SqlCsvSource.pm',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'Exporter'   => 0,
        'File::Spec' => 0,
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
    dist  => { COMPRESS => 'gzip -9f', SUFFIX => 'gz' },
    clean => { FILES => 'CodingAdventures-SqlCsvSource-*' },
);
