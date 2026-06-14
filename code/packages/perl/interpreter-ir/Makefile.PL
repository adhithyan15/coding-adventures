use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::InterpreterIr',
    VERSION_FROM     => 'lib/CodingAdventures/InterpreterIr.pm',
    ABSTRACT         => 'LANG VM interpreter IR for Perl packages',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
