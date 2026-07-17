use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::HashSet',
    VERSION_FROM     => 'lib/CodingAdventures/HashSet.pm',
    ABSTRACT         => 'Persistent hash set with complete set algebra',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::HashMap' => 0,
        'Exporter'                  => 0,
        'Scalar::Util'              => 0,
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
