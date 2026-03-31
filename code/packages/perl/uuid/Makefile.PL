use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::UUID',
    VERSION_FROM     => 'lib/CodingAdventures/UUID.pm',
    ABSTRACT         => 'UUID v1/v3/v4/v5/v7 generation and parsing',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::Md5'  => '0.01',
        'CodingAdventures::Sha1' => '0.01',
    },
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
