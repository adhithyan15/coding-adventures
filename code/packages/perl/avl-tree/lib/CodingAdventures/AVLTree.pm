package CodingAdventures::AVLTree::Node;

use strict;
use warnings;

sub _is_node {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa(__PACKAGE__) };
}

sub _height {
    my ($root) = @_;
    return defined($root) ? $root->{height} : -1;
}

sub _size {
    my ($root) = @_;
    return defined($root) ? $root->{size} : 0;
}

sub new {
    my ($class, $value, $left, $right, $height, $size) = @_;
    die "node value must be defined\n" if !defined $value;
    die "left must be a CodingAdventures::AVLTree::Node or undef\n"
        if defined($left) && !_is_node($left);
    die "right must be a CodingAdventures::AVLTree::Node or undef\n"
        if defined($right) && !_is_node($right);

    my $actual_height = defined($height)
        ? $height
        : 1 + (_height($left) > _height($right) ? _height($left) : _height($right));
    die "height must be a non-negative integer\n"
        if $actual_height !~ /^\d+$/;

    my $actual_size = defined($size)
        ? $size
        : 1 + _size($left) + _size($right);
    die "size must be a non-negative integer\n"
        if $actual_size !~ /^\d+$/;

    return bless {
        value  => $value,
        left   => $left,
        right  => $right,
        height => 0 + $actual_height,
        size   => 0 + $actual_size,
    }, $class;
}

sub value  { return $_[0]->{value}; }
sub left   { return $_[0]->{left}; }
sub right  { return $_[0]->{right}; }
sub height { return $_[0]->{height}; }
sub size   { return $_[0]->{size}; }

package CodingAdventures::AVLTree;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

sub _is_node {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa('CodingAdventures::AVLTree::Node') };
}

sub _default_compare {
    my ($left, $right) = @_;
    return $left <=> $right if looks_like_number($left) && looks_like_number($right);
    return "$left" cmp "$right";
}

sub _node_height {
    my ($root) = @_;
    return defined($root) ? $root->{height} : -1;
}

sub _node_size {
    my ($root) = @_;
    return defined($root) ? $root->{size} : 0;
}

sub _node {
    my ($value, $left, $right) = @_;
    return CodingAdventures::AVLTree::Node->new($value, $left, $right);
}

sub new {
    my ($class, $root, $compare) = @_;
    die "root must be a CodingAdventures::AVLTree::Node or undef\n"
        if defined($root) && !_is_node($root);
    $compare = \&_default_compare if !defined $compare;
    die "compare must be a code reference\n" if ref($compare) ne 'CODE';
    return bless {root => $root, compare => $compare}, $class;
}

sub empty {
    my ($class, $compare) = @_;
    return $class->new(undef, $compare);
}

sub from_values {
    my ($class, $values, $compare) = @_;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';
    my $tree = $class->empty($compare);
    for my $index (0 .. $#$values) {
        die "value at index $index must be defined\n" if !defined $values->[$index];
        $tree = $tree->insert($values->[$index]);
    }
    return $tree;
}

sub root    { return $_[0]->{root}; }
sub compare { return $_[0]->{compare}; }

sub _balance_factor {
    my ($root) = @_;
    return _node_height($root->{left}) - _node_height($root->{right});
}

sub _rotate_left {
    my ($root) = @_;
    my $right = $root->{right};
    return $root if !defined $right;
    my $new_left = _node($root->{value}, $root->{left}, $right->{left});
    return _node($right->{value}, $new_left, $right->{right});
}

sub _rotate_right {
    my ($root) = @_;
    my $left = $root->{left};
    return $root if !defined $left;
    my $new_right = _node($root->{value}, $left->{right}, $root->{right});
    return _node($left->{value}, $left->{left}, $new_right);
}

sub _rebalance {
    my ($root) = @_;
    my $factor = _balance_factor($root);
    if ($factor > 1) {
        my $left = $root->{left};
        $left = _rotate_left($left) if defined($left) && _balance_factor($left) < 0;
        return _rotate_right(_node($root->{value}, $left, $root->{right}));
    }
    if ($factor < -1) {
        my $right = $root->{right};
        $right = _rotate_right($right) if defined($right) && _balance_factor($right) > 0;
        return _rotate_left(_node($root->{value}, $root->{left}, $right));
    }
    return $root;
}

sub insert {
    my ($self, $value) = @_;
    die "value must be defined\n" if !defined $value;
    return __PACKAGE__->new(_insert_node($self->{root}, $value, $self->{compare}), $self->{compare});
}

sub _insert_node {
    my ($root, $value, $compare) = @_;
    return CodingAdventures::AVLTree::Node->new($value) if !defined $root;
    my $order = $compare->($value, $root->{value});
    return _rebalance(_node($root->{value}, _insert_node($root->{left}, $value, $compare), $root->{right}))
        if $order < 0;
    return _rebalance(_node($root->{value}, $root->{left}, _insert_node($root->{right}, $value, $compare)))
        if $order > 0;
    return $root;
}

sub delete {
    my ($self, $value) = @_;
    die "value must be defined\n" if !defined $value;
    return __PACKAGE__->new(_delete_node($self->{root}, $value, $self->{compare}), $self->{compare});
}

sub _delete_node {
    my ($root, $value, $compare) = @_;
    return undef if !defined $root;
    my $order = $compare->($value, $root->{value});
    return _rebalance(_node($root->{value}, _delete_node($root->{left}, $value, $compare), $root->{right}))
        if $order < 0;
    return _rebalance(_node($root->{value}, $root->{left}, _delete_node($root->{right}, $value, $compare)))
        if $order > 0;
    return $root->{right} if !defined $root->{left};
    return $root->{left} if !defined $root->{right};

    my ($new_right, $successor) = _extract_min($root->{right});
    return _rebalance(_node($successor, $root->{left}, $new_right));
}

sub _extract_min {
    my ($root) = @_;
    return ($root->{right}, $root->{value}) if !defined $root->{left};
    my ($new_left, $minimum) = _extract_min($root->{left});
    return (_rebalance(_node($root->{value}, $new_left, $root->{right})), $minimum);
}

sub search {
    my ($self, $value) = @_;
    my $current = $self->{root};
    while (defined $current) {
        my $order = $self->{compare}->($value, $current->{value});
        if ($order < 0) {
            $current = $current->{left};
        } elsif ($order > 0) {
            $current = $current->{right};
        } else {
            return $current;
        }
    }
    return undef;
}

sub contains {
    my ($self, $value) = @_;
    return defined $self->search($value);
}

sub min_value {
    my ($self) = @_;
    my $current = $self->{root};
    $current = $current->{left} while defined($current) && defined($current->{left});
    return defined($current) ? $current->{value} : undef;
}

sub max_value {
    my ($self) = @_;
    my $current = $self->{root};
    $current = $current->{right} while defined($current) && defined($current->{right});
    return defined($current) ? $current->{value} : undef;
}

sub predecessor {
    my ($self, $value) = @_;
    my $current = $self->{root};
    my $best;
    while (defined $current) {
        if ($self->{compare}->($value, $current->{value}) <= 0) {
            $current = $current->{left};
        } else {
            $best = $current->{value};
            $current = $current->{right};
        }
    }
    return $best;
}

sub successor {
    my ($self, $value) = @_;
    my $current = $self->{root};
    my $best;
    while (defined $current) {
        if ($self->{compare}->($value, $current->{value}) >= 0) {
            $current = $current->{right};
        } else {
            $best = $current->{value};
            $current = $current->{left};
        }
    }
    return $best;
}

sub kth_smallest {
    my ($self, $k) = @_;
    return _kth_smallest($self->{root}, $k);
}

sub _kth_smallest {
    my ($root, $k) = @_;
    return undef
        if !defined($root) || !defined($k) || !looks_like_number($k) || $k <= 0 || int($k) != $k;
    my $left_size = _node_size($root->{left});
    return $root->{value} if $k == $left_size + 1;
    return _kth_smallest($root->{left}, $k) if $k <= $left_size;
    return _kth_smallest($root->{right}, $k - $left_size - 1);
}

sub rank {
    my ($self, $value) = @_;
    return _rank($self->{root}, $value, $self->{compare});
}

sub _rank {
    my ($root, $value, $compare) = @_;
    return 0 if !defined $root;
    my $order = $compare->($value, $root->{value});
    return _rank($root->{left}, $value, $compare) if $order < 0;
    return _node_size($root->{left}) + 1 + _rank($root->{right}, $value, $compare)
        if $order > 0;
    return _node_size($root->{left});
}

sub to_sorted_array {
    my ($self) = @_;
    my @out;
    _append_inorder($self->{root}, \@out);
    return \@out;
}

sub _append_inorder {
    my ($root, $out) = @_;
    return if !defined $root;
    _append_inorder($root->{left}, $out);
    push @$out, $root->{value};
    _append_inorder($root->{right}, $out);
}

sub is_valid_bst {
    my ($self) = @_;
    return _validate_bst($self->{root}, undef, undef, 0, 0, $self->{compare});
}

sub _validate_bst {
    my ($root, $minimum, $maximum, $has_minimum, $has_maximum, $compare) = @_;
    return 1 if !defined $root;
    return 0 if $has_minimum && $compare->($root->{value}, $minimum) <= 0;
    return 0 if $has_maximum && $compare->($root->{value}, $maximum) >= 0;
    return _validate_bst($root->{left}, $minimum, $root->{value}, $has_minimum, 1, $compare)
        && _validate_bst($root->{right}, $root->{value}, $maximum, 1, $has_maximum, $compare);
}

sub is_valid_avl {
    my ($self) = @_;
    return defined _validate_avl($self->{root}, undef, undef, 0, 0, $self->{compare});
}

sub _validate_avl {
    my ($root, $minimum, $maximum, $has_minimum, $has_maximum, $compare) = @_;
    return [-1, 0] if !defined $root;
    return undef if $has_minimum && $compare->($root->{value}, $minimum) <= 0;
    return undef if $has_maximum && $compare->($root->{value}, $maximum) >= 0;

    my $left = _validate_avl($root->{left}, $minimum, $root->{value}, $has_minimum, 1, $compare);
    return undef if !defined $left;
    my $right = _validate_avl($root->{right}, $root->{value}, $maximum, 1, $has_maximum, $compare);
    return undef if !defined $right;

    my $height = 1 + ($left->[0] > $right->[0] ? $left->[0] : $right->[0]);
    my $size = 1 + $left->[1] + $right->[1];
    return undef if $root->{height} != $height;
    return undef if $root->{size} != $size;
    return undef if abs($left->[0] - $right->[0]) > 1;
    return [$height, $size];
}

sub balance_factor {
    my ($self, $root) = @_;
    return 0 if !defined $root;
    die "node must be a CodingAdventures::AVLTree::Node or undef\n" if !_is_node($root);
    return _balance_factor($root);
}

sub height {
    my ($self) = @_;
    return _node_height($self->{root});
}

sub size {
    my ($self) = @_;
    return _node_size($self->{root});
}

sub to_string {
    my ($self) = @_;
    my $root = defined($self->{root}) ? $self->{root}{value} : 'undef';
    return sprintf('AVLTree(root=%s, height=%d, size=%d)', $root, $self->height, $self->size);
}

1;
