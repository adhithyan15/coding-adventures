use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::BrainfuckWasmCompiler',
    VERSION_FROM     => 'lib/CodingAdventures/BrainfuckWasmCompiler.pm',
    ABSTRACT         => 'End-to-end Brainfuck to WebAssembly compiler for Perl',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::Brainfuck'         => 0,
        'CodingAdventures::BrainfuckIrCompiler' => 0,
        'CodingAdventures::IrToWasmCompiler'  => 0,
        'CodingAdventures::IrToWasmValidator' => 0,
        'CodingAdventures::WasmModuleEncoder' => 0,
        'CodingAdventures::WasmModuleParser'  => 0,
        'CodingAdventures::WasmValidator'     => 0,
    },
    TEST_REQUIRES    => {
        'CodingAdventures::WasmRuntime' => 0,
        'Test2::V0'                     => 0,
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
