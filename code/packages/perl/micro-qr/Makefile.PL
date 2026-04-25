use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::MicroQR',
    AUTHOR           => 'Adhithya Rajasekaran <adhithyan15@gmail.com>',
    VERSION_FROM     => 'lib/CodingAdventures/MicroQR.pm',
    ABSTRACT         => 'ISO/IEC 18004:2015 Annex E compliant Micro QR Code encoder',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.020',
    PREREQ_PM        => {
        'CodingAdventures::GF256'    => '0.1.0',
        'CodingAdventures::Barcode2D' => '0.1.0',
        'List::Util'                 => 0,
        'Carp'                       => 0,
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
