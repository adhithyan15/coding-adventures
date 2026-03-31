use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::StarlarkAstToBytecodeCompiler',
    AUTHOR           => 'Coding Adventures',
    VERSION_FROM     => 'lib/CodingAdventures/StarlarkAstToBytecodeCompiler.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/StarlarkAstToBytecodeCompiler.pm',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.016',
    CONFIGURE_REQUIRES => {
        'ExtUtils::MakeMaker' => '0',
    },
    TEST_REQUIRES => {
        'Test2::V0' => '0',
    },
    PREREQ_PM => {},
    dist  => { COMPRESS => 'gzip -9f', SUFFIX => 'gz' },
    clean => { FILES    => 'CodingAdventures-StarlarkAstToBytecodeCompiler-*' },
);
