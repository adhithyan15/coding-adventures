use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::InMemoryDataStoreEngine',
    VERSION_FROM     => 'lib/CodingAdventures/InMemoryDataStoreEngine.pm',
    ABSTRACT         => 'Pure Perl execution engine for the in-memory data store',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::HyperLogLog'                 => 0,
        'CodingAdventures::InMemoryDataStoreProtocol'   => 0,
        'POSIX'                                         => 0,
        'Time::HiRes'                                   => 0,
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
