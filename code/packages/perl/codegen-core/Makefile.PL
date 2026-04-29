use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::CodegenCore',
    VERSION_FROM     => 'lib/CodingAdventures/CodegenCore.pm',
    ABSTRACT         => 'LANG VM target backend registry',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {},
    TEST_REQUIRES    => { 'Test2::V0' => 0 },
);
