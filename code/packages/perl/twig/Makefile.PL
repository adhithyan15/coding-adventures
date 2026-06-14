use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::Twig',
    VERSION_FROM     => 'lib/CodingAdventures/Twig.pm',
    ABSTRACT         => 'Twig compiler and runtime on the Perl LANG VM',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
