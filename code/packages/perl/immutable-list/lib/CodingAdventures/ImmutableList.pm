package CodingAdventures::ImmutableList;

# ============================================================================
# CodingAdventures::ImmutableList — Persistent immutable linked list
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# WHAT IS AN IMMUTABLE LIST?
# --------------------------
# A traditional Perl array is mutable: you push, pop, splice, and the same
# array changes in place.  An immutable list is different: every "modification"
# returns a *brand-new* list object, leaving the original untouched.
#
# This is the linked-list variant made famous by Lisp and Haskell.  Each node
# has exactly two fields:
#
#   head  — the value stored at this position
#   tail  — a reference to the rest of the list (another node, or the empty
#            sentinel)
#
# Diagrammatically, the list [3, 2, 1] looks like:
#
#   +--------+    +--------+    +--------+    +-------+
#   | head=3 | -> | head=2 | -> | head=1 | -> | empty |
#   +--------+    +--------+    +--------+    +-------+
#
# STRUCTURAL SHARING
# ------------------
# Because nodes are never mutated, two lists can share tail nodes.  Given:
#
#   my $l1 = empty->cons(1);           # [1]
#   my $l2 = $l1->cons(2);             # [2, 1]  — l2 shares l1's node
#   my $l3 = $l2->cons(3);             # [3, 2, 1] — l3 shares l2 and l1
#
# No memory is wasted copying; existing nodes are reused.
#
# Usage:
#
#   use CodingAdventures::ImmutableList;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# The empty sentinel
# ----------------------------------------------------------------------------
# We use a module-level singleton.  Every call to empty() returns the SAME
# blessed hashref.  This means `$a->is_empty` is a simple key-existence check
# rather than a reference comparison.
#
# The singleton is stored in $_EMPTY and initialised lazily on first use.

my $_EMPTY;

# _make_empty — construct (once) the canonical empty-list object.
sub _make_empty {
    return $_EMPTY if defined $_EMPTY;
    $_EMPTY = bless { _empty => 1 }, __PACKAGE__;
    return $_EMPTY;
}

# empty() — public constructor for the empty list.
#
#   my $e = CodingAdventures::ImmutableList->empty();
#
# Returns the singleton empty node.  Because it is a singleton you can safely
# use `==` to compare two lists for "both empty":
#
#   $a == $b   # true only if both are the empty sentinel
sub empty {
    return _make_empty();
}

# ----------------------------------------------------------------------------
# cons($value) — prepend a value, returning a new list
# ----------------------------------------------------------------------------
# "cons" comes from Lisp.  It constructs a new node whose head is $value and
# whose tail is $self (the list being prepended to).
#
#   my $l = empty->cons(1)->cons(2)->cons(3);
#   # l is [3, 2, 1] — last cons call wins the head position
#
# Because we do NOT modify $self, $self remains valid and can be used
# independently:
#
#   my $base  = empty->cons(1)->cons(2);  # [2, 1]
#   my $left  = $base->cons(10);          # [10, 2, 1]
#   my $right = $base->cons(20);          # [20, 2, 1]
#   # $base is still [2, 1] — neither $left nor $right changed it
sub cons {
    my ($self, $value) = @_;
    return bless { head => $value, tail => $self }, __PACKAGE__;
}

# ----------------------------------------------------------------------------
# head() — return the first element
# ----------------------------------------------------------------------------
# Calling head() on the empty list is a programming error; we die loudly.
sub head {
    my ($self) = @_;
    die "head() called on empty list" if $self->is_empty;
    return $self->{head};
}

# ----------------------------------------------------------------------------
# tail() — return everything after the first element
# ----------------------------------------------------------------------------
# Returns another ImmutableList (possibly the empty sentinel).
sub tail {
    my ($self) = @_;
    die "tail() called on empty list" if $self->is_empty;
    return $self->{tail};
}

# ----------------------------------------------------------------------------
# is_empty() — predicate
# ----------------------------------------------------------------------------
sub is_empty {
    my ($self) = @_;
    return exists $self->{_empty} ? 1 : 0;
}

# ----------------------------------------------------------------------------
# length() — count the nodes
# ----------------------------------------------------------------------------
# Walks the list, so O(n).  Immutable lists are not designed for random access;
# use arrays when you need O(1) length.
sub length {
    my ($self) = @_;
    my $count = 0;
    my $node  = $self;
    while (!$node->is_empty) {
        $count++;
        $node = $node->{tail};
    }
    return $count;
}

# ----------------------------------------------------------------------------
# to_array() — materialise as a plain Perl list
# ----------------------------------------------------------------------------
# Returns a list in the same order as iteration: head first.
#
#   my $l = empty->cons(1)->cons(2)->cons(3);  # [3,2,1]
#   my @a = $l->to_array();                    # (3, 2, 1)
sub to_array {
    my ($self) = @_;
    my @result;
    my $node = $self;
    while (!$node->is_empty) {
        push @result, $node->{head};
        $node = $node->{tail};
    }
    return @result;
}

# ----------------------------------------------------------------------------
# from_array(@values) — class method, build a list from a Perl array
# ----------------------------------------------------------------------------
# from_array(1, 2, 3) builds a list where the *first* argument is the head:
#
#   head=1 -> head=2 -> head=3 -> empty
#
# Implementation: we cons from right to left so the final head is the first
# element of @values.
#
#   from_array(1, 2, 3)
#   = cons(1, cons(2, cons(3, empty)))
#   = [1, 2, 3]
sub from_array {
    my ($class, @values) = @_;
    my $list = _make_empty();
    # Walk backwards: cons(last), cons(second-to-last), …, cons(first)
    for my $val (reverse @values) {
        $list = $list->cons($val);
    }
    return $list;
}

# ----------------------------------------------------------------------------
# append($other) — concatenate two lists
# ----------------------------------------------------------------------------
# Returns a new list: all elements of $self, followed by all elements of
# $other.  Neither $self nor $other is mutated.
#
# Example:
#   [3,2,1]->append([5,4]) => [3,2,1,5,4]
#
# Implementation: convert $self to array, prepend each element (in reverse)
# onto $other.
sub append {
    my ($self, $other) = @_;
    # Collect self's elements, then build right-to-left onto $other
    my @mine = $self->to_array;
    my $result = $other;
    for my $val (reverse @mine) {
        $result = $result->cons($val);
    }
    return $result;
}

# ----------------------------------------------------------------------------
# reverse() — reverse the list
# ----------------------------------------------------------------------------
# Returns a new list with elements in reverse order.
#
#   [3,2,1]->reverse()  =>  [1,2,3]
sub reverse {
    my ($self) = @_;
    my $result = _make_empty();
    my $node   = $self;
    while (!$node->is_empty) {
        $result = $result->cons($node->{head});
        $node   = $node->{tail};
    }
    return $result;
}

# ----------------------------------------------------------------------------
# map($fn) — apply a function to every element
# ----------------------------------------------------------------------------
# $fn receives one argument (the element) and must return the transformed
# value.  A new list is returned; elements are in the same order.
#
#   [3,2,1]->map(sub { $_[0] * 2 })  =>  [6,4,2]
sub map {
    my ($self, $fn) = @_;
    # Build in reverse order by reversing twice, or collect and reconstruct
    my @vals  = $self->to_array;
    my @mapped = CORE::map { $fn->($_) } @vals;
    return __PACKAGE__->from_array(@mapped);
}

# ----------------------------------------------------------------------------
# filter($pred) — keep elements matching a predicate
# ----------------------------------------------------------------------------
# $pred receives one argument and must return a truthy/falsy value.
#
#   [3,2,1]->filter(sub { $_[0] > 1 })  =>  [3,2]
sub filter {
    my ($self, $pred) = @_;
    my @vals      = $self->to_array;
    my @filtered  = grep { $pred->($_) } @vals;
    return __PACKAGE__->from_array(@filtered);
}

# ----------------------------------------------------------------------------
# foldl($init, $fn) — left fold (reduce)
# ----------------------------------------------------------------------------
# Starting from $init, repeatedly apply $fn($accumulator, $element).
#
#   [3,2,1]->foldl(0, sub { $_[0] + $_[1] })  =>  6
#
# The "l" in foldl means we process elements left-to-right (head first).
# This is the same as Perl's List::Util::reduce with an explicit initial
# value.
sub foldl {
    my ($self, $init, $fn) = @_;
    my $acc  = $init;
    my $node = $self;
    while (!$node->is_empty) {
        $acc  = $fn->($acc, $node->{head});
        $node = $node->{tail};
    }
    return $acc;
}

# ----------------------------------------------------------------------------
# nth($n) — 1-based element access
# ----------------------------------------------------------------------------
# nth(1) returns head, nth(2) returns head of tail, etc.
# Dies if $n is out of range.
sub nth {
    my ($self, $n) = @_;
    die "nth() index must be >= 1" if $n < 1;
    my $node = $self;
    for my $i (1 .. $n) {
        die "nth($n) out of range" if $node->is_empty;
        return $node->{head} if $i == $n;
        $node = $node->{tail};
    }
}

1;

__END__

=head1 NAME

CodingAdventures::ImmutableList - Persistent immutable linked list with structural sharing

=head1 SYNOPSIS

    use CodingAdventures::ImmutableList;

    my $empty = CodingAdventures::ImmutableList->empty();
    my $l1 = $empty->cons(1);          # [1]
    my $l2 = $l1->cons(2);             # [2, 1]  (head is 2)
    my $l3 = $l2->cons(3);             # [3, 2, 1]

    $l3->head();                        # 3
    $l3->tail();                        # list object [2, 1]
    $l3->is_empty();                    # 0
    $empty->is_empty();                 # 1
    $l3->length();                      # 3
    my @arr = $l3->to_array();          # (3, 2, 1)

    my $l4 = CodingAdventures::ImmutableList->from_array(1, 2, 3);
    my $l5 = $l3->map(sub { $_[0] * 2 });
    my $l6 = $l3->filter(sub { $_[0] > 1 });
    my $sum = $l3->foldl(0, sub { $_[0] + $_[1] }); # 6
    $l3->nth(1);  # 3

=head1 DESCRIPTION

An immutable linked list built without any CPAN dependencies.  Every operation
that "modifies" the list actually returns a brand-new list, leaving the
original untouched.  Because nodes are never mutated, multiple lists can share
their tail segments (structural sharing), keeping memory use low.

The design mirrors the classic Lisp cons-cell model:

=over 4

=item * C<empty()> — the empty sentinel (singleton)

=item * C<cons($val)> — prepend a value, O(1)

=item * C<head()> / C<tail()> — destructure, O(1)

=item * C<length()>, C<to_array()>, C<append()>, C<reverse()> — O(n)

=item * C<map()>, C<filter()>, C<foldl()> — O(n)

=item * C<nth($n)> — O(n)

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
