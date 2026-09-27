use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::X509Extension',
    VERSION_FROM     => 'lib/CodingAdventures/X509Extension.pm',
    ABSTRACT         => 'Bounded generic X.509 Extension decoding',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::DerAsn1' => 0,
        'Hash::Util::FieldHash'      => 0,
        'Scalar::Util'               => 0,
    },
    TEST_REQUIRES    => {
        'JSON::PP'   => 0,
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
