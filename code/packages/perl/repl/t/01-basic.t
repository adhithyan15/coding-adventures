use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Repl;
use CodingAdventures::Repl::EchoLanguage;
use CodingAdventures::Repl::DefaultPrompt;
use CodingAdventures::Repl::SilentWaiting;
use CodingAdventures::Repl::Loop;
use CodingAdventures::Repl::Language;
use CodingAdventures::Repl::Prompt;
use CodingAdventures::Repl::Waiting;

# ============================================================================
# Helper: run_repl_with_input(\@lines) → \@output
#
# This helper drives the REPL with a prepared list of input lines and captures
# all output in an array.
#
# It uses CodingAdventures::Repl::run_with_io() — the fully-injectable API —
# so no terminal, STDIN, or STDOUT is involved. The test is completely
# hermetic: it produces the same result every time regardless of environment.
#
# How the input_fn works:
#   - Each call to $input_fn->() dequeues one line from the front of @lines.
#   - When @lines is empty, shift() returns undef, which signals EOF to the loop.
#
# How the output_fn works:
#   - Each call to $output_fn->($str) pushes $str onto @captured.
#   - At the end, @captured holds every string the REPL printed, in order.
#
# ============================================================================
sub run_repl_with_input {
    my ($lines_ref, %extra_args) = @_;
    my @lines   = @$lines_ref;
    my @captured;

    my $language = $extra_args{language} // CodingAdventures::Repl::EchoLanguage->new();
    my $prompt   = $extra_args{prompt}   // CodingAdventures::Repl::DefaultPrompt->new();
    my $waiting  = $extra_args{waiting}  // CodingAdventures::Repl::SilentWaiting->new();

    CodingAdventures::Repl::run_with_io(
        language  => $language,
        prompt    => $prompt,
        waiting   => $waiting,
        input_fn  => sub { shift @lines },
        output_fn => sub { push @captured, $_[0] },
    );

    return \@captured;
}

# ============================================================================
# Test 1: EchoLanguage echoes input back
#
# When we send "hello" to the EchoLanguage REPL and then EOF, we expect:
#   a) The global prompt "> " is printed before the input line
#   b) "hello\n" is printed as the result (the echo)
#   c) A second "> " is printed (the loop asks for the next line)
#   d) input_fn returns undef → loop exits
#
# Note: the loop always prints a prompt BEFORE calling input_fn. So for N
# complete inputs and then EOF, we see N+1 prompts total (the last prompt
# is the one printed just before EOF is read).
# ============================================================================
subtest 'EchoLanguage echoes input' => sub {
    my $output = run_repl_with_input(['hello']);

    # Find the output line (not the prompts)
    my @result_lines = grep { $_ eq "hello\n" } @$output;
    is(scalar @result_lines, 1, 'exactly one echo line');
    is($result_lines[0], "hello\n", 'echoed line is correct');
};

# ============================================================================
# Test 2: :quit command exits the loop
#
# Sending ":quit" causes the EchoLanguage to return the bare string 'quit',
# which the loop treats as a signal to stop immediately — no output is printed,
# and the loop does not ask for another line.
# ============================================================================
subtest ':quit exits the loop' => sub {
    my $output = run_repl_with_input([':quit']);

    # The loop prints the prompt, reads ":quit", exits — no echo output.
    my @non_prompts = grep { $_ ne '> ' && $_ ne "... " } @$output;
    is(scalar @non_prompts, 0, 'no non-prompt output after :quit');
};

# ============================================================================
# Test 3: Multiple input lines before :quit
#
# Verify that the loop processes multiple inputs correctly before stopping.
# Each non-quit line should produce an echo; :quit should produce none.
# ============================================================================
subtest 'multiple inputs before :quit' => sub {
    my $output = run_repl_with_input(['alpha', 'beta', 'gamma', ':quit']);

    my @echos = grep { /\A(?:alpha|beta|gamma)\n\z/ } @$output;
    is(scalar @echos, 3, 'three echo lines for three inputs');

    # Order should be preserved
    is($echos[0], "alpha\n", 'first echo is alpha');
    is($echos[1], "beta\n",  'second echo is beta');
    is($echos[2], "gamma\n", 'third echo is gamma');
};

# ============================================================================
# Test 4: EOF (empty input list) exits cleanly
#
# When the input source is exhausted immediately (no lines at all), the loop
# should exit after printing the first prompt. No errors, no crashes.
# ============================================================================
subtest 'EOF exits cleanly' => sub {
    my $output = run_repl_with_input([]);

    # The loop prints one prompt, calls input_fn (gets undef = EOF), exits.
    is(scalar @$output, 1, 'exactly one output (the prompt)');
    is($output->[0], '> ', 'output is the global prompt');
};

# ============================================================================
# Test 5: Language that returns an error tuple
#
# Build a tiny anonymous Language that always returns ['error', 'oops'].
# Verify that the loop prints "Error: oops\n".
# ============================================================================
subtest 'error result prints error message' => sub {
    # We create a minimal language object using an anonymous package.
    # Perl's bless works with any hashref, so we can create a one-off object
    # for testing without declaring a full package at the top level.
    my $error_lang = bless {}, 'TestErrorLang';
    no strict 'refs';
    *{'TestErrorLang::eval'} = sub { return ['error', 'oops'] };
    use strict 'refs';

    my $output = run_repl_with_input(['anything'], language => $error_lang);

    my @errors = grep { /\AError:/ } @$output;
    is(scalar @errors, 1, 'one error line');
    is($errors[0], "Error: oops\n", 'error message is correct');
};

# ============================================================================
# Test 6: Exception inside language->eval() is caught and presented as error
#
# If the language's eval() method calls die(), the REPL should catch it and
# display an error message rather than crashing the whole process.
# ============================================================================
subtest 'exception inside eval is caught' => sub {
    my $die_lang = bless {}, 'TestDieLang';
    no strict 'refs';
    *{'TestDieLang::eval'} = sub { die "kaboom" };
    use strict 'refs';

    my $output = run_repl_with_input(['anything'], language => $die_lang);

    my @errors = grep { /\AError:.*kaboom/ } @$output;
    is(scalar @errors, 1, 'exception is converted to error output');
};

# ============================================================================
# Test 7: DefaultPrompt returns expected strings
# ============================================================================
subtest 'DefaultPrompt strings' => sub {
    my $p = CodingAdventures::Repl::DefaultPrompt->new();
    is($p->global_prompt(), '> ',   'global_prompt is "> "');
    is($p->line_prompt(),   '... ', 'line_prompt is "... "');
};

# ============================================================================
# Test 8: SilentWaiting all methods work without error
# ============================================================================
subtest 'SilentWaiting is a no-op' => sub {
    my $w = CodingAdventures::Repl::SilentWaiting->new();
    ok(1, 'SilentWaiting object created');

    my $state = $w->start();
    ok(1, 'start() returns without error');

    $state = $w->tick($state);
    ok(1, 'tick() returns without error');

    my $ms = $w->tick_ms();
    is($ms, 100, 'tick_ms() returns 100');

    $w->stop($state);
    ok(1, 'stop() returns without error');
};

# ============================================================================
# Test 9: EchoLanguage interface contract
# ============================================================================
subtest 'EchoLanguage interface contract' => sub {
    my $lang = CodingAdventures::Repl::EchoLanguage->new();

    # :quit returns the string 'quit'
    my $quit = $lang->eval(':quit');
    ok(!ref($quit), ':quit result is not a reference');
    is($quit, 'quit', ':quit returns "quit"');

    # Any other input returns ['ok', $input]
    my $ok = $lang->eval('hello world');
    is(ref($ok), 'ARRAY', 'normal result is an arrayref');
    is($ok->[0], 'ok', 'tag is "ok"');
    is($ok->[1], 'hello world', 'payload is the original input');

    # Empty string also echoes
    my $empty = $lang->eval('');
    is($empty->[0], 'ok',  'empty input tag is "ok"');
    is($empty->[1], '',    'empty input echoes empty string');
};

# ============================================================================
# Test 10: base Language returns error (not implemented)
# ============================================================================
subtest 'Language base class eval returns error' => sub {
    my $base = CodingAdventures::Repl::Language->new();
    my $result = $base->eval('anything');
    is(ref($result), 'ARRAY', 'base eval returns arrayref');
    is($result->[0], 'error', 'base eval tag is "error"');
};

# ============================================================================
# Test 11: Prompt returned in output during normal session
#
# The loop prints the global prompt before each input_fn call.
# Verify that prompts appear in the output stream interleaved with results.
# ============================================================================
subtest 'prompts appear in output' => sub {
    my $output = run_repl_with_input(['x', ':quit']);

    # Expected output sequence:
    #   "> "      (prompt before "x")
    #   "x\n"     (echo of "x")
    #   "> "      (prompt before ":quit")
    # Then loop exits — no more output.

    is($output->[0], '> ',  'first token is the global prompt');
    is($output->[1], "x\n", 'second token is the echo');
    is($output->[2], '> ',  'third token is the prompt before :quit');
    is(scalar @$output, 3,   'exactly 3 output tokens');
};

# ============================================================================
# Test 12: run_with_io validates required arguments
# ============================================================================
subtest 'run_with_io dies without required args' => sub {
    ok(
        !eval {
            CodingAdventures::Repl::run_with_io(
                input_fn  => sub { undef },
                output_fn => sub { },
            );
            1;
        },
        'run_with_io dies when language is missing'
    );

    ok(
        !eval {
            CodingAdventures::Repl::run_with_io(
                language  => CodingAdventures::Repl::EchoLanguage->new(),
                output_fn => sub { },
            );
            1;
        },
        'run_with_io dies when input_fn is missing'
    );

    ok(
        !eval {
            CodingAdventures::Repl::run_with_io(
                language => CodingAdventures::Repl::EchoLanguage->new(),
                input_fn => sub { undef },
            );
            1;
        },
        'run_with_io dies when output_fn is missing'
    );
};

done_testing;
