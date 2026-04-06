package CodingAdventures::Repl::Language;

# ============================================================================
# CodingAdventures::Repl::Language — The Language interface
# ============================================================================
#
# # What is a "Language" in the REPL framework?
#
# A REPL (Read-Eval-Print Loop) is a kind of interactive shell. You type
# something in, the REPL evaluates it, prints the result, and then loops back
# to wait for the next input. The "language" is what decides how to EVALUATE
# each piece of input.
#
# Think of it as a plug socket: the REPL framework provides the socket (the
# loop, the I/O, the wait animation), and a Language object is the plug (the
# evaluation logic). You can swap out the plug without changing the socket.
#
# # The Interface Contract (Duck Typing)
#
# Perl does not have formal interfaces the way Java or Go does. Instead, it
# uses "duck typing": if an object responds to the right method calls, it
# QUACKS like an interface and IS that interface, regardless of what class
# it belongs to.
#
# To be a valid Language for this framework, an object must implement exactly
# one method:
#
#   eval($input_string) → return value
#
# The eval method MUST return one of:
#
#   'quit'             — a bare string literal; signals the loop to exit cleanly
#
#   ['ok', $output]    — an array reference with tag 'ok'; $output is a string
#                        (or undef) that will be printed to the user
#
#   ['error', $msg]    — an array reference with tag 'error'; $msg is an error
#                        message string that will be printed to the user
#
# # Why array references instead of objects?
#
# Array references like ['ok', $value] are Perl's lightweight discriminated
# unions. They are cheap, printable, and require no class setup. They are
# sometimes called "tagged tuples" — the first element is the "tag" (the kind
# of result), and subsequent elements are the payload.
#
# This pattern will be familiar if you have used Haskell (Either / Maybe),
# Rust (Result / Option), or Elixir ({:ok, value} / {:error, reason}).
#
# # Exception Safety
#
# The REPL loop wraps every call to eval() inside a Perl eval{} block to catch
# any exceptions. If your eval() method dies, the loop catches the error and
# presents it to the user rather than crashing the whole REPL. You do NOT need
# to add eval{} inside your own eval() method — the framework does it for you.
#
# However, it is still GOOD PRACTICE to return ['error', $msg] for expected
# error conditions (parse errors, type mismatches, etc.) and let Perl's die()
# propagate only for truly unexpected failures.
#
# # Example implementation skeleton
#
#   package MyLanguage;
#
#   sub new { bless {}, shift }
#
#   sub eval {
#       my ($self, $input) = @_;
#       return 'quit' if $input eq ':quit';
#       # ... interpret $input ...
#       return ['ok', $result];
#   }
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → Language instance
#
# Base "class". In most cases Language subclasses call their own bless, but
# this base new() exists as a documentation anchor and for completeness.
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# ----------------------------------------------------------------------------
# eval($input) → 'quit' | ['ok', $output] | ['error', $msg]
#
# Evaluate one line (or multi-line block) of user input.
#
# @param $input   A string of user input, already stripped of the trailing
#                 newline by the REPL loop.
# @return         See the interface contract described above.
# ----------------------------------------------------------------------------
sub eval {
    my ($self, $input) = @_;
    # Base class: always return an error — subclasses must override.
    return ['error', 'Language::eval() not implemented'];
}

1;

__END__

=head1 NAME

CodingAdventures::Repl::Language - Language interface for the REPL framework

=head1 SYNOPSIS

    package MyLanguage;
    use parent 'CodingAdventures::Repl::Language';

    sub eval {
        my ($self, $input) = @_;
        return 'quit' if $input eq ':quit';
        return ['ok', uc($input)];   # echo in uppercase
    }

=head1 DESCRIPTION

Duck-typing interface. Implement C<eval($input)> to plug a language into the
REPL framework.  The method must return one of:

=over 4

=item C<'quit'>

The string literal C<'quit'>. Signals the REPL loop to exit cleanly.

=item C<['ok', $output]>

An array reference. C<$output> is a string (or undef for silent success)
that will be printed to the user.

=item C<['error', $message]>

An array reference. C<$message> is an error string printed to the user.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
