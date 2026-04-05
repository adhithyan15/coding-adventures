use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::WasmExecution',
    VERSION_FROM     => 'lib/CodingAdventures/WasmExecution.pm',
    ABSTRACT         => 'WebAssembly 1.0 wasm-execution',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::WasmLeb128' => 0,
        'CodingAdventures::WasmTypes' => 0,
        'CodingAdventures::WasmOpcodes' => 0,
        'CodingAdventures::WasmModuleParser' => 0,
        'CodingAdventures::VirtualMachine' => 0,
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
