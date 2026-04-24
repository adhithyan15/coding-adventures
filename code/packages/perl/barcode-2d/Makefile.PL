use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Barcode2D',
    VERSION_FROM     => 'lib/CodingAdventures/Barcode2D.pm',
    ABSTRACT         => 'Shared 2D barcode abstraction: ModuleGrid and layout()',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::PaintInstructions' => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
