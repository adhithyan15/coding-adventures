use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::HashFunctions',
    VERSION_FROM     => 'lib/CodingAdventures/HashFunctions.pm',
    ABSTRACT         => 'Educational non-cryptographic hash functions and analysis helpers',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'Encode'       => 0,
        'Exporter'     => 0,
        'Math::BigInt' => 0,
        'Scalar::Util' => 0,
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
