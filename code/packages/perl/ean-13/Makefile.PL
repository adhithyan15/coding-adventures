use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Ean13',
    VERSION_FROM     => 'lib/CodingAdventures/Ean13.pm',
    ABSTRACT         => 'EAN-13 barcode encoder that emits paint scenes',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::BarcodeLayout1D' => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
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
