use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::MarkovChain',
    VERSION_FROM     => 'lib/CodingAdventures/MarkovChain.pm',
    ABSTRACT         => 'Pure-Perl general-purpose Markov Chain',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::DirectedGraph' => 0,
        'List::Util'                      => 0,
    },
    TEST_REQUIRES    => {
        'Test::More' => 0,
    },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
                web  => 'https://github.com/adhithyan15/coding-adventures',
            },
        },
    },
);
