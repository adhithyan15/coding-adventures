use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::DartmouthBasicParser',
    VERSION_FROM     => 'lib/CodingAdventures/DartmouthBasicParser.pm',
    ABSTRACT         => 'Hand-written recursive-descent 1964 Dartmouth BASIC parser',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::DartmouthBasicLexer' => 0,
        'File::Basename'                         => 0,
        'File::Spec'                             => 0,
    },
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
                web  => 'https://github.com/adhithyan15/coding-adventures',
            },
        },
    },
);
