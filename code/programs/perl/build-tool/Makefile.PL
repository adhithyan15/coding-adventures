use strict;
use warnings;
use ExtUtils::MakeMaker;

# Makefile.PL -- Build configuration for the Perl build tool
# ===========================================================
#
# ExtUtils::MakeMaker is Perl's oldest, most portable build system. It ships
# with every Perl distribution (5.0+) and generates a native Makefile for the
# current platform. We use it here because:
#
#   1. It is a core module — no CPAN install required.
#   2. It handles `perl Makefile.PL && make test` — the standard Perl workflow.
#   3. It integrates with cpanm for dependency installation.
#
# To build and test:
#
#   perl Makefile.PL
#   make
#   make test
#
# To install:
#
#   make install   # system-wide
#   make install DESTDIR=/some/prefix   # prefix install

WriteMakefile(
    NAME             => 'CodingAdventures::BuildTool',
    AUTHOR           => 'coding-adventures contributors',
    VERSION_FROM     => 'lib/CodingAdventures/BuildTool.pm',
    ABSTRACT_FROM    => 'lib/CodingAdventures/BuildTool.pm',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026',

    # EXE_FILES lists scripts that should be installed into the user's $PATH.
    # We install `build-tool` from the bin/ directory.
    EXE_FILES => ['bin/build-tool'],

    # PREREQ_PM lists runtime dependencies. All core modules are listed
    # explicitly for documentation purposes, even though they do not need
    # to be installed. The only non-core dependency is the internal
    # coding-adventures-directed-graph package (loaded dynamically when
    # available, otherwise falls back to the inline graph implementation).
    PREREQ_PM => {
        'Digest::SHA'     => '0',
        'JSON::PP'        => '2.0',
        'File::Find'      => '0',
        'File::Spec'      => '0',
        'File::Basename'  => '0',
        'Getopt::Long'    => '2.0',
        'POSIX'           => '0',
        'Cwd'             => '0',
        'File::Path'      => '2.0',
        'Scalar::Util'    => '0',
        'List::Util'      => '0',
    },

    # TEST_REQUIRES lists test-only dependencies. Test2::V0 is not a core
    # module — it must be installed from CPAN. We use it because:
    #   1. It has a clean, modern API (unlike Test::More).
    #   2. It provides subtests, diagnostics, and structured output.
    #   3. It produces valid TAP output compatible with prove(1).
    TEST_REQUIRES => {
        'Test2::V0' => '0.000159',
    },

    META_MERGE => {
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
