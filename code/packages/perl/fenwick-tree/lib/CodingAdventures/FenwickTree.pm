package CodingAdventures::FenwickTree;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

sub _require_integer {
    my ($value, $name) = @_;
    die "$name must be an integer\n"
        if !defined($value) || !looks_like_number($value) || int($value) != $value;
}

sub _lowbit {
    my ($index) = @_;
    return $index & -$index;
}

sub new {
    my ($class, $size) = @_;
    _require_integer($size, 'size');
    die "size must be non-negative\n" if $size < 0;

    return bless {
        n   => $size,
        bit => [(0) x ($size + 1)],
    }, $class;
}

sub from_list {
    my ($class, $values) = @_;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';

    my $tree = $class->new(scalar @$values);
    for my $index (1 .. $tree->{n}) {
        my $value = $values->[$index - 1];
        die "value at index $index must be a number\n"
            if !defined($value) || !looks_like_number($value);
        $tree->{bit}[$index] += $value;
        my $parent = $index + _lowbit($index);
        $tree->{bit}[$parent] += $tree->{bit}[$index] if $parent <= $tree->{n};
    }
    return $tree;
}

sub _check_index {
    my ($self, $index) = @_;
    _require_integer($index, 'index');
    die "index $index out of range [1, $self->{n}]\n"
        if $index < 1 || $index > $self->{n};
}

sub update {
    my ($self, $index, $delta) = @_;
    $self->_check_index($index);
    die "delta must be a number\n"
        if !defined($delta) || !looks_like_number($delta);

    for (my $current = $index; $current <= $self->{n}; $current += _lowbit($current)) {
        $self->{bit}[$current] += $delta;
    }
    return $self;
}

sub prefix_sum {
    my ($self, $index) = @_;
    _require_integer($index, 'index');
    die "prefix index $index out of range [0, $self->{n}]\n"
        if $index < 0 || $index > $self->{n};

    my $total = 0;
    for (my $current = $index; $current > 0; $current -= _lowbit($current)) {
        $total += $self->{bit}[$current];
    }
    return $total;
}

sub range_sum {
    my ($self, $left, $right) = @_;
    _require_integer($left, 'left');
    _require_integer($right, 'right');
    die "left must be <= right\n" if $left > $right;
    $self->_check_index($left);
    $self->_check_index($right);
    return $self->prefix_sum($right) - $self->prefix_sum($left - 1);
}

sub point_query {
    my ($self, $index) = @_;
    $self->_check_index($index);
    return $self->range_sum($index, $index);
}

sub find_kth {
    my ($self, $target) = @_;
    die "find_kth called on empty tree\n" if $self->{n} == 0;
    die "target must be positive\n"
        if !defined($target) || !looks_like_number($target) || $target <= 0;

    my $total = $self->prefix_sum($self->{n});
    die "target exceeds total sum\n" if $target > $total;

    my $index = 0;
    my $step = 1;
    $step *= 2 while $step * 2 <= $self->{n};

    while ($step > 0) {
        my $next = $index + $step;
        if ($next <= $self->{n} && $self->{bit}[$next] < $target) {
            $index = $next;
            $target -= $self->{bit}[$index];
        }
        $step = int($step / 2);
    }
    return $index + 1;
}

sub size {
    my ($self) = @_;
    return $self->{n};
}

sub len {
    my ($self) = @_;
    return $self->{n};
}

sub is_empty {
    my ($self) = @_;
    return $self->{n} == 0;
}

sub bit_array {
    my ($self) = @_;
    return [] if $self->{n} == 0;
    return [@{ $self->{bit} }[1 .. $self->{n}]];
}

sub to_string {
    my ($self) = @_;
    return sprintf('FenwickTree(n=%d, bit=[%s])', $self->{n}, join(', ', @{ $self->bit_array }));
}

1;
