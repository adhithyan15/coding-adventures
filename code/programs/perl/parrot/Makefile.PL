use strict;
use warnings;
use ExtUtils::MakeMaker;

# Makefile.PL — build and test configuration for the Parrot REPL program.
#
# ExtUtils::MakeMaker is Perl's standard build system. It generates a
# Makefile from the metadata below. The key targets:
#
#   make          — check dependencies (no compilation needed for pure Perl)
#   make test     — run t/*.t via prove
#   make install  — install the script (optional)
#
# We list only Test2::V0 as a test dependency. The runtime dependencies
# (CodingAdventures::Repl::*) are resolved via the `use lib` directives in
# parrot.pl and t/test_parrot.t, pointing directly at the monorepo packages.

WriteMakefile(
    NAME             => 'Parrot',
    VERSION          => '0.1.0',
    ABSTRACT         => 'Parrot REPL - echoes back whatever you type',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
    EXE_FILES        => ['parrot.pl'],
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
