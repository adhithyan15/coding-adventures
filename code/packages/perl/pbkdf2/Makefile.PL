use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::PBKDF2',
    VERSION_FROM     => 'lib/CodingAdventures/PBKDF2.pm',
    ABSTRACT         => 'PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::HMAC'   => 0,
        'POSIX'                    => 0,
    },
    TEST_REQUIRES => {
        'Test2::V0' => 0,
    },
    META_MERGE => {
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
