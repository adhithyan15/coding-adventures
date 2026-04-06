package CodingAdventures::Repl;

# ============================================================================
# CodingAdventures::Repl — REPL Framework
# ============================================================================
#
# # What is a REPL?
#
# REPL stands for Read-Eval-Print Loop. It is the interactive shell mode that
# most programming languages provide: you type an expression, the language
# evaluates it, prints the result, and waits for the next expression. Rinse
# and repeat.
#
# Famous REPLs you may have used:
#
#   Python   → python3 (the ">>>" prompt)
#   Ruby     → irb
#   Node.js  → node (type "node" with no arguments)
#   Haskell  → ghci
#   Lisp     → various, historically the FIRST REPLs (1960s)
#   Erlang   → erl
#   Elixir   → iex
#
# REPLs are beloved by programmers because they give INSTANT FEEDBACK. Instead
# of write → compile → run → observe, the cycle is just type → observe.
#
# # This Framework
#
# CodingAdventures::Repl is a REPL framework: it provides the loop machinery
# so that you can focus on implementing the language-specific eval logic.
#
# The framework is built around three pluggable interfaces (see the sub-modules
# for the full interface contracts):
#
#   Language  — knows how to EVALUATE one expression and return a result
#   Prompt    — knows what prompt string to DISPLAY ("> ", "myrepl> ", etc.)
#   Waiting   — knows how to SHOW PROGRESS while eval is running
#
# Together, these three form a clean separation of concerns:
#
#     USER INPUT
#         │
#         ▼
#   ┌──────────┐    ┌──────────┐    ┌──────────┐
#   │  Prompt  │    │ Language │    │ Waiting  │
#   │  "what   │    │ "how to  │    │ "show    │
#   │   to ask"│    │  eval"   │    │ progress"│
#   └──────────┘    └──────────┘    └──────────┘
#         │                │                │
#         └────────────────┴────────────────┘
#                          │
#                    ┌─────┴─────┐
#                    │   Loop    │
#                    │ (this pkg)│
#                    └─────┬─────┘
#                          │
#                    OUTPUT / RESULT
#
# # I/O Injection
#
# The framework does not read from STDIN or write to STDOUT directly.
# Instead, you pass two coderefs:
#
#   input_fn   → called to get the next line of input (returns undef for EOF)
#   output_fn  → called with each string to display to the user
#
# This makes the framework FULLY TESTABLE without a terminal. See the test
# suite (t/01-basic.t) for examples.
#
# # Quick Start
#
#   use CodingAdventures::Repl;
#
#   CodingAdventures::Repl::run(
#       language => CodingAdventures::Repl::EchoLanguage->new(),
#   );
#
# Or with full customisation:
#
#   CodingAdventures::Repl::run_with_io(
#       language  => MyLanguage->new(),
#       prompt    => MyPrompt->new(),
#       waiting   => MyWaiting->new(),
#       input_fn  => sub { <STDIN> },
#       output_fn => sub { print $_[0] },
#   );
#
# ============================================================================

use strict;
use warnings;

use CodingAdventures::Repl::Loop;
use CodingAdventures::Repl::EchoLanguage;
use CodingAdventures::Repl::DefaultPrompt;
use CodingAdventures::Repl::SilentWaiting;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# run(%args) → void
#
# Convenience entry point that runs the REPL with default I/O (STDIN/STDOUT)
# and default Prompt/Waiting implementations.
#
# Required arguments:
#
#   language   — A Language object (duck type: must have eval() method)
#
# Optional arguments:
#
#   prompt     — A Prompt object (default: DefaultPrompt)
#   waiting    — A Waiting object (default: SilentWaiting)
#
# The loop reads from STDIN and writes to STDOUT. Input lines are read with
# Perl's readline (<STDIN>), which returns undef at EOF.
#
# Example:
#
#   CodingAdventures::Repl::run(
#       language => MyLanguage->new(),
#   );
#
# @param %args   Named arguments as described above
# ----------------------------------------------------------------------------
sub run {
    my (%args) = @_;

    my $language = $args{language} or die 'Repl::run: language is required';
    my $prompt   = $args{prompt}   // CodingAdventures::Repl::DefaultPrompt->new();
    my $waiting  = $args{waiting}  // CodingAdventures::Repl::SilentWaiting->new();

    # Default I/O: read from STDIN, write to STDOUT.
    #
    # We disable STDOUT buffering so that prompt strings appear before blocking
    # on the STDIN read. Without this, the "> " prompt may not appear until
    # the next line is returned, making the REPL feel broken.
    local $| = 1;

    my $input_fn  = sub { scalar <STDIN> };
    my $output_fn = sub { print $_[0] };

    CodingAdventures::Repl::Loop::run(
        language  => $language,
        prompt    => $prompt,
        waiting   => $waiting,
        input_fn  => $input_fn,
        output_fn => $output_fn,
    );

    return;
}

# ----------------------------------------------------------------------------
# run_with_io(%args) → void
#
# Full-control entry point. Runs the REPL with caller-supplied I/O coderefs.
#
# Required arguments:
#
#   language   — A Language object (duck type: must have eval() method)
#   input_fn   — Coderef; called with no args; returns next input line or undef
#   output_fn  — Coderef; called with a string to output
#
# Optional arguments:
#
#   prompt     — A Prompt object (default: DefaultPrompt)
#   waiting    — A Waiting object (default: SilentWaiting)
#
# This is the preferred interface for:
#   * Unit tests (inject strings, capture output)
#   * Embedding the REPL in a larger application
#   * Redirecting I/O to network sockets, pipes, etc.
#
# Example:
#
#   my @lines = ("1 + 1", ":quit");
#   my @output;
#
#   CodingAdventures::Repl::run_with_io(
#       language  => MyLanguage->new(),
#       input_fn  => sub { shift @lines },
#       output_fn => sub { push @output, $_[0] },
#   );
#
# @param %args   Named arguments as described above
# ----------------------------------------------------------------------------
sub run_with_io {
    my (%args) = @_;

    my $language  = $args{language}  or die 'Repl::run_with_io: language is required';
    my $input_fn  = $args{input_fn}  or die 'Repl::run_with_io: input_fn is required';
    my $output_fn = $args{output_fn} or die 'Repl::run_with_io: output_fn is required';
    my $prompt    = $args{prompt}    // CodingAdventures::Repl::DefaultPrompt->new();
    my $waiting   = $args{waiting}   // CodingAdventures::Repl::SilentWaiting->new();

    CodingAdventures::Repl::Loop::run(
        language  => $language,
        prompt    => $prompt,
        waiting   => $waiting,
        input_fn  => $input_fn,
        output_fn => $output_fn,
    );

    return;
}

1;

__END__

=head1 NAME

CodingAdventures::Repl - REPL framework with pluggable Language, Prompt, and Waiting

=head1 VERSION

Version 0.01

=head1 SYNOPSIS

    use CodingAdventures::Repl;
    use CodingAdventures::Repl::EchoLanguage;

    # Simplest usage — reads from STDIN, writes to STDOUT
    CodingAdventures::Repl::run(
        language => CodingAdventures::Repl::EchoLanguage->new(),
    );

    # Fully injected I/O (great for tests)
    my @input  = ("hello", ":quit");
    my @output;
    CodingAdventures::Repl::run_with_io(
        language  => CodingAdventures::Repl::EchoLanguage->new(),
        input_fn  => sub { shift @input },
        output_fn => sub { push @output, $_[0] },
    );

=head1 DESCRIPTION

A REPL (Read-Eval-Print Loop) framework for building interactive language
shells in Perl.

The framework separates concerns into three pluggable interfaces:

=over 4

=item B<Language> (C<CodingAdventures::Repl::Language>)

Evaluates one expression. Implement C<eval($input)> to return C<'quit'>,
C<['ok', $output]>, or C<['error', $message]>.

=item B<Prompt> (C<CodingAdventures::Repl::Prompt>)

Provides prompt strings. Implement C<global_prompt()> and C<line_prompt()>.

=item B<Waiting> (C<CodingAdventures::Repl::Waiting>)

Provides progress feedback during eval. Implement C<start()>, C<tick($state)>,
C<tick_ms()>, and C<stop($state)>.

=back

Built-in implementations:

=over 4

=item * C<CodingAdventures::Repl::EchoLanguage> — echoes input back

=item * C<CodingAdventures::Repl::DefaultPrompt> — "> " and "... "

=item * C<CodingAdventures::Repl::SilentWaiting> — no-op (Null Object)

=back

=head1 FUNCTIONS

=head2 run(%args)

Run the REPL reading from STDIN and writing to STDOUT.

Required: C<language>. Optional: C<prompt>, C<waiting>.

=head2 run_with_io(%args)

Run the REPL with injected I/O coderefs. Required: C<language>, C<input_fn>,
C<output_fn>. Optional: C<prompt>, C<waiting>.

=head1 SYNCHRONOUS EVALUATION

Eval is synchronous. An infinite loop in the language will hang the REPL.
The user can press Ctrl-C (SIGINT) to interrupt.

Perl threads are not used because they are not universally available and
introduce significant complexity. See C<CodingAdventures::Repl::Waiting> for
the full rationale.

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
