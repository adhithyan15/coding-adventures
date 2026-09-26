use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::DerAsn1',
    VERSION_FROM     => 'lib/CodingAdventures/DerAsn1.pm',
    ABSTRACT         => 'Bounded typed ASN.1 DER value decoding',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::DerTlv' => 0,
        'Hash::Util::FieldHash'     => 0,
        'Math::BigInt'              => 0,
        'Scalar::Util'              => 0,
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
