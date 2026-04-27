use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::PaintVmAscii',
    VERSION_FROM     => 'lib/CodingAdventures/PaintVmAscii.pm',
    ABSTRACT         => 'Terminal backend for backend-neutral paint scenes',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::PaintInstructions' => 0,
    },
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
