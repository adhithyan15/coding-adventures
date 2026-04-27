package CodingAdventures::CompilerIr::IDGenerator;

# ============================================================================
# CodingAdventures::CompilerIr::IDGenerator — monotonic unique ID counter
# ============================================================================
#
# Every IR instruction in the pipeline needs a unique ID for source
# mapping.  The IDGenerator ensures no two instructions ever share an ID,
# even across multiple compiler invocations within the same process.
#
# ## Why unique IDs matter
#
# The source map chain links every machine code byte back to the source
# character that caused it.  The link is: source → AST node → IR ID →
# machine code offset.  If two instructions shared an ID, the chain would
# be ambiguous — you wouldn't know which instruction the machine code
# came from.
#
# ## Usage
#
#   my $gen = CodingAdventures::CompilerIr::IDGenerator->new();
#   my $id1 = $gen->next;    # 0
#   my $id2 = $gen->next;    # 1
#   my $id3 = $gen->next;    # 2
#   my $cur  = $gen->current; # 3 (next value, not incremented)
#
# ## Starting from a non-zero value
#
# When multiple compilers contribute instructions to the same program,
# use new_from() to start the counter at a value that won't collide:
#
#   my $gen2 = CodingAdventures::CompilerIr::IDGenerator->new_from(100);
#   my $id = $gen2->next;    # 100
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new() — create a new generator starting at 0.
sub new {
    my ($class) = @_;
    return bless { _next => 0 }, $class;
}

# new_from($start) — create a new generator starting at $start.
#
# Useful when composing multiple compiler passes that must not share IDs.
sub new_from {
    my ($class, $start) = @_;
    return bless { _next => $start }, $class;
}

# next() — return the next unique ID and advance the counter.
sub next {    ## no critic (ProhibitBuiltinHomonyms)
    my ($self) = @_;
    my $id = $self->{_next};
    $self->{_next}++;
    return $id;
}

# current() — return the counter value WITHOUT incrementing it.
#
# This is the ID that will be returned by the next call to next().
# Useful when you need to record "the first ID in this batch" before
# emitting the instructions.
sub current {
    my ($self) = @_;
    return $self->{_next};
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IDGenerator - monotonic unique instruction ID counter

=head1 SYNOPSIS

  my $gen = CodingAdventures::CompilerIr::IDGenerator->new;
  my $id1 = $gen->next;     # 0
  my $id2 = $gen->next;     # 1
  my $cur  = $gen->current; # 2

  my $gen2 = CodingAdventures::CompilerIr::IDGenerator->new_from(100);
  my $id3  = $gen2->next;   # 100

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
