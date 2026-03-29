package CodingAdventures::Tree;

# ============================================================================
# CodingAdventures::Tree — Classic tree data structures in Pure Perl
# ============================================================================
#
# This module implements three foundational tree data structures:
#
#   1. BST  — Binary Search Tree: sorted storage with O(log n) search
#   2. MinHeap — Priority queue backed by a binary heap
#   3. Trie — Prefix tree for efficient string/word operations
#
# Each structure is implemented as a Perl class using plain hash-based OO
# (blessed hashrefs). No CPAN dependencies; no XS; fully portable.
#
# # Why Trees?
#
# Arrays are great for indexed access but slow for searching (O(n) scan).
# Hash tables are fast for exact lookup but can't do range queries or
# sorted iteration. Trees fill the gap:
#
#   * BST   → sorted order, range queries, min/max in O(log n)
#   * Heap  → constant-time access to the minimum (or maximum) element
#   * Trie  → linear-time string operations, efficient prefix search
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Package CodingAdventures::Tree::BST
# ============================================================================
#
# A Binary Search Tree (BST) is a rooted binary tree where every node stores
# a value, and the following invariant holds:
#
#   For any node N with value V:
#     - Every value in N's left subtree  is LESS THAN V
#     - Every value in N's right subtree is GREATER THAN V
#
# This invariant makes searching, insertion, and deletion all O(log n) on
# average (O(n) worst case for a degenerate/sorted-input tree).
#
# Example tree after inserting [5, 3, 7, 1, 4]:
#
#         5         ← root
#        / \
#       3   7
#      / \
#     1   4
#
# Traversals produce different orderings:
#   Inorder    (L-Root-R): 1, 3, 4, 5, 7  ← always sorted for BSTs!
#   Preorder   (Root-L-R): 5, 3, 1, 4, 7  ← useful for tree copying
#   Postorder  (L-R-Root): 1, 4, 3, 7, 5  ← useful for deletion

package CodingAdventures::Tree::BST;

use strict;
use warnings;

# ----------------------------------------------------------------------------
# new() → BST instance
#
# Creates an empty BST. The internal structure uses nested hashrefs:
#
#   Node: { val => $value, left => undef|node, right => undef|node }
#
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless { root => undef, size => 0 }, $class;
}

# ----------------------------------------------------------------------------
# insert($val) → void
#
# Insert a value into the BST maintaining the BST invariant.
#
# Algorithm: start at root, go left if val < current, right if val > current,
# stop when we find an empty slot. Duplicates are silently ignored.
#
# @param $val  The value to insert (compared with numeric <=>)
# ----------------------------------------------------------------------------
sub insert {
    my ($self, $val) = @_;
    my $inserted = 0;
    $self->{root} = _insert_node($self->{root}, $val, \$inserted);
    $self->{size}++ if $inserted;
}

sub _insert_node {
    my ($node, $val, $inserted_ref) = @_;
    # Base case: empty slot — create a new leaf node here
    unless (defined $node) {
        $$inserted_ref = 1;
        return { val => $val, left => undef, right => undef };
    }
    my $cmp = $val <=> $node->{val};
    if    ($cmp < 0) { $node->{left}  = _insert_node($node->{left},  $val, $inserted_ref) }
    elsif ($cmp > 0) { $node->{right} = _insert_node($node->{right}, $val, $inserted_ref) }
    # cmp == 0: duplicate → ignore (BSTs typically store unique values)
    return $node;
}

# ----------------------------------------------------------------------------
# search($val) → 1 or 0
#
# Check whether $val exists in the tree.
#
# Algorithm: just like insert, but instead of placing a node, we report
# whether we find the target value. O(log n) average.
#
# @param $val  The value to search for
# @return 1 if found, 0 if not found
# ----------------------------------------------------------------------------
sub search {
    my ($self, $val) = @_;
    return _search_node($self->{root}, $val);
}

sub _search_node {
    my ($node, $val) = @_;
    return 0 unless defined $node;
    my $cmp = $val <=> $node->{val};
    return 1 if $cmp == 0;
    return $cmp < 0
        ? _search_node($node->{left},  $val)
        : _search_node($node->{right}, $val);
}

# ----------------------------------------------------------------------------
# delete($val) → void
#
# Remove a value from the tree while maintaining the BST invariant.
#
# Deletion has three cases:
#
#   Case 1 — Node has NO children (is a leaf):
#     Simply remove the node. Easy!
#
#   Case 2 — Node has ONE child:
#     Replace the node with its only child.
#
#   Case 3 — Node has TWO children:
#     We can't just remove the node — we must find a replacement that
#     preserves BST order. We use the *inorder successor*: the smallest
#     value in the right subtree. That value is "just barely" larger than
#     the deleted value, so substituting it maintains BST order.
#     Then we delete the inorder successor from the right subtree.
#
#   Example: delete 3 from [5, 3, 7, 1, 4]
#
#       Before:          After (inorder successor of 3 is 4):
#           5                5
#          / \              / \
#         3   7    →       4   7
#        / \              /
#       1   4            1
#
# @param $val  The value to delete (no-op if not found)
# ----------------------------------------------------------------------------
sub delete {
    my ($self, $val) = @_;
    my $found = 0;
    $self->{root} = _delete_node($self->{root}, $val, \$found);
    $self->{size}-- if $found;
}

sub _delete_node {
    my ($node, $val, $found_ref) = @_;
    return undef unless defined $node;

    my $cmp = $val <=> $node->{val};
    if ($cmp < 0) {
        $node->{left}  = _delete_node($node->{left},  $val, $found_ref);
    } elsif ($cmp > 0) {
        $node->{right} = _delete_node($node->{right}, $val, $found_ref);
    } else {
        # Found the node to delete
        $$found_ref = 1;

        # Case 1 & 2: zero or one child — return the other child (or undef)
        return $node->{right} unless defined $node->{left};
        return $node->{left}  unless defined $node->{right};

        # Case 3: two children — find inorder successor (min of right subtree)
        my $successor = _find_min($node->{right});
        $node->{val}   = $successor->{val};
        my $dummy = 0;
        $node->{right} = _delete_node($node->{right}, $successor->{val}, \$dummy);
    }
    return $node;
}

# Return the node with the minimum value in a subtree (leftmost node).
sub _find_min {
    my ($node) = @_;
    $node = $node->{left} while defined $node->{left};
    return $node;
}

# ----------------------------------------------------------------------------
# inorder() → @values
#
# Return all values in sorted ascending order (Left → Root → Right).
# This is the most useful traversal for a BST because it produces sorted output.
#
# @return list of values in sorted order
# ----------------------------------------------------------------------------
sub inorder {
    my ($self) = @_;
    my @result;
    _inorder_node($self->{root}, \@result);
    return @result;
}

sub _inorder_node {
    my ($node, $result) = @_;
    return unless defined $node;
    _inorder_node($node->{left},  $result);
    push @$result, $node->{val};
    _inorder_node($node->{right}, $result);
}

# ----------------------------------------------------------------------------
# preorder() → @values
#
# Return all values in preorder (Root → Left → Right).
# Useful for serializing/copying a tree: you can recreate the tree by
# inserting values in preorder.
#
# @return list of values in preorder
# ----------------------------------------------------------------------------
sub preorder {
    my ($self) = @_;
    my @result;
    _preorder_node($self->{root}, \@result);
    return @result;
}

sub _preorder_node {
    my ($node, $result) = @_;
    return unless defined $node;
    push @$result, $node->{val};
    _preorder_node($node->{left},  $result);
    _preorder_node($node->{right}, $result);
}

# ----------------------------------------------------------------------------
# postorder() → @values
#
# Return all values in postorder (Left → Right → Root).
# Useful for bottom-up operations like computing subtree sizes or
# safely freeing memory (children before parents).
#
# @return list of values in postorder
# ----------------------------------------------------------------------------
sub postorder {
    my ($self) = @_;
    my @result;
    _postorder_node($self->{root}, \@result);
    return @result;
}

sub _postorder_node {
    my ($node, $result) = @_;
    return unless defined $node;
    _postorder_node($node->{left},  $result);
    _postorder_node($node->{right}, $result);
    push @$result, $node->{val};
}

# ----------------------------------------------------------------------------
# to_sorted_array() → arrayref
#
# Convenience wrapper around inorder() that returns an array reference.
#
# @return arrayref of values in sorted order
# ----------------------------------------------------------------------------
sub to_sorted_array {
    my ($self) = @_;
    my @vals = $self->inorder();
    return \@vals;
}

# ----------------------------------------------------------------------------
# size() → integer
#
# Return the number of values currently stored in the tree.
#
# @return integer
# ----------------------------------------------------------------------------
sub size {
    my ($self) = @_;
    return $self->{size};
}

# ============================================================================
# Package CodingAdventures::Tree::MinHeap
# ============================================================================
#
# A Min-Heap is a complete binary tree where every node's value is LESS THAN
# OR EQUAL TO its children's values. The minimum element is always at the root,
# giving O(1) access to the minimum.
#
# # Array Representation
#
# We store the heap as a Perl array (0-indexed). For a node at index i:
#
#   Left child  = 2*i + 1
#   Right child = 2*i + 2
#   Parent      = floor((i-1) / 2)
#
# This mapping avoids pointer overhead and gives excellent cache performance
# (all nodes are contiguous in memory).
#
# Example for [1, 3, 5, 7, 9, 6]:
#
#        1          (index 0)
#       / \
#      3   5        (indices 1, 2)
#     / \ /
#    7  9 6         (indices 3, 4, 5)
#
# # Operations
#
#   push($val)  — Insert: add to the end, then "sift up" (bubble up) until
#                 the heap property is restored. O(log n).
#
#   pop()       — Remove minimum: swap root with last element, shrink array,
#                 then "sift down" (bubble down) to restore heap property. O(log n).
#
#   peek()      — Return the minimum without removing it. O(1).

package CodingAdventures::Tree::MinHeap;

use strict;
use warnings;
use POSIX qw(floor);

# ----------------------------------------------------------------------------
# new() → MinHeap instance
#
# Creates an empty min-heap backed by an array.
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless { heap => [] }, $class;
}

# Helper: parent index for node at index $i
sub _parent { return int(($_[0] - 1) / 2) }

# Helper: left child index
sub _left   { return 2 * $_[0] + 1 }

# Helper: right child index
sub _right  { return 2 * $_[0] + 2 }

# ----------------------------------------------------------------------------
# push($val) → void
#
# Insert a value into the heap.
#
# Algorithm:
#   1. Append the value to the end of the array.
#   2. Sift up: compare with parent, swap if smaller, repeat until
#      at the root or no swap needed.
#
# @param $val  The value to insert (compared with <=>)
# ----------------------------------------------------------------------------
sub push {
    my ($self, $val) = @_;
    my $heap = $self->{heap};
    CORE::push @$heap, $val;

    # Sift up: bubble the newly-added element toward the root while it is
    # smaller than its parent.
    my $i = $#$heap;
    while ($i > 0) {
        my $parent = _parent($i);
        if ($heap->[$i] < $heap->[$parent]) {
            # Swap with parent — bringing the smaller value closer to the root
            @{$heap}[$i, $parent] = @{$heap}[$parent, $i];
            $i = $parent;
        } else {
            last;   # heap property satisfied — stop
        }
    }
}

# ----------------------------------------------------------------------------
# pop() → $val or undef
#
# Remove and return the minimum value (the root).
#
# Algorithm:
#   1. Save the root (minimum).
#   2. Move the last element to the root position.
#   3. Shrink the array by 1.
#   4. Sift down: compare with children, swap with the smaller child if
#      the current node is larger, repeat until at a leaf or no swap needed.
#
# @return The minimum value, or undef if the heap is empty
# ----------------------------------------------------------------------------
sub pop {
    my ($self) = @_;
    my $heap = $self->{heap};
    return undef unless @$heap;

    # Special case: only one element
    return CORE::pop @$heap if @$heap == 1;

    my $min = $heap->[0];

    # Move last element to root position and remove from end
    $heap->[0] = CORE::pop @$heap;

    # Sift down: restore heap property from the root downward
    my $i = 0;
    my $n = scalar @$heap;

    while (1) {
        my $left    = _left($i);
        my $right   = _right($i);
        my $smallest = $i;

        # Find the smallest among current node and its children
        if ($left < $n && $heap->[$left] < $heap->[$smallest]) {
            $smallest = $left;
        }
        if ($right < $n && $heap->[$right] < $heap->[$smallest]) {
            $smallest = $right;
        }

        last if $smallest == $i;   # already in the right place

        # Swap current with the smaller child
        @{$heap}[$i, $smallest] = @{$heap}[$smallest, $i];
        $i = $smallest;
    }

    return $min;
}

# ----------------------------------------------------------------------------
# peek() → $val or undef
#
# Return the minimum value without removing it. O(1).
#
# @return The minimum value, or undef if the heap is empty
# ----------------------------------------------------------------------------
sub peek {
    my ($self) = @_;
    return $self->{heap}[0];
}

# ----------------------------------------------------------------------------
# size() → integer
#
# Return the number of elements in the heap.
#
# @return integer
# ----------------------------------------------------------------------------
sub size {
    my ($self) = @_;
    return scalar @{$self->{heap}};
}

# ============================================================================
# Package CodingAdventures::Tree::Trie
# ============================================================================
#
# A Trie (also called a Prefix Tree or Radix Tree) stores strings character
# by character. Each node represents one character position; a path from root
# to a marked node spells out a complete word.
#
# # Structure
#
#   Each node is a hashref:
#     { children => { 'a' => node, 'b' => node, ... },
#       is_end   => 0 or 1 }
#
#   The root node doesn't represent any character — it's the starting point.
#
# # Example: inserting "cat", "car", "card"
#
#   root
#   └── c
#       └── a
#           ├── t  [is_end=1]   ← "cat"
#           └── r  [is_end=1]   ← "car"
#               └── d  [is_end=1]  ← "card"
#
# # Why Tries?
#
#   - Search is O(m) where m is the length of the string — independent of
#     how many strings are stored (unlike a hash which has O(m) worst-case
#     for hashing, plus collision handling).
#   - Prefix queries ("all words starting with 'car'") are O(m+k) where k
#     is the number of results — try doing that with a hash!
#   - Autocomplete, spell-checking, and IP routing all use trie-like structures.

package CodingAdventures::Tree::Trie;

use strict;
use warnings;

# Helper: create a new trie node
sub _new_node {
    return { children => {}, is_end => 0 };
}

# ----------------------------------------------------------------------------
# new() → Trie instance
#
# Creates a Trie with an empty root node.
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless { root => _new_node() }, $class;
}

# ----------------------------------------------------------------------------
# insert($word) → void
#
# Insert a word into the trie.
#
# Algorithm: walk down the trie one character at a time. If a child for the
# current character doesn't exist, create it. After processing all characters,
# mark the final node as is_end=1.
#
# @param $word  String to insert
# ----------------------------------------------------------------------------
sub insert {
    my ($self, $word) = @_;
    my $node = $self->{root};
    for my $ch (split //, $word) {
        # Create child node if it doesn't exist yet
        $node->{children}{$ch} //= _new_node();
        $node = $node->{children}{$ch};
    }
    $node->{is_end} = 1;   # mark the end of this word
}

# ----------------------------------------------------------------------------
# search($word) → 1 or 0
#
# Check whether $word exists in the trie (exact match).
#
# Algorithm: walk down the trie character by character. If any character
# has no corresponding child, the word is not present. If we exhaust all
# characters, check is_end — the path might exist but not represent a whole
# word (e.g., "car" exists in the trie, but "ca" may not if "ca" was not
# inserted independently).
#
# @param $word  String to search for
# @return 1 if found as a complete word, 0 otherwise
# ----------------------------------------------------------------------------
sub search {
    my ($self, $word) = @_;
    my $node = $self->{root};
    for my $ch (split //, $word) {
        return 0 unless exists $node->{children}{$ch};
        $node = $node->{children}{$ch};
    }
    return $node->{is_end} ? 1 : 0;
}

# ----------------------------------------------------------------------------
# starts_with($prefix) → 1 or 0
#
# Check whether any inserted word starts with $prefix.
#
# This is the key operation that makes tries valuable for autocomplete: we
# walk down the trie along the prefix characters. If we reach a node for every
# character of the prefix, at least one word with that prefix exists.
#
# @param $prefix  The prefix string to check
# @return 1 if any word starts with $prefix, 0 otherwise
# ----------------------------------------------------------------------------
sub starts_with {
    my ($self, $prefix) = @_;
    my $node = $self->{root};
    for my $ch (split //, $prefix) {
        return 0 unless exists $node->{children}{$ch};
        $node = $node->{children}{$ch};
    }
    return 1;   # reached the end of prefix → at least one word exists
}

# ============================================================================
# Back to main package
# ============================================================================

package CodingAdventures::Tree;

1;

__END__

=head1 NAME

CodingAdventures::Tree - Classic tree data structures in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::Tree;

    # Binary Search Tree
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5); $bst->insert(3); $bst->insert(7);
    my @sorted = $bst->inorder();  # (3, 5, 7)
    print $bst->search(3);         # 1
    $bst->delete(3);

    # Min-Heap
    my $heap = CodingAdventures::Tree::MinHeap->new();
    $heap->push(5); $heap->push(1); $heap->push(3);
    print $heap->peek();  # 1
    print $heap->pop();   # 1

    # Trie
    my $trie = CodingAdventures::Tree::Trie->new();
    $trie->insert('cat'); $trie->insert('car');
    print $trie->search('cat');        # 1
    print $trie->starts_with('ca');    # 1
    print $trie->search('ca');         # 0 (not inserted as whole word)

=head1 DESCRIPTION

Three classic tree data structures:

=over 4

=item * C<CodingAdventures::Tree::BST> — Binary Search Tree

=item * C<CodingAdventures::Tree::MinHeap> — Min-heap priority queue

=item * C<CodingAdventures::Tree::Trie> — Prefix tree for strings

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
