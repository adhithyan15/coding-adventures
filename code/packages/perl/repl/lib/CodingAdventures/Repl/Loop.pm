package CodingAdventures::Repl::Loop;

# ============================================================================
# CodingAdventures::Repl::Loop — The REPL loop implementation
# ============================================================================
#
# # The Heart of the REPL
#
# This module contains the actual loop that drives the Read-Eval-Print cycle.
# Everything else in the framework (Language, Prompt, Waiting) is infrastructure
# that this loop orchestrates.
#
# # The REPL lifecycle
#
# A REPL session proceeds through these phases, repeated until the user quits:
#
#   1. READ    — Display a prompt, then read a line of input.
#                If the input source is exhausted (EOF), exit the loop.
#
#   2. EVAL    — Pass the input to the Language's eval() method.
#                Wrap the call in Perl's eval{} to catch any exceptions.
#
#   3. PRINT   — Examine the result:
#                  'quit'           → exit the loop immediately
#                  ['ok',  $output] → print $output if defined
#                  ['error', $msg]  → print the error message
#
#   4. LOOP    — Go back to step 1.
#
# # I/O Injection
#
# This loop does NOT read from STDIN or write to STDOUT directly. Instead,
# it accepts two coderefs:
#
#   $input_fn  — called with no arguments; returns the next line as a string,
#                or undef to signal EOF (end of input)
#
#   $output_fn — called with a string; writes it to the output destination
#
# Why inject I/O? Because it makes the REPL framework fully testable without
# any terminal or keyboard required. Tests can supply:
#
#   input_fn  → a closure over a list of pre-written strings
#   output_fn → a closure that appends to an array for later assertion
#
# This same technique is used in Go (io.Reader / io.Writer interfaces),
# Java (InputStream / OutputStream), and Python (file-like objects). The Perl
# equivalent is passing coderefs.
#
# # Synchronous Evaluation
#
# Eval is synchronous. The Waiting handler's start() and stop() bracket the
# eval call, but tick() is never called in the middle — the eval blocks the
# entire main thread.
#
# See Repl/Waiting.pm for a full discussion of why synchronous is the right
# default for Perl.
#
# # Exception Safety
#
# Every call to $language->eval() is wrapped in Perl's eval{} block:
#
#   my $result = eval { $language->eval($input) };
#   if ($@) {
#       $result = ['error', $@];
#   }
#
# This means:
#   - A die() inside the language eval does NOT crash the REPL.
#   - The user sees the error message instead.
#   - The REPL continues accepting input.
#
# This is essential for a good interactive experience. Imagine a Python REPL
# that exited every time you had a syntax error — it would be unusable.
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# run(%args) → void
#
# Run the REPL loop until the user quits or input is exhausted.
#
# Named arguments:
#
#   language   (required)  — A Language object with an eval() method
#   prompt     (required)  — A Prompt object with global_prompt() and line_prompt()
#   waiting    (required)  — A Waiting object with start/tick/stop methods
#   input_fn   (required)  — Coderef; returns next input line or undef for EOF
#   output_fn  (required)  — Coderef; called with each string to output
#   mode       (optional)  — Execution mode string. Only 'sync' is supported.
#                            Defaults to 'sync'. Passing 'async' causes an
#                            immediate die, because Perl has no core threading
#                            module and asynchronous execution cannot be
#                            provided without external dependencies.
#
# The loop terminates when:
#   a) input_fn returns undef (EOF)
#   b) language->eval() returns the string 'quit'
#
# @param %args   Named arguments as described above
# ----------------------------------------------------------------------------
sub run {
    my (%args) = @_;

    # ----------------------------------------------------------------
    # Validate required arguments
    # ----------------------------------------------------------------
    #
    # We use croak() from Carp rather than die() because croak() reports
    # the error from the CALLER's perspective (the file and line that called
    # run()), not from inside Loop.pm. This makes error messages much more
    # useful during development.

    my $language  = $args{language}  or croak 'Loop::run: language is required';
    my $prompt    = $args{prompt}    or croak 'Loop::run: prompt is required';
    my $waiting   = $args{waiting}   or croak 'Loop::run: waiting is required';
    my $input_fn  = $args{input_fn}  or croak 'Loop::run: input_fn is required';
    my $output_fn = $args{output_fn} or croak 'Loop::run: output_fn is required';

    # ----------------------------------------------------------------
    # Validate the mode option
    # ----------------------------------------------------------------
    #
    # Perl does not ship a core threading module that is universally
    # available across platforms and Perl versions. The ithreads model
    # (threads.pm) was added in Perl 5.8 but has never been enabled by
    # default on all builds, and its use is widely discouraged due to
    # the complexity it introduces (every shared value must be explicitly
    # declared, data is copied between threads, etc.).
    #
    # Therefore this framework supports only synchronous ("sync") mode.
    # Rather than silently ignoring an "async" request — which would confuse
    # callers who expect non-blocking behaviour — we fail loudly and early,
    # before any I/O is performed. This follows the "fail fast" principle:
    # surface mismatches between caller expectations and implementation
    # capabilities as soon as possible.
    #
    # The check is performed HERE (in Loop::run), not just in Repl::run_with_io,
    # so that code which calls Loop::run directly also gets the protection.

    my $mode = $args{mode} // 'sync';

    if ( $mode eq 'async' ) {
        die "async mode is not supported in the Perl REPL implementation.\n"
          . "Use mode => 'sync' instead.";
    }

    # ----------------------------------------------------------------
    # The REPL loop
    # ----------------------------------------------------------------

    while (1) {

        # ---- READ phase ----
        #
        # Show the global prompt and read one line of input.
        # $input_fn returns undef to signal end-of-input (EOF).

        $output_fn->( $prompt->global_prompt() );
        my $input = $input_fn->();

        # EOF: the input source is exhausted. Exit the loop cleanly.
        # This is the normal exit path when processing a script file or
        # a test that has no more lines to feed.
        last unless defined $input;

        # Strip the trailing newline if present. The newline is an artefact
        # of how lines are delimited in text; it is not part of the user's
        # expression.
        chomp $input;

        # ---- EVAL phase ----
        #
        # Delegate to the Language object. Wrap in eval{} to catch exceptions.

        my $wait_state = $waiting->start();

        my $result = eval { $language->eval($input) };

        # Perl sets $@ to the error message when eval{} catches an exception.
        # We convert any exception into an ['error', ...] tuple so that the
        # PRINT phase below can handle it uniformly.
        if ($@) {
            my $err = $@;
            # Trim trailing newline from die() messages (Perl appends " at file line N.\n")
            chomp $err;
            $result = ['error', $err];
        }

        $waiting->stop($wait_state);

        # ---- PRINT phase ----
        #
        # Examine the result and produce appropriate output.

        # 'quit': the language has requested a clean exit.
        # We do NOT print anything — the session simply ends.
        if (!ref($result) && defined $result && $result eq 'quit') {
            last;
        }

        # ['ok', $output]: success.
        # Print the output if it is defined and non-empty.
        elsif (ref($result) eq 'ARRAY' && defined $result->[0] && $result->[0] eq 'ok') {
            my $output = $result->[1];
            if (defined $output) {
                $output_fn->($output . "\n");
            }
        }

        # ['error', $message]: an error occurred.
        # Print the error message, prefixed with "Error: " to distinguish
        # it visually from normal output.
        elsif (ref($result) eq 'ARRAY' && defined $result->[0] && $result->[0] eq 'error') {
            my $message = $result->[1] // '(unknown error)';
            $output_fn->('Error: ' . $message . "\n");
        }

        # Unknown result format: this is a bug in the Language implementation.
        # We report it as an error rather than silently ignoring it.
        else {
            $output_fn->("Error: language returned unexpected result format\n");
        }

        # ---- LOOP phase ----
        # Go back to the top of the while loop.
    }

    return;
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::Loop - The REPL loop engine

=head1 SYNOPSIS

    use CodingAdventures::Repl::Loop;
    use CodingAdventures::Repl::EchoLanguage;
    use CodingAdventures::Repl::DefaultPrompt;
    use CodingAdventures::Repl::SilentWaiting;

    my @lines = ("hello", ":quit");
    my @output;

    CodingAdventures::Repl::Loop::run(
        language  => CodingAdventures::Repl::EchoLanguage->new(),
        prompt    => CodingAdventures::Repl::DefaultPrompt->new(),
        waiting   => CodingAdventures::Repl::SilentWaiting->new(),
        input_fn  => sub { shift @lines },
        output_fn => sub { push @output, $_[0] },
    );

=head1 DESCRIPTION

Runs the Read-Eval-Print loop. Accepts I/O via coderefs for testability.

The loop exits when C<input_fn> returns C<undef> (EOF) or when the language
returns C<'quit'>.

Every call to C<$language-E<gt>eval()> is wrapped in C<eval {}> for exception
safety.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
