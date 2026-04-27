use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::IrToWasmCompiler',
    VERSION_FROM     => 'lib/CodingAdventures/IrToWasmCompiler.pm',
    ABSTRACT         => 'Lower compiler IR programs into WebAssembly modules',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::CompilerIr' => 0,
        'CodingAdventures::WasmLeb128' => 0,
        'CodingAdventures::WasmOpcodes' => 0,
        'CodingAdventures::WasmTypes'  => 0,
    },
    TEST_REQUIRES    => {
        'CodingAdventures::Brainfuck::Parser' => 0,
        'CodingAdventures::BrainfuckIrCompiler' => 0,
        'CodingAdventures::WasmModuleEncoder' => 0,
        'CodingAdventures::WasmModuleParser' => 0,
        'CodingAdventures::WasmValidator' => 0,
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
