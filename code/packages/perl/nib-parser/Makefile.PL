use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::NibParser',
    VERSION_FROM     => 'lib/CodingAdventures/NibParser.pm',
    ABSTRACT         => 'Grammar-driven Nib parser',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::GrammarTools' => 0,
        'CodingAdventures::Parser'       => 0,
        'CodingAdventures::NibLexer'     => 0,
        'File::Basename'                 => 0,
        'File::Spec'                     => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
);
