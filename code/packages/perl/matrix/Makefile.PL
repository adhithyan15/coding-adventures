use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Matrix',
    VERSION_FROM     => 'lib/CodingAdventures/Matrix.pm',
    ABSTRACT         => '2D matrix type with arithmetic and linear-algebra operations',
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
