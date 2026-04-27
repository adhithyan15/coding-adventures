use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::NibWasmCompiler',
    VERSION_FROM     => 'lib/CodingAdventures/NibWasmCompiler.pm',
    ABSTRACT         => 'End-to-end Nib to WebAssembly compiler for Perl',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::IrToWasmCompiler'  => 0,
        'CodingAdventures::IrToWasmValidator' => 0,
        'CodingAdventures::NibIrCompiler'     => 0,
        'CodingAdventures::NibParser'         => 0,
        'CodingAdventures::NibTypeChecker'    => 0,
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
