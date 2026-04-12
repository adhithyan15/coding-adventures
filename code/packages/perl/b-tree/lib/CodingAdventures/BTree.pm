package CodingAdventures::BTree;

# BTree.pm — B-Tree (DT11) implementation in Perl
# ==================================================
#
# A B-Tree is a self-balancing search tree invented at Boeing Research Labs
# in 1970 by Rudolf Bayer and Edward McCreight.  Unlike a binary search tree
# (at most 2 children), a B-Tree node can have many children.  This "wide and
# shallow" structure keeps the tree very short — a billion-entry B-Tree with
# t=500 has a height of only 4.
#
# B-Trees power virtually every database and filesystem:
#   • SQLite, PostgreSQL, MySQL InnoDB — all use B-Trees for indexes
#   • NTFS, HFS+, ext4 — all use B-Trees for directory structures
#
# The minimum-degree t
# ─────────────────────
# Every B-Tree has a parameter t ≥ 2 called the minimum degree:
#   • Non-root nodes hold at least t-1 keys  (the "half-full" invariant)
#   • Every node holds at most 2t-1 keys     (the "not overflowing" invariant)
#   • A non-leaf with k keys has exactly k+1 children
#
# Invariants (rules that must NEVER be broken)
# ─────────────────────────────────────────────
# 1. All leaves are at the same depth.
# 2. Keys within a node are in strictly ascending order.
# 3. Non-root nodes have between t-1 and 2t-1 keys.
# 4. An internal node's i-th child contains keys strictly between
#    keys[i-1] and keys[i].
#
# Insertion — proactive top-down splitting
# ─────────────────────────────────────────
# Walk down the tree, splitting any full node we encounter BEFORE descending
# into it.  This way we never have to backtrack up.
#
# Deletion — CLRS top-down approach
# ───────────────────────────────────
# Before descending into any child, ensure it has ≥ t keys so that removing
# a key from it won't cause underflow.  Three sub-cases:
#   Case A: key in leaf → remove directly.
#   Case B: key in internal node → use predecessor/successor or merge.
#   Case C: key not in node → pre-fill child, then descend.
#
# ASCII diagram — splitting a full node with t=2:
#
#        P: [10, 30]                    P: [10, 20, 30]
#           /   |   \          →           /  |   |   \
#          A  [15,20,25]  B            A  [15] [25]  B
#                               (median 20 rises into P)

use strict;
use warnings;

our $VERSION = '0.1.0';

# ─────────────────────────────────────────────────────────────────────────────
# Constructor
# ─────────────────────────────────────────────────────────────────────────────

# new( t => $t )
#
# Creates an empty B-Tree with minimum degree $t (default 2).
# The root starts as an empty leaf.
sub new {
    my ($class, %args) = @_;
    my $t = $args{t} // 2;
    die "t must be >= 2" unless $t >= 2;
    return bless {
        t     => $t,
        root  => _new_node(1),   # root is an empty leaf
        count => 0,
    }, $class;
}

# _new_node( $is_leaf )
#
# Create a bare node hash:
#   keys     => []   sorted array of search keys
#   values   => []   parallel array of values (same index as keys)
#   children => []   child node refs (internal nodes only)
#   is_leaf  => 1/0
sub _new_node {
    my ($is_leaf) = @_;
    return {
        keys     => [],
        values   => [],
        children => [],
        is_leaf  => $is_leaf,
    };
}

# ─────────────────────────────────────────────────────────────────────────────
# Public API
# ─────────────────────────────────────────────────────────────────────────────

# search( $key )  →  $value or undef
sub search {
    my ($self, $key) = @_;
    return _search($self->{root}, $key);
}

# insert( $key, $value )
# Upsert semantics: if key exists, update its value.
sub insert {
    my ($self, $key, $value) = @_;
    my $t    = $self->{t};
    my $root = $self->{root};

    # If the root is full we must split it before inserting.
    # This is the only point where the tree grows taller.
    if (@{$root->{keys}} == 2 * $t - 1) {
        my $new_root = _new_node(0);
        push @{$new_root->{children}}, $root;
        _split_child($new_root, 0, $t);
        $self->{root} = $new_root;
        $root = $new_root;
    }
    my $inserted = _insert_non_full($root, $key, $value, $t);
    $self->{count}++ if $inserted;
}

# delete( $key )  →  1 if deleted, 0 if not found
sub delete {
    my ($self, $key) = @_;
    return 0 unless $self->{count} > 0;
    my $found = _delete($self->{root}, $key, $self->{t});
    if ($found) {
        $self->{count}--;
        # If the root has no keys but has a child, shrink the tree.
        if (@{$self->{root}{keys}} == 0 && !$self->{root}{is_leaf}) {
            $self->{root} = $self->{root}{children}[0];
        }
    }
    return $found ? 1 : 0;
}

# size()  →  number of (key, value) pairs
sub size { return $_[0]->{count} }

# height()  →  height of the tree (0 for an empty or single-leaf tree)
sub height { return _height($_[0]->{root}) }

# min_key()  →  minimum key or undef
sub min_key {
    my ($self) = @_;
    return undef unless @{$self->{root}{keys}};
    my $node = $self->{root};
    while (!$node->{is_leaf}) { $node = $node->{children}[0] }
    return $node->{keys}[0];
}

# max_key()  →  maximum key or undef
sub max_key {
    my ($self) = @_;
    return undef unless @{$self->{root}{keys}};
    my $node = $self->{root};
    while (!$node->{is_leaf}) {
        $node = $node->{children}[-1];
    }
    return $node->{keys}[-1];
}

# inorder()  →  list of [key, value] pairs in sorted order
sub inorder {
    my ($self) = @_;
    my @result;
    _inorder($self->{root}, \@result);
    return @result;
}

# range_query( $low, $high )  →  list of [key, value] pairs where low ≤ key ≤ high
sub range_query {
    my ($self, $low, $high) = @_;
    my @result;
    _range_query($self->{root}, $low, $high, \@result);
    return @result;
}

# is_valid()  →  1 if all B-Tree invariants hold, 0 otherwise
sub is_valid {
    my ($self) = @_;
    return _is_valid(
        $self->{root},
        undef, undef,
        $self->height,
        0,
        1,         # is_root
        $self->{t}
    );
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — search
# ─────────────────────────────────────────────────────────────────────────────

sub _search {
    my ($node, $key) = @_;
    my $i = _find_key_index($node, $key);
    if ($i < @{$node->{keys}} && $node->{keys}[$i] == $key) {
        return $node->{values}[$i];
    }
    return undef if $node->{is_leaf};
    return _search($node->{children}[$i], $key);
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — helpers
# ─────────────────────────────────────────────────────────────────────────────

# _find_key_index( $node, $key )
# Linear scan: return the first index i where keys[i] >= key.
sub _find_key_index {
    my ($node, $key) = @_;
    my $i = 0;
    $i++ while $i < @{$node->{keys}} && $node->{keys}[$i] < $key;
    return $i;
}

sub _height {
    my ($node) = @_;
    return 0 if $node->{is_leaf};
    return 1 + _height($node->{children}[0]);
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — split
# ─────────────────────────────────────────────────────────────────────────────

# _split_child( $parent, $child_index, $t )
#
# Split the full child at $child_index (which has 2t-1 keys).
# The median key rises into $parent; left half stays in the child,
# right half goes into a new node inserted at child_index+1.
sub _split_child {
    my ($parent, $ci, $t) = @_;
    my $full  = $parent->{children}[$ci];
    my $new   = _new_node($full->{is_leaf});

    # The median is at index t-1.
    my $median_key = $full->{keys}[$t - 1];
    my $median_val = $full->{values}[$t - 1];

    # Right half: keys and values from index t onward.
    $new->{keys}   = [ @{$full->{keys}}[$t .. $#{$full->{keys}}]     ];
    $new->{values} = [ @{$full->{values}}[$t .. $#{$full->{values}}] ];
    if (!$full->{is_leaf}) {
        $new->{children} = [ @{$full->{children}}[$t .. $#{$full->{children}}] ];
        $full->{children} = [ @{$full->{children}}[0 .. $t - 1] ];
    }

    # Trim full child to the left half.
    $full->{keys}   = [ @{$full->{keys}}[0 .. $t - 2]   ];
    $full->{values} = [ @{$full->{values}}[0 .. $t - 2] ];

    # Insert median into parent and new node into parent's children.
    splice @{$parent->{keys}},     $ci,     0, $median_key;
    splice @{$parent->{values}},   $ci,     0, $median_val;
    splice @{$parent->{children}}, $ci + 1, 0, $new;
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — insert
# ─────────────────────────────────────────────────────────────────────────────

# _insert_non_full( $node, $key, $value, $t )
# Insert into a node that is guaranteed not full.
# Returns 1 if a new key was added, 0 if an existing key was updated.
sub _insert_non_full {
    my ($node, $key, $value, $t) = @_;
    my $i = _find_key_index($node, $key);

    # Exact match → upsert.
    if ($i < @{$node->{keys}} && $node->{keys}[$i] == $key) {
        $node->{values}[$i] = $value;
        return 0;
    }

    if ($node->{is_leaf}) {
        splice @{$node->{keys}},   $i, 0, $key;
        splice @{$node->{values}}, $i, 0, $value;
        return 1;
    }

    # Internal node: pre-emptively split the child if full.
    my $ci = $i;
    if (@{$node->{children}[$ci]{keys}} == 2 * $t - 1) {
        _split_child($node, $ci, $t);
        # After split, node->{keys}[$ci] is the risen median.
        if ($key > $node->{keys}[$ci]) {
            $ci++;
        } elsif ($key == $node->{keys}[$ci]) {
            $node->{values}[$ci] = $value;
            return 0;
        }
    }
    return _insert_non_full($node->{children}[$ci], $key, $value, $t);
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — delete
# ─────────────────────────────────────────────────────────────────────────────

# _delete( $node, $key, $t )
#
# Recursively delete $key from the subtree rooted at $node.
# Returns 1 if found, 0 if not.
#
# INVARIANT: $node has ≥ t keys, or $node is the root.
sub _delete {
    my ($node, $key, $t) = @_;
    my $i    = _find_key_index($node, $key);
    my $here = ($i < @{$node->{keys}} && $node->{keys}[$i] == $key);

    if ($here) {
        if ($node->{is_leaf}) {
            # Case A: remove from leaf.
            splice @{$node->{keys}},   $i, 1;
            splice @{$node->{values}}, $i, 1;
            return 1;
        }

        my $lc = $node->{children}[$i];
        my $rc = $node->{children}[$i + 1];

        if (@{$lc->{keys}} >= $t) {
            # Case B1: left child has a spare key.
            my ($pk, $pv) = _find_max($lc);
            $node->{keys}[$i]   = $pk;
            $node->{values}[$i] = $pv;
            _delete_descend($lc, $pk, $t);
            return 1;

        } elsif (@{$rc->{keys}} >= $t) {
            # Case B2: right child has a spare key.
            my ($sk, $sv) = _find_min($rc);
            $node->{keys}[$i]   = $sk;
            $node->{values}[$i] = $sv;
            _delete_descend($rc, $sk, $t);
            return 1;

        } else {
            # Case B3: merge.
            _merge_children($node, $i);
            return _delete($node->{children}[$i], $key, $t);
        }

    } else {
        return 0 if $node->{is_leaf};

        # Case C: key is not here; descend into children[$i].
        my $ci = _prepare_child($node, $i, $t);

        # After restructuring, check if the key is now in the parent.
        my $j = _find_key_index($node, $key);
        if ($j < @{$node->{keys}} && $node->{keys}[$j] == $key) {
            return _delete($node, $key, $t);
        }

        my $safe = ($ci <= $#{$node->{children}}) ? $ci : $#{$node->{children}};
        return _delete($node->{children}[$safe], $key, $t);
    }
}

# _delete_descend — same as _delete, for Case B1/B2.
sub _delete_descend {
    my ($node, $key, $t) = @_;
    _delete($node, $key, $t);
}

# _prepare_child( $parent, $i, $t )
#
# Ensure parent->{children}[$i] has ≥ t keys.
# Returns the (possibly shifted) index of the child.
sub _prepare_child {
    my ($parent, $i, $t) = @_;
    my $child = $parent->{children}[$i];
    return $i if @{$child->{keys}} >= $t;

    my $has_left  = $i > 0;
    my $has_right = $i < $#{$parent->{children}};

    if ($has_left && @{$parent->{children}[$i - 1]{keys}} >= $t) {
        # Rotate from left sibling.
        my $left = $parent->{children}[$i - 1];
        unshift @{$child->{keys}},   $parent->{keys}[$i - 1];
        unshift @{$child->{values}}, $parent->{values}[$i - 1];
        $parent->{keys}[$i - 1]   = pop @{$left->{keys}};
        $parent->{values}[$i - 1] = pop @{$left->{values}};
        if (!$left->{is_leaf}) {
            unshift @{$child->{children}}, pop @{$left->{children}};
        }
        return $i;

    } elsif ($has_right && @{$parent->{children}[$i + 1]{keys}} >= $t) {
        # Rotate from right sibling.
        my $right = $parent->{children}[$i + 1];
        push @{$child->{keys}},   $parent->{keys}[$i];
        push @{$child->{values}}, $parent->{values}[$i];
        $parent->{keys}[$i]   = shift @{$right->{keys}};
        $parent->{values}[$i] = shift @{$right->{values}};
        if (!$right->{is_leaf}) {
            push @{$child->{children}}, shift @{$right->{children}};
        }
        return $i;

    } elsif ($has_left) {
        # Merge with left sibling.
        _merge_children($parent, $i - 1);
        return $i - 1;

    } else {
        # Merge with right sibling.
        _merge_children($parent, $i);
        return $i;
    }
}

# _merge_children( $parent, $i )
#
# Merge parent->{children}[$i+1] into parent->{children}[$i],
# pulling down parent->{keys}[$i] as separator.
sub _merge_children {
    my ($parent, $i) = @_;
    my $left  = $parent->{children}[$i];
    my $right = $parent->{children}[$i + 1];

    # Pull separator down.
    push @{$left->{keys}},   $parent->{keys}[$i];
    push @{$left->{values}}, $parent->{values}[$i];

    # Append right's keys, values, children.
    push @{$left->{keys}},     @{$right->{keys}};
    push @{$left->{values}},   @{$right->{values}};
    push @{$left->{children}}, @{$right->{children}} unless $right->{is_leaf};

    # Remove separator and right pointer from parent.
    splice @{$parent->{keys}},     $i,     1;
    splice @{$parent->{values}},   $i,     1;
    splice @{$parent->{children}}, $i + 1, 1;
}

sub _find_max {
    my ($node) = @_;
    while (!$node->{is_leaf}) { $node = $node->{children}[-1] }
    return ($node->{keys}[-1], $node->{values}[-1]);
}

sub _find_min {
    my ($node) = @_;
    while (!$node->{is_leaf}) { $node = $node->{children}[0] }
    return ($node->{keys}[0], $node->{values}[0]);
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — traversal
# ─────────────────────────────────────────────────────────────────────────────

sub _inorder {
    my ($node, $result) = @_;
    my $n = scalar @{$node->{keys}};
    for my $i (0 .. $n - 1) {
        _inorder($node->{children}[$i], $result) unless $node->{is_leaf};
        push @$result, [$node->{keys}[$i], $node->{values}[$i]];
    }
    _inorder($node->{children}[$n], $result) unless $node->{is_leaf};
}

sub _range_query {
    my ($node, $low, $high, $result) = @_;
    my $n = scalar @{$node->{keys}};
    for my $i (0 .. $n - 1) {
        if (!$node->{is_leaf} && $node->{keys}[$i] > $low) {
            _range_query($node->{children}[$i], $low, $high, $result);
        }
        if ($node->{keys}[$i] >= $low && $node->{keys}[$i] <= $high) {
            push @$result, [$node->{keys}[$i], $node->{values}[$i]];
        }
    }
    if (!$node->{is_leaf} && $n > 0 && $node->{keys}[$n - 1] < $high) {
        _range_query($node->{children}[$n], $low, $high, $result);
    }
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — validation
# ─────────────────────────────────────────────────────────────────────────────

sub _is_valid {
    my ($node, $min_key, $max_key, $expected_depth, $depth, $is_root, $t) = @_;

    if ($node->{is_leaf} && $depth != $expected_depth) { return 0 }
    if (!$node->{is_leaf} && @{$node->{children}} != @{$node->{keys}} + 1) { return 0 }

    my $min_keys = $is_root ? 0 : ($t - 1);
    my $n = scalar @{$node->{keys}};
    return 0 if $n < $min_keys || $n > 2 * $t - 1;

    for my $i (0 .. $n - 1) {
        return 0 if defined $min_key && $node->{keys}[$i] <= $min_key;
        return 0 if defined $max_key && $node->{keys}[$i] >= $max_key;
        return 0 if $i > 0 && $node->{keys}[$i] <= $node->{keys}[$i - 1];
    }

    unless ($node->{is_leaf}) {
        for my $i (0 .. $n) {
            my $cmin = $i == 0     ? $min_key : $node->{keys}[$i - 1];
            my $cmax = $i == $n    ? $max_key : $node->{keys}[$i];
            return 0 unless _is_valid(
                $node->{children}[$i], $cmin, $cmax,
                $expected_depth, $depth + 1, 0, $t
            );
        }
    }
    return 1;
}

1;
__END__

=head1 NAME

CodingAdventures::BTree - B-Tree (DT11) in Perl

=head1 SYNOPSIS

    use CodingAdventures::BTree;

    my $tree = CodingAdventures::BTree->new(t => 2);
    $tree->insert("apple",  1);
    $tree->insert("banana", 2);
    my $v = $tree->search("apple");   # 1
    $tree->delete("apple");
    my @all = $tree->inorder;         # (["banana", 2])

=head1 DESCRIPTION

A generic B-Tree that maps string keys to arbitrary Perl values.
Keys are compared with string comparison operators (lt, gt, eq).

=head1 METHODS

=over 4

=item new( t => $t )

Create an empty B-Tree with minimum degree C<$t> (default 2).

=item insert( $key, $value )

Insert or update C<$key>.

=item delete( $key )

Remove C<$key>.  Returns 1 if found, 0 if absent.

=item search( $key )

Return the value for C<$key>, or C<undef> if absent.

=item size()

Number of key-value pairs.

=item height()

Height of the tree.

=item min_key(), max_key()

Minimum and maximum keys.

=item inorder()

All C<[key, value]> pairs in sorted order.

=item range_query( $low, $high )

All C<[key, value]> pairs where C<$low le $key le $high>.

=item is_valid()

Return 1 if all B-Tree invariants hold.

=back

=cut
