use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::MosaicEmitReact',
    VERSION_FROM     => 'lib/CodingAdventures/MosaicEmitReact.pm',
    ABSTRACT         => 'React backend: emits TSX functional components from MosaicIR',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::MosaicVm' => 0,
        'CodingAdventures::MosaicAnalyzer' => 0,
        'CodingAdventures::MosaicParser' => 0,
        'CodingAdventures::MosaicLexer' => 0,
        'CodingAdventures::GrammarTools' => 0,
        'CodingAdventures::Lexer' => 0,
        'CodingAdventures::DirectedGraph' => 0,
        'CodingAdventures::Parser' => 0,
        'CodingAdventures::StateMachine' => 0,
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
