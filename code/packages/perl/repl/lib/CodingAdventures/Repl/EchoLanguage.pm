package CodingAdventures::Repl::EchoLanguage;

# ============================================================================
# CodingAdventures::Repl::EchoLanguage — Built-in demo language
# ============================================================================
#
# # What is EchoLanguage?
#
# EchoLanguage is the simplest possible Language implementation: it echoes
# back whatever you type. It is not a real programming language — it has no
# parser, no interpreter, no semantics. It exists for two purposes:
#
#   1. TESTING — the test suite can drive the REPL without needing a real
#      language implementation. Tests are deterministic, have no side effects,
#      and finish instantly.
#
#   2. LEARNING — reading this tiny implementation teaches you the full
#      Language interface contract in ~10 lines of code.
#
# # Behaviour
#
#   Input ":quit"  → returns the bare string 'quit' (REPL exits)
#   Any other input → returns ['ok', $input] (REPL prints the input back)
#
# # The :quit convention
#
# The ":quit" command uses a colon prefix to distinguish REPL meta-commands
# from language-level expressions. This is the same convention used by GHCi
# (Haskell's REPL), where ":type", ":load", ":quit" are all meta-commands.
#
# Why not just "quit"? Because a language might have a function or variable
# called "quit". The colon prefix creates a separate namespace for REPL
# control commands that cannot collide with user-defined names.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → EchoLanguage instance
#
# No configuration needed — EchoLanguage is stateless.
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# eval($input) → 'quit' | ['ok', $input]
#
# Evaluate one user input string.
#
# The special input ":quit" causes the REPL to terminate. Every other
# string is echoed back verbatim as a successful result.
#
# Note how simple this is compared to a real language. A real language would:
#   1. Lex the input into tokens
#   2. Parse the tokens into an AST (Abstract Syntax Tree)
#   3. Type-check the AST (if statically typed)
#   4. Evaluate the AST against an environment
#   5. Serialise the result value to a display string
#
# EchoLanguage skips all of that and goes straight from raw string input
# to raw string output.
#
# @param $input   String typed by the user (no trailing newline)
# @return         'quit' or ['ok', $input]
# ----------------------------------------------------------------------------
sub eval {
    my ($self, $input) = @_;

    # The quit command — check for it first, before any other processing.
    return 'quit' if $input eq ':quit';

    # For all other input: echo it back wrapped in the success tuple.
    return ['ok', $input];
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::EchoLanguage - Trivial echo language for the REPL framework

=head1 SYNOPSIS

    use CodingAdventures::Repl;
    use CodingAdventures::Repl::EchoLanguage;

    CodingAdventures::Repl::run(
        language => CodingAdventures::Repl::EchoLanguage->new(),
    );

=head1 DESCRIPTION

EchoLanguage is the simplest possible Language implementation.

=over 4

=item C<eval(":quit")> returns C<'quit'>, causing the REPL to exit.

=item C<eval($x)> returns C<['ok', $x]> for any other input.

=back

Useful for testing the REPL framework and as a minimal example implementation.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
