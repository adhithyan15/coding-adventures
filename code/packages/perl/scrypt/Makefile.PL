use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Scrypt',
    VERSION_FROM     => 'lib/CodingAdventures/Scrypt.pm',
    ABSTRACT         => 'scrypt (RFC 7914) memory-hard password-based key derivation — implemented from scratch',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::HMAC'   => 0,
        'CodingAdventures::SHA256' => 0,
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
