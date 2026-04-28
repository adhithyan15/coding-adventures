use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::ChaCha20Poly1305',
    VERSION_FROM     => 'lib/CodingAdventures/ChaCha20Poly1305.pm',
    ABSTRACT         => 'ChaCha20-Poly1305 AEAD cipher (RFC 8439)',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'Math::BigInt' => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
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
