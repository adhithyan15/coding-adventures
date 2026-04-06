package CodingAdventures::Repl::Prompt;

# ============================================================================
# CodingAdventures::Repl::Prompt — The Prompt interface
# ============================================================================
#
# # What is a Prompt?
#
# When you open a terminal and see a blinking cursor, something placed that
# cursor there after printing a small string — the PROMPT. A prompt tells the
# user: "I am ready; type your next command."
#
# In many languages, prompts are richer than a bare ">". Python's REPL shows
# ">>> " for new statements and "... " when it expects continuation lines
# (e.g., the body of a function). IRB (Ruby's REPL) shows the class name.
# GHCi (Haskell) shows the module name.
#
# Our REPL framework supports two kinds of prompts:
#
#   global_prompt()   — shown at the start of every NEW expression; also
#                       called the "primary prompt" or "PS1" in shell parlance
#
#   line_prompt()     — shown when the REPL is waiting for ADDITIONAL lines
#                       to complete an expression; the "secondary prompt" or
#                       "PS2"
#
# A Language can tell the REPL whether the current input buffer is "complete"
# or needs more lines. (Our simple EchoLanguage treats every line as complete,
# so only the global prompt is used in practice.)
#
# # The Interface Contract
#
# To be a valid Prompt object, implement two methods that take no arguments
# and return strings:
#
#   global_prompt() → string   (e.g., "> ")
#   line_prompt()   → string   (e.g., "... ")
#
# # Design note: why not just a hashref?
#
# We could pass the prompt strings directly:
#
#   run(prompt => '> ', continuation => '... ')
#
# But objects give us room to grow — a prompt could consult the current
# interpreter state, change colour based on the time of day, or show the
# depth of an open bracket count. A plain string cannot do any of that.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → Prompt instance
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# global_prompt() → string
#
# The primary prompt, shown before the user types a brand-new expression.
#
# @return   A string to print, e.g. "> " or "ruby> ".
# ----------------------------------------------------------------------------
sub global_prompt {
    return '> ';
}

# ----------------------------------------------------------------------------
# line_prompt() → string
#
# The secondary/continuation prompt, shown when the REPL is waiting for
# additional input to complete the current expression.
#
# @return   A string to print, e.g. "... " or "     ".
# ----------------------------------------------------------------------------
sub line_prompt {
    return '... ';
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::Prompt - Prompt interface for the REPL framework

=head1 SYNOPSIS

    package MyPrompt;
    use parent 'CodingAdventures::Repl::Prompt';

    sub global_prompt { return 'myrepl> ' }
    sub line_prompt   { return '      > ' }

=head1 DESCRIPTION

Duck-typing interface for REPL prompts. Implement two no-argument methods:

=over 4

=item C<global_prompt()>

Primary prompt shown at the start of each new expression.

=item C<line_prompt()>

Secondary/continuation prompt shown when awaiting more input lines.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
