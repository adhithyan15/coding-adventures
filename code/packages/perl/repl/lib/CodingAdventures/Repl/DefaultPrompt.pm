package CodingAdventures::Repl::DefaultPrompt;

# ============================================================================
# CodingAdventures::Repl::DefaultPrompt — Built-in prompt implementation
# ============================================================================
#
# # DefaultPrompt: the simplest useful prompt
#
# Most interactive REPLs use a short, memorable prompt. Python uses ">>> ".
# The Bourne shell uses "$ ". Prolog uses "?- ". These prompts share a
# common trait: they are short enough not to crowd the user's input yet
# distinctive enough to be recognisable.
#
# DefaultPrompt follows the same philosophy, providing classic Unix-style
# prompts that work well in any terminal:
#
#   global_prompt()  → "> "   (primary: ready for new expression)
#   line_prompt()    → "... " (secondary: awaiting continuation line)
#
# # Why two spaces in "... "?
#
# The trailing space after the prompt string is not decoration — it is a
# readability courtesy. Without a space, the user's cursor would appear
# immediately after the last character of the prompt, making it hard to
# distinguish prompt from input. Compare:
#
#   >hello world        ← prompt ">" with no space — hard to read
#   > hello world       ← prompt "> " with space — easy to read
#
# This is a small detail that every production REPL gets right.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → DefaultPrompt instance
#
# Stateless — no configuration needed.
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# global_prompt() → "> "
#
# The primary prompt shown before each new user expression.
#
# @return   The string "> " (greater-than sign followed by a space)
# ----------------------------------------------------------------------------
sub global_prompt {
    return '> ';
}

# ----------------------------------------------------------------------------
# line_prompt() → "... "
#
# The continuation/secondary prompt shown when the REPL is waiting for
# additional lines to complete a multi-line expression.
#
# The "... " (three dots and a space) is a widely recognised convention
# signalling "you started something, keep going".
#
# @return   The string "... " (three dots followed by a space)
# ----------------------------------------------------------------------------
sub line_prompt {
    return '... ';
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::DefaultPrompt - Default prompt for the REPL framework

=head1 SYNOPSIS

    use CodingAdventures::Repl::DefaultPrompt;

    my $p = CodingAdventures::Repl::DefaultPrompt->new();
    print $p->global_prompt();  # "> "
    print $p->line_prompt();    # "... "

=head1 DESCRIPTION

A simple Prompt implementation with classic Unix-style prompts:

=over 4

=item C<global_prompt()> — returns C<'E<gt> '>

=item C<line_prompt()> — returns C<'... '>

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
