#!/usr/bin/env perl
# t/test_parrot.t — Tests for the Parrot REPL program
#
# We test two things:
#
#   1. Parrot::Prompt directly — verifying the prompt strings are correct.
#   2. The full REPL loop with injected I/O — verifying end-to-end behaviour.
#
# # I/O injection
#
# Tests never touch STDIN or STDOUT. Instead:
#
#   input_fn  — a closure over an array of pre-written strings. shift()
#               removes and returns the next element; returns undef when
#               the array is empty (= EOF).
#
#   output_fn — a closure that push()es each string into a local @output
#               array that we examine after the run.
#
# This technique makes tests deterministic, fast, and completely silent.
#
# # Test2::V0
#
# We use Test2::V0 because it is the modern Perl testing framework. It has
# richer assertion vocabulary than Test::More and better error diagnostics.
# Note: Test2::V0 does NOT have use_ok(). To test that a module loads, use:
#
#   ok( eval { require My::Module; 1 }, 'My::Module loads' );

use strict;
use warnings;
use FindBin qw($Bin);

# Add this program's lib/ directory and the repl framework's lib/ directory
# to the search path. $Bin is the directory of THIS test file (t/).
use lib "$Bin/../lib";
use lib "$Bin/../../../../packages/perl/repl/lib";

use Test2::V0;

use CodingAdventures::Repl::Loop;
use CodingAdventures::Repl::EchoLanguage;
use CodingAdventures::Repl::SilentWaiting;
use Parrot::Prompt;

# ============================================================================
# Helper: run_parrot
#
# Run the Parrot REPL loop with a list of pre-defined input strings.
# Returns the list of output strings collected during the run.
#
# Parameters:
#   @inputs — strings to feed one-by-one as input lines.
#             The queue runs out after the last string, simulating EOF.
#
# Returns:
#   @output — all strings passed to output_fn, in order.
#
# The loop terminates when:
#   a) input_fn returns undef (queue exhausted = EOF)
#   b) language->eval() returns 'quit' (the user typed :quit)
# ============================================================================

sub run_parrot {
    my (@inputs) = @_;
    my @output;

    CodingAdventures::Repl::Loop::run(
        language  => CodingAdventures::Repl::EchoLanguage->new(),
        prompt    => Parrot::Prompt->new(),
        waiting   => CodingAdventures::Repl::SilentWaiting->new(),
        input_fn  => sub { scalar @inputs ? shift @inputs : undef },
        output_fn => sub { push @output, $_[0] },
        mode      => 'sync',
    );

    return @output;
}

# Helper: join all output strings into one for substring matching.
sub joined { join('', @_) }

# ============================================================================
# Test 1: Parrot::Prompt loads
# ============================================================================

ok( eval { require Parrot::Prompt; 1 }, 'Parrot::Prompt loads without error' );

# ============================================================================
# Test 2: Parrot::Prompt->new() returns a blessed reference
# ============================================================================

{
    my $p = Parrot::Prompt->new();
    ok( ref($p), 'new() returns a blessed reference' );
}

# ============================================================================
# Test 3: global_prompt returns a non-empty string
# ============================================================================

{
    my $p  = Parrot::Prompt->new();
    my $gp = $p->global_prompt();
    ok( defined $gp && length $gp > 0, 'global_prompt returns a non-empty string' );
}

# ============================================================================
# Test 4: global_prompt contains the word "Parrot"
# ============================================================================

{
    my $p  = Parrot::Prompt->new();
    my $gp = $p->global_prompt();
    like( $gp, qr/Parrot/, 'global_prompt mentions "Parrot"' );
}

# ============================================================================
# Test 5: global_prompt contains the quit instruction
# ============================================================================

{
    my $p  = Parrot::Prompt->new();
    my $gp = $p->global_prompt();
    like( $gp, qr/:quit/, 'global_prompt mentions ":quit"' );
}

# ============================================================================
# Test 6: line_prompt returns a non-empty string
# ============================================================================

{
    my $p  = Parrot::Prompt->new();
    my $lp = $p->line_prompt();
    ok( defined $lp && length $lp > 0, 'line_prompt returns a non-empty string' );
}

# ============================================================================
# Test 7: global_prompt and line_prompt are different strings
# ============================================================================

{
    my $p = Parrot::Prompt->new();
    isnt( $p->global_prompt(), $p->line_prompt(),
          'global_prompt and line_prompt are different' );
}

# ============================================================================
# Test 8: echoes basic input back
# ============================================================================

{
    my @out  = run_parrot( 'hello', ':quit' );
    my $full = joined(@out);
    like( $full, qr/hello\n/, 'echoes basic input back with newline' );
}

# ============================================================================
# Test 9: :quit ends the session without echoing ":quit"
# ============================================================================

{
    my @out  = run_parrot(':quit');
    my $full = joined(@out);
    unlike( $full, qr/:quit\n/, ':quit is not echoed as output' );
}

# ============================================================================
# Test 10: EOF (no :quit) exits gracefully
# ============================================================================

{
    my @out;
    ok( eval {
            @out = run_parrot('one line only');
            1;
        },
        'loop exits cleanly on EOF without :quit'
    );
    like( joined(@out), qr/one line only\n/, 'EOF input was echoed before exit' );
}

# ============================================================================
# Test 11: multiple inputs are all echoed
# ============================================================================

{
    my @out  = run_parrot( 'alpha', 'beta', 'gamma', ':quit' );
    my $full = joined(@out);
    like( $full, qr/alpha\n/, 'echoes first input' );
    like( $full, qr/beta\n/,  'echoes second input' );
    like( $full, qr/gamma\n/, 'echoes third input' );
}

# ============================================================================
# Test 12: empty string is echoed (produces a newline in output)
# ============================================================================

{
    my @out  = run_parrot( '', ':quit' );
    my $full = joined(@out);
    # EchoLanguage returns ['ok', ''] for empty input.
    # The loop prints $output . "\n" when output is defined (even if empty).
    # So the output stream should contain at least one bare "\n".
    like( $full, qr/\n/, 'empty string produces a newline in output' );
}

# ============================================================================
# Test 13: session stops after :quit even when more input is queued
# ============================================================================

{
    my @out  = run_parrot( ':quit', 'should-not-appear', 'also-not-this' );
    my $full = joined(@out);
    unlike( $full, qr/should-not-appear/, 'input after :quit is not processed' );
}

# ============================================================================
# Test 14: prompt text appears in output
# ============================================================================

{
    my @out  = run_parrot( 'hi', ':quit' );
    my $full = joined(@out);
    like( $full, qr/Parrot/, 'prompt text appears in output' );
}

# ============================================================================
# Test 15: async mode raises an error
# ============================================================================

{
    my $died = 0;
    eval {
        CodingAdventures::Repl::Loop::run(
            language  => CodingAdventures::Repl::EchoLanguage->new(),
            prompt    => Parrot::Prompt->new(),
            waiting   => CodingAdventures::Repl::SilentWaiting->new(),
            input_fn  => sub { undef },
            output_fn => sub {},
            mode      => 'async',
        );
    };
    $died = 1 if $@;
    ok( $died, 'async mode raises an error' );
}

# ============================================================================
# Test 16: new() creates independent instances (stateless object)
# ============================================================================

{
    my $p1 = Parrot::Prompt->new();
    my $p2 = Parrot::Prompt->new();
    # They should be different references (distinct objects)...
    isnt( $p1, $p2, 'new() creates distinct instances' );
    # ...but return identical strings (same stateless behaviour).
    is( $p1->global_prompt(), $p2->global_prompt(),
        'independent instances return identical global_prompt' );
}

done_testing;
