use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::BarcodeLayout1D',
    VERSION_FROM     => 'lib/CodingAdventures/BarcodeLayout1D.pm',
    ABSTRACT         => 'Pure 1D barcode layout that converts barcode runs into paint scenes',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::PaintInstructions' => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
