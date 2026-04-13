package CodingAdventures::HuffmanTree;

use strict;
use warnings;
use CodingAdventures::Heap;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::HuffmanTree - Huffman Tree optimal prefix-free entropy coding (DT27)

=head1 SYNOPSIS

  use CodingAdventures::HuffmanTree;

  my $tree = CodingAdventures::HuffmanTree->build([
      [65, 3],   # 'A' appears 3 times
      [66, 2],   # 'B' appears 2 times
      [67, 1],   # 'C' appears 1 time
  ]);

  my $table  = $tree->code_table();      # hashref: symbol → bit string
  my $code   = $tree->code_for(65);     # single-symbol lookup
  my $canon  = $tree->canonical_code_table();
  my @decoded = $tree->decode_all("001110", 4);

  print $tree->weight();        # 6
  print $tree->depth();         # 2
  print $tree->symbol_count();  # 3
  print $tree->is_valid() ? "valid" : "invalid";

=head1 DESCRIPTION

A Huffman tree is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
variable-length bit code. Symbols that appear often get short codes; symbols
that appear rarely get long codes. The total bits needed to encode a message is
minimised — it is the theoretically optimal prefix-free code for a given symbol
frequency distribution.

Think of it like Morse code. In Morse, C<E> is C<.> (one dot) and C<Z> is
C<--..> (four symbols). The designers knew C<E> is the most common letter in
English so they gave it the shortest code. Huffman's algorithm does this
automatically and optimally for any alphabet with any frequency distribution.

=head2 Algorithm: Greedy construction via min-heap

=over 4

=item 1.

Create one leaf node per distinct symbol, each with its frequency as weight.
Push all leaves onto a min-heap keyed by priority tuple.

=item 2.

While the heap has more than one node:
  a. Pop the two nodes with the smallest weight.
  b. Create a new internal node whose weight = sum of the two children.
  c. Set left = first popped, right = second popped.
  d. Push the new internal node back.

=item 3.

The one remaining node is the root.

=back

Tie-breaking rules (for deterministic output across implementations):

=over 4

=item 1. Lowest weight pops first.

=item 2. Leaf before internal at equal weight.

=item 3. Lower symbol value among equal-weight leaves.

=item 4. Earlier-created (FIFO) among equal-weight internal nodes.

=back

=head2 Prefix-Free Property

Symbols live ONLY at leaves, never at internal nodes. The code for a symbol is
the path from root to its leaf (left edge = '0', right edge = '1'). Since one
leaf is never an ancestor of another, no code can be a prefix of another — the
bit stream can be decoded unambiguously without separators.

=head2 Canonical Codes (DEFLATE / zlib style)

Canonical codes normalise code assignment: given only code I<lengths>, the
exact codes can always be reproduced. This is how DEFLATE stores Huffman tables
— it transmits lengths, not full codes.

Algorithm:

=over 4

=item 1. Collect (symbol, code_length) pairs from the tree.

=item 2. Sort by (code_length, symbol_value).

=item 3. Assign codes numerically: first = 0, then increment and shift left
when moving to a longer length.

=back

=head2 Heap Dependency

This module depends on L<CodingAdventures::Heap> for the shared min-heap used
during greedy construction. Heap items are stored as C<[$priority, $node]>
pairs, and the comparator compares the four-element priority tuples
lexicographically.

=cut

# _INF is the infinity sentinel used for unused priority tuple fields.
# In Perl, 9**9**9 evaluates to the floating-point infinity.
use constant _INF => 9**9**9;

sub _entry_compare {
    my ($left, $right) = @_;
    my $a = $left->[0];
    my $b = $right->[0];

    for my $i (0..3) {
        return -1 if $a->[$i] < $b->[$i];
        return 1 if $a->[$i] > $b->[$i];
    }

    return 0;
}

# _new_leaf creates a leaf node.
#
# A leaf holds a single symbol and its frequency weight.
# It has no children.
#
# @param $symbol  (int) non-negative integer symbol identifier.
# @param $weight  (int) positive frequency count.
# @return (hashref) leaf node.
sub _new_leaf {
    my ($symbol, $weight) = @_;
    return { kind => 'leaf', symbol => $symbol, weight => $weight };
}

# _new_internal creates an internal node combining two sub-trees.
#
# An internal node is a routing node: it has no symbol, only children.
# Its weight is the sum of its children's weights.
# The order field is a monotonic counter used for tie-breaking.
#
# @param $left   (hashref) left child node.
# @param $right  (hashref) right child node.
# @param $order  (int)     monotonic insertion counter.
# @return (hashref) internal node.
sub _new_internal {
    my ($left, $right, $order) = @_;
    return {
        kind   => 'internal',
        weight => $left->{weight} + $right->{weight},
        left   => $left,
        right  => $right,
        order  => $order,
    };
}

# _node_priority computes the 4-element heap priority tuple for a node.
#
# Tuple layout:
#   [0] weight           — lower weight = higher priority
#   [1] leaf_flag        — 0=leaf (higher priority), 1=internal
#   [2] sym_or_inf       — leaf: symbol value; internal: +Inf
#   [3] order_or_inf     — internal: insertion order; leaf: +Inf
#
# @param $node  (hashref) a leaf or internal node.
# @return (arrayref) 4-element priority tuple.
sub _node_priority {
    my ($node) = @_;
    if ($node->{kind} eq 'leaf') {
        return [$node->{weight}, 0, $node->{symbol}, _INF];
    } else {
        return [$node->{weight}, 1, _INF, $node->{order}];
    }
}

# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------

=head1 METHODS

=head2 build

  my $tree = CodingAdventures::HuffmanTree->build(\@weights);

Constructs a Huffman tree from C<[symbol, frequency]> pairs.

C<\@weights> is an arrayref of C<[$symbol, $freq]> pairs. Each symbol must
be a non-negative integer; each frequency must be positive.

Returns a blessed C<CodingAdventures::HuffmanTree> object.

Dies if C<\@weights> is empty or any frequency is <= 0.

=cut

sub build {
    my ($class, $weights) = @_;

    die "weights must not be empty\n"
        unless defined $weights && @$weights > 0;

    for my $pair (@$weights) {
        die sprintf("frequency must be positive; got symbol=%d, freq=%d\n",
                    $pair->[0], $pair->[1])
            if $pair->[1] <= 0;
    }

    my $heap = CodingAdventures::Heap::MinHeap->new(\&_entry_compare);

    # Seed the heap with one leaf per symbol.
    for my $pair (@$weights) {
        my $leaf = _new_leaf($pair->[0], $pair->[1]);
        $heap->push([_node_priority($leaf), $leaf]);
    }

    my $order_counter = 0;

    # Merge phase: pop two smallest, create internal, push back.
    while ($heap->size() > 1) {
        my $left_entry  = $heap->pop();
        my $right_entry = $heap->pop();
        my $left = $left_entry->[1];
        my $right = $right_entry->[1];
        my $internal = _new_internal($left, $right, $order_counter);
        $order_counter++;
        $heap->push([_node_priority($internal), $internal]);
    }

    my $root = $heap->pop()->[1];

    return bless {
        _root         => $root,
        _symbol_count => scalar @$weights,
    }, $class;
}

# ---------------------------------------------------------------------------
# code_table
# ---------------------------------------------------------------------------

=head2 code_table

  my $table = $tree->code_table();

Returns a hashref C<{symbol => bit_string}> for all symbols in the tree.

Left edges are C<'0'>, right edges are C<'1'>. For a single-symbol tree the
convention is C<{symbol => '0'}> (one bit per occurrence).

Time: O(n) where n = number of distinct symbols.

=cut

sub code_table {
    my ($self) = @_;
    my %table;

    my $walk;
    $walk = sub {
        my ($node, $prefix) = @_;
        if ($node->{kind} eq 'leaf') {
            $table{$node->{symbol}} = (length($prefix) > 0 ? $prefix : '0');
            return;
        }
        $walk->($node->{left},  $prefix . '0');
        $walk->($node->{right}, $prefix . '1');
    };
    $walk->($self->{_root}, '');

    return \%table;
}

# ---------------------------------------------------------------------------
# code_for
# ---------------------------------------------------------------------------

=head2 code_for

  my $code = $tree->code_for($symbol);

Returns the bit string for a specific symbol, or C<undef> if not in the tree.

Walks the tree searching for the leaf with the given symbol; does NOT build
the full code table.

Time: O(n) worst case (full tree traversal).

=cut

sub code_for {
    my ($self, $symbol) = @_;

    my $find;
    $find = sub {
        my ($node, $prefix) = @_;
        if ($node->{kind} eq 'leaf') {
            if ($node->{symbol} == $symbol) {
                return (length($prefix) > 0 ? $prefix : '0');
            }
            return undef;
        }
        my $left_result = $find->($node->{left}, $prefix . '0');
        return $left_result if defined $left_result;
        return $find->($node->{right}, $prefix . '1');
    };

    return $find->($self->{_root}, '');
}

# ---------------------------------------------------------------------------
# canonical_code_table
# ---------------------------------------------------------------------------

=head2 canonical_code_table

  my $table = $tree->canonical_code_table();

Returns canonical Huffman codes (DEFLATE-style).

Sorted by C<(code_length, symbol_value)>; codes assigned numerically. Useful
when you need to transmit only code lengths, not the tree structure.

Time: O(n log n).

=cut

sub canonical_code_table {
    my ($self) = @_;

    # Step 1: collect lengths.
    my %lengths;

    my $collect;
    $collect = sub {
        my ($node, $depth) = @_;
        if ($node->{kind} eq 'leaf') {
            # Single-leaf: depth 0, but length is 1 by convention.
            $lengths{$node->{symbol}} = ($depth > 0 ? $depth : 1);
            return;
        }
        $collect->($node->{left},  $depth + 1);
        $collect->($node->{right}, $depth + 1);
    };
    $collect->($self->{_root}, 0);

    # Single-leaf edge case.
    if ($self->{_symbol_count} == 1) {
        my ($sym) = keys %lengths;
        return { $sym => '0' };
    }

    # Step 2: sort by (length, symbol).
    my @sorted = sort { $lengths{$a} <=> $lengths{$b} || $a <=> $b } keys %lengths;

    # Step 3: assign canonical codes numerically.
    my $code_val = 0;
    my $prev_len = $lengths{$sorted[0]};
    my %result;

    for my $sym (@sorted) {
        my $len = $lengths{$sym};
        if ($len > $prev_len) {
            $code_val <<= ($len - $prev_len);
        }
        # Format as zero-padded binary string of length $len.
        $result{$sym} = sprintf('%0*b', $len, $code_val);
        $code_val++;
        $prev_len = $len;
    }

    return \%result;
}

# ---------------------------------------------------------------------------
# decode_all
# ---------------------------------------------------------------------------

=head2 decode_all

  my @symbols = $tree->decode_all($bits, $count);

Decodes exactly C<$count> symbols from a bit string by walking the tree.

C<$bits> is a string of C<'0'> and C<'1'> characters.

Returns a list of decoded symbols of length C<$count>.

Dies if the bit stream is exhausted before C<$count> symbols are decoded.

For a single-leaf tree, each C<'0'> bit decodes to that symbol.

Time: O(total bits consumed).

=cut

sub decode_all {
    my ($self, $bits, $count) = @_;

    my @result;
    my $node        = $self->{_root};
    my $i           = 0;  # current position in $bits (0-indexed)
    my $len         = length($bits);
    my $single_leaf = ($self->{_root}{kind} eq 'leaf');

    while (@result < $count) {
        if ($node->{kind} eq 'leaf') {
            push @result, $node->{symbol};
            $node = $self->{_root};
            if ($single_leaf) {
                # Consume one '0' bit per symbol for single-leaf trees.
                $i++ if $i < $len;
            }
            # Multi-leaf: index already advanced past the last edge bit.
        } else {
            die sprintf("bit stream exhausted after %d symbols; expected %d\n",
                        scalar @result, $count)
                if $i >= $len;
            my $bit = substr($bits, $i, 1);
            $i++;
            $node = ($bit eq '0') ? $node->{left} : $node->{right};
        }
    }

    return @result;
}

# ---------------------------------------------------------------------------
# Inspection methods
# ---------------------------------------------------------------------------

=head2 weight

  my $w = $tree->weight();

Returns the total weight of the tree (= sum of all leaf frequencies = root weight).

O(1) — stored at the root.

=cut

sub weight {
    my ($self) = @_;
    return $self->{_root}{weight};
}

=head2 depth

  my $d = $tree->depth();

Returns the maximum code length (= depth of the deepest leaf).

O(n) — must traverse the tree.

=cut

sub depth {
    my ($self) = @_;

    my $max_depth;
    $max_depth = sub {
        my ($node, $d) = @_;
        return $d if $node->{kind} eq 'leaf';
        my $l = $max_depth->($node->{left},  $d + 1);
        my $r = $max_depth->($node->{right}, $d + 1);
        return $l > $r ? $l : $r;
    };

    return $max_depth->($self->{_root}, 0);
}

=head2 symbol_count

  my $n = $tree->symbol_count();

Returns the number of distinct symbols (= number of leaf nodes).

O(1) — stored at construction time.

=cut

sub symbol_count {
    my ($self) = @_;
    return $self->{_symbol_count};
}

=head2 leaves

  my @pairs = $tree->leaves();

Returns an in-order (left-to-right) traversal of all leaves.

Each element is a C<[$symbol, $code]> arrayref.

Time: O(n).

=cut

sub leaves {
    my ($self) = @_;
    my $code_tbl = $self->code_table();
    my @result;

    my $walk;
    $walk = sub {
        my ($node) = @_;
        if ($node->{kind} eq 'leaf') {
            push @result, [$node->{symbol}, $code_tbl->{$node->{symbol}}];
            return;
        }
        $walk->($node->{left});
        $walk->($node->{right});
    };
    $walk->($self->{_root});

    return @result;
}

=head2 is_valid

  my $ok = $tree->is_valid();

Checks structural invariants of the tree. Returns C<1> if valid, C<0> otherwise.

Invariants checked:

=over 4

=item 1. Every internal node has exactly 2 children (full binary tree).

=item 2. C<weight(internal) == weight(left) + weight(right)>.

=item 3. No symbol appears in more than one leaf (no duplicates).

=back

=cut

sub is_valid {
    my ($self) = @_;
    my %seen;

    my $check;
    $check = sub {
        my ($node) = @_;
        if ($node->{kind} eq 'leaf') {
            return 0 if exists $seen{$node->{symbol}};
            $seen{$node->{symbol}} = 1;
            return 1;
        }
        # Internal node must have both children.
        return 0 unless defined $node->{left} && defined $node->{right};
        # Weight invariant.
        return 0 if $node->{weight} != $node->{left}{weight} + $node->{right}{weight};
        return $check->($node->{left}) && $check->($node->{right});
    };

    return $check->($self->{_root}) ? 1 : 0;
}

1;

__END__

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
