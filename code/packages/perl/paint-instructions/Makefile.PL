use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::PaintInstructions',
    VERSION_FROM     => 'lib/CodingAdventures/PaintInstructions.pm',
    ABSTRACT         => 'Backend-neutral paint scene primitives',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
