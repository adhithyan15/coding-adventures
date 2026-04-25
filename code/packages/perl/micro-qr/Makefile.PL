use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::MicroQR',
    VERSION_FROM     => 'lib/CodingAdventures/MicroQR.pm',
    ABSTRACT         => 'ISO/IEC 18004:2015 Annex E Micro QR Code encoder',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::GF256'    => 0,
        'CodingAdventures::Barcode2D' => 0,
        'List::Util'                  => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
