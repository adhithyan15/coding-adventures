use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::GradientDescent',
    VERSION_FROM     => 'lib/CodingAdventures/GradientDescent.pm',
    ABSTRACT         => 'Gradient descent weight optimiser — SGD training loop with numerical gradient support',
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
