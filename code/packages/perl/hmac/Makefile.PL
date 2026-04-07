use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::HMAC',
    VERSION_FROM     => 'lib/CodingAdventures/HMAC.pm',
    ABSTRACT         => 'HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::MD5'    => 0,
        'CodingAdventures::SHA1'   => 0,
        'CodingAdventures::SHA256' => 0,
        'CodingAdventures::SHA512' => 0,
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
