use strict;
use warnings;
use ExtUtils::MakeMaker;

# Makefile.PL — build and test configuration for the grammar-tools CLI program.
#
# Runtime dependencies (CodingAdventures::GrammarTools::*) are resolved via
# the `use lib` directive in grammar-tools.pl, pointing directly at
# code/packages/perl/grammar-tools/lib in the monorepo.

WriteMakefile(
    NAME             => 'GrammarToolsCli',
    VERSION          => '1.0.0',
    ABSTRACT         => 'CLI for validating and compiling .tokens/.grammar files',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    EXE_FILES        => ['grammar-tools.pl'],
    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/coding-adventures/coding-adventures',
            },
        },
    },
);
