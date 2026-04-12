package CodingAdventures::BPlusTree;

# BPlusTree.pm — B+ Tree (DT12) implementation in Perl
# ======================================================
#
# A B+ Tree is a refinement of the B-Tree with two structural differences:
#
# 1. Internal nodes hold ONLY keys (no values).
#    All (key, value) pairs live exclusively in leaf nodes.
#    Internal nodes are a pure routing index.
#
# 2. Leaf nodes form a singly-linked list.
#    Each leaf has a 'next' pointer to the next leaf in key order.
#    Range scans walk this linked list without touching internal nodes.
#
# ASCII diagram — B+ Tree with t=2, 5 entries:
#
#   Internal:        [3]
#                   /    \
#   Leaves:  [1,2] ──▶ [3,4,5]
#             ↑↑         ↑↑↑
#           values       values
#
# Key 3 appears in BOTH the internal node AND the right leaf.
#
# Leaf nodes:
#   keys[]   — data keys in sorted order
#   values[] — values for each key (parallel array)
#   next     — reference to next leaf, or undef
#   is_leaf  => 1
#
# Internal nodes:
#   keys[]    — separator keys (routing only)
#   children  — array of child node refs (length = keys+1)
#   next      — undef (only leaves have next pointers)
#   is_leaf   => 0
#
# Leaf split (full leaf has 2t-1 keys):
#   Left  keeps first half
#   Right keeps second half
#   First key of right half is COPIED up into parent (stays in leaf too).
#
# Internal split (same as B-Tree):
#   Median key is MOVED up into parent.

use strict;
use warnings;

our $VERSION = '0.1.0';

# ─────────────────────────────────────────────────────────────────────────────
# Constructor
# ─────────────────────────────────────────────────────────────────────────────

sub new {
    my ($class, %args) = @_;
    my $t = $args{t} // 2;
    die "t must be >= 2" unless $t >= 2;
    my $leaf = _new_leaf();
    return bless {
        t          => $t,
        root       => $leaf,
        first_leaf => $leaf,
        count      => 0,
    }, $class;
}

sub _new_leaf {
    return { keys => [], values => [], is_leaf => 1, next => undef };
}

sub _new_internal {
    return { keys => [], children => [], is_leaf => 0, next => undef };
}

# ─────────────────────────────────────────────────────────────────────────────
# Public API
# ─────────────────────────────────────────────────────────────────────────────

sub search {
    my ($self, $key) = @_;
    my $leaf = _find_leaf($self->{root}, $key);
    my $i    = _find_key_index($leaf, $key);
    return ($i < @{$leaf->{keys}} && $leaf->{keys}[$i] == $key)
        ? $leaf->{values}[$i]
        : undef;
}

sub insert {
    my ($self, $key, $value) = @_;
    my $t    = $self->{t};
    my $root = $self->{root};

    if (@{$root->{keys}} == 2 * $t - 1) {
        my $new_root = _new_internal();
        push @{$new_root->{children}}, $root;
        _split_child($new_root, 0, $t);
        $self->{root} = $new_root;
        $root = $new_root;
    }
    my $inserted = _insert_non_full($root, $key, $value, $t);
    $self->{count}++ if $inserted;
    # Update first_leaf pointer (in case a split created a new leftmost leaf).
    $self->{first_leaf} = _leftmost_leaf($self->{root});
}

sub delete {
    my ($self, $key) = @_;
    return 0 unless $self->{count} > 0;
    my $found = _delete($self->{root}, $key, $self->{t});
    if ($found) {
        $self->{count}--;
        if (@{$self->{root}{keys}} == 0 && !$self->{root}{is_leaf}) {
            $self->{root} = $self->{root}{children}[0];
        }
    }
    $self->{first_leaf} = _leftmost_leaf($self->{root});
    return $found ? 1 : 0;
}

sub size   { $_[0]->{count} }
sub height { _height($_[0]->{root}) }

sub min_key {
    my ($self) = @_;
    return $self->{first_leaf}{keys}[0];
}

sub max_key {
    my ($self) = @_;
    my $leaf = $self->{first_leaf};
    while (defined $leaf->{next}) { $leaf = $leaf->{next} }
    return $leaf->{keys}[-1];
}

# range_scan( $low, $high )
# Efficient range scan using the leaf linked list.  O(log n + k).
sub range_scan {
    my ($self, $low, $high) = @_;
    my @result;
    my $leaf = _find_leaf($self->{root}, $low);
    while (defined $leaf) {
        for my $i (0 .. $#{$leaf->{keys}}) {
            last if $leaf->{keys}[$i] > $high;
            if ($leaf->{keys}[$i] >= $low) {
                push @result, [$leaf->{keys}[$i], $leaf->{values}[$i]];
            }
        }
        $leaf = $leaf->{next};
    }
    return @result;
}

# full_scan() — walks the entire leaf linked list.  O(n).
sub full_scan {
    my ($self) = @_;
    my @result;
    my $leaf = $self->{first_leaf};
    while (defined $leaf) {
        for my $i (0 .. $#{$leaf->{keys}}) {
            push @result, [$leaf->{keys}[$i], $leaf->{values}[$i]];
        }
        $leaf = $leaf->{next};
    }
    return @result;
}

# inorder() — same as full_scan for B+ Trees (all data is in leaves).
sub inorder { goto &full_scan }

sub is_valid {
    my ($self) = @_;
    return 0 unless _is_valid(
        $self->{root}, undef, undef,
        $self->height, 0, 1, $self->{t}
    );
    return _is_linked_list_valid($self->{first_leaf}, $self->{count});
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — helpers
# ─────────────────────────────────────────────────────────────────────────────

# _find_key_index( $node, $key )
# First index i where keys[i] >= key.
sub _find_key_index {
    my ($node, $key) = @_;
    my $i = 0;
    $i++ while $i < @{$node->{keys}} && $node->{keys}[$i] < $key;
    return $i;
}

# _child_index( $internal_node, $key )
# For a B+ Tree internal node, go to children[i] where i is the number of
# separator keys <= key.
sub _child_index {
    my ($node, $key) = @_;
    my $i = 0;
    $i++ while $i < @{$node->{keys}} && $key >= $node->{keys}[$i];
    return $i;
}

sub _height {
    my ($node) = @_;
    return 0 if $node->{is_leaf};
    return 1 + _height($node->{children}[0]);
}

sub _leftmost_leaf {
    my ($node) = @_;
    while (!$node->{is_leaf}) { $node = $node->{children}[0] }
    return $node;
}

sub _find_leaf {
    my ($node, $key) = @_;
    while (!$node->{is_leaf}) {
        my $ci = _child_index($node, $key);
        $node = $node->{children}[$ci];
    }
    return $node;
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — split
# ─────────────────────────────────────────────────────────────────────────────

sub _split_child {
    my ($parent, $ci, $t) = @_;
    my $child = $parent->{children}[$ci];
    if ($child->{is_leaf}) {
        _split_leaf($parent, $ci);
    } else {
        _split_internal($parent, $ci, $t);
    }
}

sub _split_leaf {
    my ($parent, $ci) = @_;
    my $left  = $parent->{children}[$ci];
    my $right = _new_leaf();

    my $total = scalar @{$left->{keys}};
    my $mid   = int($total / 2);

    $right->{keys}   = [ @{$left->{keys}}[$mid .. $#{ $left->{keys}}]   ];
    $right->{values} = [ @{$left->{values}}[$mid .. $#{ $left->{values}}] ];
    $left->{keys}    = [ @{$left->{keys}}[0 .. $mid - 1]   ];
    $left->{values}  = [ @{$left->{values}}[0 .. $mid - 1] ];

    # Fix linked list: left → right → old left.next
    $right->{next} = $left->{next};
    $left->{next}  = $right;

    # Separator = first key of right leaf (stays in right leaf too).
    my $sep = $right->{keys}[0];
    splice @{$parent->{keys}},     $ci,     0, $sep;
    splice @{$parent->{children}}, $ci + 1, 0, $right;
}

sub _split_internal {
    my ($parent, $ci, $t) = @_;
    my $full  = $parent->{children}[$ci];
    my $right = _new_internal();

    # Median key MOVES up into parent.
    my $mk = $full->{keys}[$t - 1];

    $right->{keys}     = [ @{$full->{keys}}[$t .. $#{$full->{keys}}]         ];
    $right->{children} = [ @{$full->{children}}[$t .. $#{$full->{children}}] ];
    $full->{keys}      = [ @{$full->{keys}}[0 .. $t - 2]      ];
    $full->{children}  = [ @{$full->{children}}[0 .. $t - 1]  ];

    splice @{$parent->{keys}},     $ci,     0, $mk;
    splice @{$parent->{children}}, $ci + 1, 0, $right;
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — insert
# ─────────────────────────────────────────────────────────────────────────────

sub _insert_non_full {
    my ($node, $key, $value, $t) = @_;
    if ($node->{is_leaf}) {
        my $i = _find_key_index($node, $key);
        if ($i < @{$node->{keys}} && $node->{keys}[$i] == $key) {
            $node->{values}[$i] = $value;
            return 0;
        }
        splice @{$node->{keys}},   $i, 0, $key;
        splice @{$node->{values}}, $i, 0, $value;
        return 1;
    }

    my $ci = _child_index($node, $key);
    $ci = $#{$node->{children}} if $ci > $#{$node->{children}};

    if (@{$node->{children}[$ci]{keys}} == 2 * $t - 1) {
        _split_child($node, $ci, $t);
        if ($ci < @{$node->{keys}} && $key >= $node->{keys}[$ci]) { $ci++ }
    }
    $ci = $#{$node->{children}} if $ci > $#{$node->{children}};
    return _insert_non_full($node->{children}[$ci], $key, $value, $t);
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — delete
# ─────────────────────────────────────────────────────────────────────────────

sub _delete {
    my ($node, $key, $t) = @_;
    if ($node->{is_leaf}) {
        my $i = _find_key_index($node, $key);
        return 0 unless $i < @{$node->{keys}} && $node->{keys}[$i] == $key;
        splice @{$node->{keys}},   $i, 1;
        splice @{$node->{values}}, $i, 1;
        return 1;
    }

    my $ci = _child_index($node, $key);
    $ci = $#{$node->{children}} if $ci > $#{$node->{children}};

    my $new_ci = _prepare_child($node, $ci, $t);
    my $safe   = ($new_ci <= $#{$node->{children}}) ? $new_ci : $#{$node->{children}};
    return _delete($node->{children}[$safe], $key, $t);
}

sub _prepare_child {
    my ($parent, $i, $t) = @_;
    my $child = $parent->{children}[$i];
    return $i if @{$child->{keys}} >= $t;

    my $has_left  = $i > 0;
    my $has_right = $i < $#{$parent->{children}};

    if ($has_left && @{$parent->{children}[$i - 1]{keys}} >= $t) {
        _borrow_from_left($parent, $i);
        return $i;
    } elsif ($has_right && @{$parent->{children}[$i + 1]{keys}} >= $t) {
        _borrow_from_right($parent, $i);
        return $i;
    } elsif ($has_left) {
        _merge_children($parent, $i - 1, $t);
        return $i - 1;
    } else {
        _merge_children($parent, $i, $t);
        return $i;
    }
}

sub _borrow_from_left {
    my ($parent, $i) = @_;
    my $child = $parent->{children}[$i];
    my $left  = $parent->{children}[$i - 1];
    if ($child->{is_leaf}) {
        unshift @{$child->{keys}},   pop @{$left->{keys}};
        unshift @{$child->{values}}, pop @{$left->{values}};
        $parent->{keys}[$i - 1] = $child->{keys}[0];
    } else {
        unshift @{$child->{keys}},     $parent->{keys}[$i - 1];
        unshift @{$child->{children}}, pop @{$left->{children}};
        $parent->{keys}[$i - 1] = pop @{$left->{keys}};
    }
}

sub _borrow_from_right {
    my ($parent, $i) = @_;
    my $child = $parent->{children}[$i];
    my $right = $parent->{children}[$i + 1];
    if ($child->{is_leaf}) {
        push @{$child->{keys}},   shift @{$right->{keys}};
        push @{$child->{values}}, shift @{$right->{values}};
        $parent->{keys}[$i] = $right->{keys}[0];
    } else {
        push @{$child->{keys}},     $parent->{keys}[$i];
        push @{$child->{children}}, shift @{$right->{children}};
        $parent->{keys}[$i] = shift @{$right->{keys}};
    }
}

sub _merge_children {
    my ($parent, $i, $t) = @_;
    my $left  = $parent->{children}[$i];
    my $right = $parent->{children}[$i + 1];
    if ($left->{is_leaf}) {
        push @{$left->{keys}},   @{$right->{keys}};
        push @{$left->{values}}, @{$right->{values}};
        $left->{next} = $right->{next};
    } else {
        push @{$left->{keys}},     $parent->{keys}[$i];
        push @{$left->{keys}},     @{$right->{keys}};
        push @{$left->{children}}, @{$right->{children}};
    }
    splice @{$parent->{keys}},     $i,     1;
    splice @{$parent->{children}}, $i + 1, 1;
}

# ─────────────────────────────────────────────────────────────────────────────
# Private — validation
# ─────────────────────────────────────────────────────────────────────────────

sub _is_valid {
    my ($node, $min_key, $max_key, $expected_depth, $depth, $is_root, $t) = @_;

    if ($node->{is_leaf}) {
        return 0 unless $depth == $expected_depth;
        return 0 unless @{$node->{keys}} == @{$node->{values}};
        # Keys must be sorted.
        for my $i (1 .. $#{$node->{keys}}) {
            return 0 if $node->{keys}[$i] <= $node->{keys}[$i - 1];
        }
        # Leaf key bounds (for B+ Tree we check leaf keys against bounds).
        if (defined $min_key) {
            return 0 if @{$node->{keys}} && $node->{keys}[0] < $min_key;
        }
        if (defined $max_key) {
            return 0 if @{$node->{keys}} && $node->{keys}[-1] > $max_key;
        }
    } else {
        # Internal nodes must have no values.
        return 0 if exists $node->{values} && @{$node->{values}};
        return 0 unless @{$node->{children}} == @{$node->{keys}} + 1;
        # Separators must be sorted.
        for my $i (1 .. $#{$node->{keys}}) {
            return 0 if $node->{keys}[$i] <= $node->{keys}[$i - 1];
        }
    }

    my $min_keys = $is_root ? 0 : ($t - 1);
    my $n = scalar @{$node->{keys}};
    return 0 if $n < $min_keys || $n > 2 * $t - 1;

    unless ($node->{is_leaf}) {
        for my $i (0 .. $n) {
            my $cmin = $i == 0 ? $min_key : $node->{keys}[$i - 1];
            my $cmax = $i == $n ? $max_key : $node->{keys}[$i];
            return 0 unless _is_valid(
                $node->{children}[$i], $cmin, $cmax,
                $expected_depth, $depth + 1, 0, $t
            );
        }
    }
    return 1;
}

sub _is_linked_list_valid {
    my ($first_leaf, $expected_count) = @_;
    my $total    = 0;
    my $prev_key = undef;
    my $leaf     = $first_leaf;
    while (defined $leaf) {
        for my $i (0 .. $#{$leaf->{keys}}) {
            if ($i > 0 && $leaf->{keys}[$i] <= $leaf->{keys}[$i - 1]) { return 0 }
            if (defined $prev_key && $leaf->{keys}[$i] <= $prev_key)   { return 0 }
            $prev_key = $leaf->{keys}[$i];
            $total++;
        }
        $leaf = $leaf->{next};
    }
    return $total == $expected_count ? 1 : 0;
}

1;
__END__

=head1 NAME

CodingAdventures::BPlusTree - B+ Tree (DT12) in Perl

=head1 SYNOPSIS

    use CodingAdventures::BPlusTree;

    my $tree = CodingAdventures::BPlusTree->new(t => 2);
    $tree->insert(1, "one");
    $tree->insert(2, "two");
    $tree->insert(3, "three");

    $tree->search(2);           # "two"
    my @r = $tree->range_scan(1, 2);   # ([1,"one"],[2,"two"])
    my @all = $tree->full_scan;

=head1 DESCRIPTION

B+ Tree that maps numeric keys to arbitrary Perl values.
All data lives in leaf nodes; internal nodes are routing indexes.
Leaf nodes are connected by a linked list for efficient range scans.

=head1 METHODS

=over 4

=item new( t => $t )

Create an empty B+ Tree with minimum degree C<$t> (default 2).

=item insert( $key, $value )

Insert or update C<$key>.

=item delete( $key )

Remove C<$key>. Returns 1 if found, 0 if absent.

=item search( $key )

Return value for C<$key> or C<undef>.

=item size(), height()

Cardinality and height.

=item min_key(), max_key()

Minimum and maximum keys.

=item range_scan( $low, $high )

All C<[key, value]> pairs where C<$low E<lt>= $key E<lt>= $high>.
Uses the leaf linked list.

=item full_scan()

All C<[key, value]> pairs via the leaf linked list.

=item inorder()

Alias for C<full_scan>.

=item is_valid()

Return 1 if all structural invariants and linked-list integrity hold.

=back

=cut
