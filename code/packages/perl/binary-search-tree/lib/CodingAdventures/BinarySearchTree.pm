package CodingAdventures::BinarySearchTree::Node;

use strict;
use warnings;

sub _is_node {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa(__PACKAGE__) };
}

sub new {
    my ($class, $value, $left, $right, $size) = @_;
    die "node value must be defined\n" if !defined $value;
    die "left must be a CodingAdventures::BinarySearchTree::Node or undef\n"
        if defined($left) && !_is_node($left);
    die "right must be a CodingAdventures::BinarySearchTree::Node or undef\n"
        if defined($right) && !_is_node($right);

    my $actual_size = defined($size)
        ? $size
        : 1 + (defined($left) ? $left->{size} : 0) + (defined($right) ? $right->{size} : 0);
    die "size must be a non-negative integer\n"
        if $actual_size !~ /^\d+$/;

    return bless {
        value => $value,
        left  => $left,
        right => $right,
        size  => 0 + $actual_size,
    }, $class;
}

sub value { return $_[0]->{value}; }
sub left  { return $_[0]->{left}; }
sub right { return $_[0]->{right}; }
sub size  { return $_[0]->{size}; }

package CodingAdventures::BinarySearchTree;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

sub _is_node {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa('CodingAdventures::BinarySearchTree::Node') };
}

sub _default_compare {
    my ($left, $right) = @_;
    return $left <=> $right if looks_like_number($left) && looks_like_number($right);
    return "$left" cmp "$right";
}

sub _node_size {
    my ($root) = @_;
    return defined($root) ? $root->{size} : 0;
}

sub new {
    my ($class, $root, $compare) = @_;
    die "root must be a CodingAdventures::BinarySearchTree::Node or undef\n"
        if defined($root) && !_is_node($root);
    $compare = \&_default_compare if !defined $compare;
    die "compare must be a code reference\n" if ref($compare) ne 'CODE';
    return bless {root => $root, compare => $compare}, $class;
}

sub empty {
    my ($class, $compare) = @_;
    return $class->new(undef, $compare);
}

sub from_sorted_array {
    my ($class, $values, $compare) = @_;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';
    return $class->new(_build_balanced($values, 0, $#$values), $compare);
}

sub _build_balanced {
    my ($values, $first, $last) = @_;
    return undef if $first > $last;
    my $middle = int(($first + $last + 1) / 2);
    die "value at index $middle must be defined\n" if !defined $values->[$middle];
    return CodingAdventures::BinarySearchTree::Node->new(
        $values->[$middle],
        _build_balanced($values, $first, $middle - 1),
        _build_balanced($values, $middle + 1, $last),
    );
}

sub root    { return $_[0]->{root}; }
sub compare { return $_[0]->{compare}; }

sub _with_children {
    my ($root, $left, $right) = @_;
    return CodingAdventures::BinarySearchTree::Node->new($root->{value}, $left, $right);
}

sub insert {
    my ($self, $value) = @_;
    die "value must be defined\n" if !defined $value;
    return __PACKAGE__->new(_insert_node($self->{root}, $value, $self->{compare}), $self->{compare});
}

sub _insert_node {
    my ($root, $value, $compare) = @_;
    return CodingAdventures::BinarySearchTree::Node->new($value) if !defined $root;
    my $order = $compare->($value, $root->{value});
    return _with_children($root, _insert_node($root->{left}, $value, $compare), $root->{right})
        if $order < 0;
    return _with_children($root, $root->{left}, _insert_node($root->{right}, $value, $compare))
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
    return _with_children($root, _delete_node($root->{left}, $value, $compare), $root->{right})
        if $order < 0;
    return _with_children($root, $root->{left}, _delete_node($root->{right}, $value, $compare))
        if $order > 0;
    return $root->{right} if !defined $root->{left};
    return $root->{left} if !defined $root->{right};

    my ($new_right, $successor) = _extract_min($root->{right});
    return CodingAdventures::BinarySearchTree::Node->new($successor, $root->{left}, $new_right);
}

sub _extract_min {
    my ($root) = @_;
    return ($root->{right}, $root->{value}) if !defined $root->{left};
    my ($new_left, $minimum) = _extract_min($root->{left});
    return (_with_children($root, $new_left, $root->{right}), $minimum);
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

sub is_valid {
    my ($self) = @_;
    return defined _validate($self->{root}, undef, undef, 0, 0, $self->{compare});
}

sub _validate {
    my ($root, $minimum, $maximum, $has_minimum, $has_maximum, $compare) = @_;
    return [-1, 0] if !defined $root;
    return undef if $has_minimum && $compare->($root->{value}, $minimum) <= 0;
    return undef if $has_maximum && $compare->($root->{value}, $maximum) >= 0;

    my $left = _validate($root->{left}, $minimum, $root->{value}, $has_minimum, 1, $compare);
    return undef if !defined $left;
    my $right = _validate($root->{right}, $root->{value}, $maximum, 1, $has_maximum, $compare);
    return undef if !defined $right;

    my $actual_size = 1 + $left->[1] + $right->[1];
    return undef if $root->{size} != $actual_size;
    my $height = 1 + ($left->[0] > $right->[0] ? $left->[0] : $right->[0]);
    return [$height, $actual_size];
}

sub height {
    my ($self) = @_;
    return _height($self->{root});
}

sub _height {
    my ($root) = @_;
    return -1 if !defined $root;
    my $left = _height($root->{left});
    my $right = _height($root->{right});
    return 1 + ($left > $right ? $left : $right);
}

sub size {
    my ($self) = @_;
    return _node_size($self->{root});
}

sub to_string {
    my ($self) = @_;
    my $root = defined($self->{root}) ? $self->{root}{value} : 'undef';
    return sprintf('BinarySearchTree(root=%s, size=%d)', $root, $self->size);
}

1;
