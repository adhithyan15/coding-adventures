use strict;
use warnings;
use Test2::V0;

# ============================================================================
# 00-load.t — Smoke test: can we load all modules?
# ============================================================================
#
# This is a "smoke test" — the simplest possible sanity check. If any of
# these require() calls fail, the rest of the test suite cannot run.
#
# We use eval { require ...; 1 } rather than use_ok() because Test2::V0
# does not export use_ok(). The pattern is equivalent:
#
#   eval { require Foo; 1 }  →  tries to load Foo; returns 1 on success,
#                                undef on failure ($@ contains the error)
#
# ============================================================================

# Main module
ok( eval { require CodingAdventures::Repl; 1 },
    'CodingAdventures::Repl loads' );
ok( CodingAdventures::Repl->VERSION, 'CodingAdventures::Repl has VERSION' );

# Interfaces
ok( eval { require CodingAdventures::Repl::Language; 1 },
    'CodingAdventures::Repl::Language loads' );

ok( eval { require CodingAdventures::Repl::Prompt; 1 },
    'CodingAdventures::Repl::Prompt loads' );

ok( eval { require CodingAdventures::Repl::Waiting; 1 },
    'CodingAdventures::Repl::Waiting loads' );

# Loop engine
ok( eval { require CodingAdventures::Repl::Loop; 1 },
    'CodingAdventures::Repl::Loop loads' );

# Built-in implementations
ok( eval { require CodingAdventures::Repl::EchoLanguage; 1 },
    'CodingAdventures::Repl::EchoLanguage loads' );

ok( eval { require CodingAdventures::Repl::DefaultPrompt; 1 },
    'CodingAdventures::Repl::DefaultPrompt loads' );

ok( eval { require CodingAdventures::Repl::SilentWaiting; 1 },
    'CodingAdventures::Repl::SilentWaiting loads' );

done_testing;
