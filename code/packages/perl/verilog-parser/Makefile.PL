use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::VerilogParser',
    VERSION_FROM     => 'lib/CodingAdventures/VerilogParser.pm',
    ABSTRACT         => 'Hand-written recursive-descent Verilog parser',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::GrammarTools'  => 0,
        'CodingAdventures::Lexer'         => 0,
        'CodingAdventures::VerilogLexer'  => 0,
        'File::Basename'                  => 0,
        'File::Spec'                      => 0,
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
