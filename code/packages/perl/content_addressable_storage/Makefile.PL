use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::ContentAddressableStorage',
    VERSION_FROM     => 'lib/CodingAdventures/ContentAddressableStorage.pm',
    ABSTRACT         => 'Generic content-addressable storage with SHA-1 hashing',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::Sha1' => 0,
        'File::Path'             => 0,
        'File::Basename'         => 0,
        'File::Temp'             => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
        'File::Temp' => 0,
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
