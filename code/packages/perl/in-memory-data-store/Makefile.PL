use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::InMemoryDataStore',
    VERSION_FROM     => 'lib/CodingAdventures/InMemoryDataStore.pm',
    ABSTRACT         => 'Pure Perl RESP facade for the in-memory data store',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::InMemoryDataStoreEngine'   => 0,
        'CodingAdventures::InMemoryDataStoreProtocol' => 0,
        'CodingAdventures::RespProtocol'              => 0,
        'Exporter'                                    => 0,
        'Scalar::Util'                                => 0,
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
