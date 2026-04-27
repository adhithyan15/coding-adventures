use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::WasmModuleEncoder',
    VERSION_FROM     => 'lib/CodingAdventures/WasmModuleEncoder.pm',
    ABSTRACT         => 'Encode structured WebAssembly modules into raw bytes',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    PREREQ_PM        => {
        'CodingAdventures::WasmLeb128' => 0,
        'CodingAdventures::WasmTypes'  => 0,
    },
    TEST_REQUIRES    => {
        'CodingAdventures::WasmModuleParser' => 0,
        'CodingAdventures::WasmValidator'    => 0,
        'Test2::V0'                          => 0,
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
