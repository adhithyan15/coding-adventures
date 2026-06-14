use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::JitCore',
    VERSION_FROM     => 'lib/CodingAdventures/JitCore.pm',
    ABSTRACT         => 'LANG VM JIT coordinator for Perl',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
