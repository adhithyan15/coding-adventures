use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::TetradRuntime',
    VERSION_FROM     => 'lib/CodingAdventures/TetradRuntime.pm',
    ABSTRACT         => 'Tetrad compiler and runtime on the Perl LANG VM',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
