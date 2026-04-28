use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::QrCode',
    VERSION_FROM     => 'lib/CodingAdventures/QrCode.pm',
    ABSTRACT         => 'ISO/IEC 18004:2015 QR Code encoder — all ECC levels, all modes',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::Barcode2D' => 0,
        'CodingAdventures::GF256'     => 0,
        'Encode'                      => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
