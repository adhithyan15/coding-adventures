use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Barcode1D',
    VERSION_FROM     => 'lib/CodingAdventures/Barcode1D.pm',
    ABSTRACT         => 'High-level 1D barcode pipeline with native rendering hooks',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::Codabar'             => 0,
        'CodingAdventures::Code128'             => 0,
        'CodingAdventures::Code39'              => 0,
        'CodingAdventures::Ean13'               => 0,
        'CodingAdventures::Itf'                 => 0,
        'CodingAdventures::PixelContainer'      => 0,
        'CodingAdventures::PaintVmMetalNative'  => 0,
        'CodingAdventures::PaintCodecPngNative' => 0,
        'CodingAdventures::UpcA'                => 0,
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
