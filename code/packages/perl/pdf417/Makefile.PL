use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::PDF417',
    AUTHOR           => 'Adhithya Rajasekaran <adhithyan15@gmail.com>',
    VERSION_FROM     => 'lib/CodingAdventures/PDF417.pm',
    ABSTRACT         => 'ISO/IEC 15438:2015 PDF417 stacked barcode encoder',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.020',
    PREREQ_PM        => {
        'CodingAdventures::Barcode2D' => '0.1.0',
        'Carp'                        => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan/coding-adventures.git',
                web  => 'https://github.com/adhithyan/coding-adventures',
            },
        },
    },
);
