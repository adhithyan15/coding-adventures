package Parrot::Prompt;

# ============================================================================
# Parrot::Prompt — implements the CodingAdventures::Repl Prompt interface
# ============================================================================
#
# # What is a Prompt?
#
# The REPL framework's Prompt interface has two responsibilities:
#
#   global_prompt()  → string
#     Text displayed before each READ step. The loop calls this method on
#     EVERY iteration, so it functions as the per-line prompt. In a simple
#     terminal REPL this is the "> " characters the user sees before they
#     type.
#
#   line_prompt()    → string
#     Text shown at the start of continuation lines — lines that follow an
#     incomplete expression. EchoLanguage never produces multi-line input, so
#     this method is never called in practice, but it must exist to satisfy
#     the interface contract.
#
# # Why a custom prompt?
#
# The framework ships a DefaultPrompt. This module shows how to replace it
# with domain-specific text. Any object with these two methods works — Perl
# uses duck typing (if it quacks like a Prompt, it IS a Prompt).
#
# # The parrot personality
#
# A parrot is famous for repeating what it hears. The prompt reinforces this:
# every line begins with the parrot emoji and a reminder that input will be
# echoed.

use strict;
use warnings;

our $VERSION = '0.1.0';

# ----------------------------------------------------------------------------
# new() → Parrot::Prompt instance
#
# Parrot::Prompt is stateless. new() exists for consistency with Perl's
# standard OOP convention (all objects are created via bless + new).
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# global_prompt() → string
#
# The prompt shown before each user input line.
#
# Design notes:
#   - We include the title "Parrot REPL" so that test assertions can search
#     for a known substring without depending on the emoji rendering.
#   - The "\x{1F99C}" is the Unicode code point for the parrot emoji (🦜).
#     Writing it as an escape keeps the source file clean ASCII, which is
#     portable across editors and Perl versions.
#   - The " > " suffix follows the Unix convention of ending a prompt with
#     a space so the user's cursor starts one space after the prompt character.
# ----------------------------------------------------------------------------
sub global_prompt {
    return "Parrot REPL - I repeat everything you say! (:quit to exit)\n"
         . "\x{1F99C} > ";
}

# ----------------------------------------------------------------------------
# line_prompt() → string
#
# The prompt shown on continuation lines. EchoLanguage always returns a
# complete result after one line, so this is never displayed in the Parrot
# REPL. It is implemented here to honour the Prompt contract.
# ----------------------------------------------------------------------------
sub line_prompt {
    return "\x{1F99C} . ";
}

1;

__END__

=head1 NAME

Parrot::Prompt - Parrot-themed prompt for the CodingAdventures REPL framework

=head1 SYNOPSIS

    use Parrot::Prompt;
    use CodingAdventures::Repl::Loop;

    CodingAdventures::Repl::Loop::run(
        language  => ...,
        prompt    => Parrot::Prompt->new(),
        waiting   => ...,
        input_fn  => sub { ... },
        output_fn => sub { ... },
    );

=head1 DESCRIPTION

Implements the two-method Prompt interface expected by C<CodingAdventures::Repl::Loop>:

=over 4

=item C<global_prompt()> — banner plus the C<< 🦜 > >> prompt string.

=item C<line_prompt()> — continuation prompt (C<< 🦜 .  >>).

=back

=head1 VERSION

0.1.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
