#!/usr/bin/env perl
# Parrot REPL — the world's simplest REPL.
#
# Whatever you type, I repeat back. Type :quit to exit.
#
# This script demonstrates the CodingAdventures::Repl framework by wiring
# three plug-in objects together and running the REPL loop on standard I/O:
#
#   EchoLanguage   — evaluates input by echoing it back unchanged.
#                    Special case: ":quit" returns 'quit', ending the loop.
#   Parrot::Prompt — parrot-themed banner and line prompt.
#   SilentWaiting  — shows nothing while "evaluating" (a no-op spinner).
#
# # Why only sync mode?
#
# Perl's core threading module (threads.pm) is not universally available and
# is widely discouraged due to complexity. The framework therefore supports
# only synchronous mode. Passing mode => 'async' causes an immediate die
# rather than silently producing wrong behaviour.
#
# # I/O model
#
# We read from STDIN line by line. Chomp removes the trailing newline; undef
# means EOF (Ctrl-D on Unix, Ctrl-Z on Windows, or piped input exhausted).
# Output is written directly to STDOUT via print. The framework calls our
# output coderef with each string — no extra newline is added by the script.
#
# # How to run
#
#   perl parrot.pl
#
# Press Ctrl-D (or Ctrl-Z on Windows) or type :quit to exit.

use strict;
use warnings;

# ---------------------------------------------------------------------------
# Module path setup
#
# FindBin::$Bin is the directory containing this script (parrot/).
# We add two lib directories:
#   1. parrot/lib/             — Parrot::Prompt lives here
#   2. ../../../packages/perl/repl/lib/  — the REPL framework lives here
#
# Using lib() at compile time (not runtime) ensures the paths are active
# before any `use Module` statement resolves.
# ---------------------------------------------------------------------------

use FindBin qw($Bin);
use lib "$Bin/lib";
use lib "$Bin/../../../packages/perl/repl/lib";

use CodingAdventures::Repl::Loop;
use CodingAdventures::Repl::EchoLanguage;
use CodingAdventures::Repl::SilentWaiting;
use Parrot::Prompt;

# ---------------------------------------------------------------------------
# Wire up the plug-ins
#
# Each plug-in is an independent, stateless object. Creating them here rather
# than inline in the run() call makes the wiring explicit and readable.
# ---------------------------------------------------------------------------

my $lang    = CodingAdventures::Repl::EchoLanguage->new();
my $prompt  = Parrot::Prompt->new();
my $waiting = CodingAdventures::Repl::SilentWaiting->new();

# ---------------------------------------------------------------------------
# Run the loop
#
# CodingAdventures::Repl::Loop::run() is a plain function (not a method).
# It takes named arguments.
#
# input_fn:
#   Reads one line from STDIN. Returns undef on EOF.
#   chomp() removes the trailing newline — the framework expects the line
#   without a newline character at the end.
#
# output_fn:
#   Writes a string to STDOUT. print() does not add a newline; the framework
#   adds "\n" after successful results, so we let it control line endings.
#
# mode:
#   'sync' is the only supported value. We pass it explicitly to document
#   the intent and to trigger a clear error if someone accidentally changes
#   it to 'async'.
# ---------------------------------------------------------------------------

CodingAdventures::Repl::Loop::run(
    language  => $lang,
    prompt    => $prompt,
    waiting   => $waiting,
    input_fn  => sub {
        my $line = <STDIN>;
        return undef unless defined $line;
        chomp $line;
        return $line;
    },
    output_fn => sub { print $_[0] },
    mode      => 'sync',
);
