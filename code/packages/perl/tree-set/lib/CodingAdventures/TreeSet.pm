package CodingAdventures::TreeSet;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use CodingAdventures::AVLTree;
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

sub _is_set {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa(__PACKAGE__) };
}

sub _require_set {
    my ($value) = @_;
    die "other must be a CodingAdventures::TreeSet\n" if !_is_set($value);
}

sub new {
    my ($class, $values, $compare) = @_;
    $values = [] if !defined $values;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';

    my $self = bless {
        backend => CodingAdventures::AVLTree->empty($compare),
    }, $class;
    for my $index (0 .. $#$values) {
        die "value at index $index must be defined\n" if !defined $values->[$index];
        $self->add($values->[$index]);
    }
    return $self;
}

sub empty {
    my ($class, $compare) = @_;
    return $class->new([], $compare);
}

sub from_values {
    my ($class, $values, $compare) = @_;
    return $class->new($values, $compare);
}

sub backend { return $_[0]->{backend}; }

sub add {
    my ($self, $value) = @_;
    die "value must be defined\n" if !defined $value;
    $self->{backend} = $self->{backend}->insert($value);
    return $self;
}

sub insert {
    my ($self, @args) = @_;
    return $self->add(@args);
}

sub delete {
    my ($self, $value) = @_;
    return 0 if !defined($value) || !$self->{backend}->contains($value);
    $self->{backend} = $self->{backend}->delete($value);
    return 1;
}

sub remove {
    my ($self, @args) = @_;
    return $self->delete(@args);
}

sub discard {
    my ($self, @args) = @_;
    return $self->delete(@args);
}

sub has {
    my ($self, $value) = @_;
    return defined($value) && $self->{backend}->contains($value);
}

sub contains {
    my ($self, @args) = @_;
    return $self->has(@args);
}

sub size   { return $_[0]->{backend}->size; }
sub length { return $_[0]->size; }
sub is_empty { return $_[0]->size == 0; }

sub min { return $_[0]->{backend}->min_value; }
sub max { return $_[0]->{backend}->max_value; }
sub first { return $_[0]->min; }
sub last { return $_[0]->max; }

sub predecessor {
    my ($self, $value) = @_;
    return $self->{backend}->predecessor($value);
}

sub successor {
    my ($self, $value) = @_;
    return $self->{backend}->successor($value);
}

sub rank {
    my ($self, $value) = @_;
    return $self->{backend}->rank($value);
}

sub by_rank {
    my ($self, $rank) = @_;
    return undef
        if !defined($rank) || !looks_like_number($rank) || $rank < 0 || int($rank) != $rank;
    return $self->{backend}->kth_smallest($rank + 1);
}

sub kth_smallest {
    my ($self, $k) = @_;
    return $self->{backend}->kth_smallest($k);
}

sub to_list {
    my ($self) = @_;
    return $self->{backend}->to_sorted_array;
}

sub to_sorted_array { return $_[0]->to_list; }
sub to_array { return $_[0]->to_list; }

sub _lower_bound {
    my ($items, $value, $compare) = @_;
    my $low = 0;
    my $high = scalar @$items;
    while ($low < $high) {
        my $middle = int(($low + $high) / 2);
        if ($compare->($items->[$middle], $value) < 0) {
            $low = $middle + 1;
        } else {
            $high = $middle;
        }
    }
    return $low;
}

sub _upper_bound {
    my ($items, $value, $compare) = @_;
    my $low = 0;
    my $high = scalar @$items;
    while ($low < $high) {
        my $middle = int(($low + $high) / 2);
        if ($compare->($items->[$middle], $value) <= 0) {
            $low = $middle + 1;
        } else {
            $high = $middle;
        }
    }
    return $low;
}

sub range {
    my ($self, $minimum, $maximum, $inclusive) = @_;
    die "minimum must be defined\n" if !defined $minimum;
    die "maximum must be defined\n" if !defined $maximum;
    $inclusive = 1 if !defined $inclusive;
    die "inclusive must be 0 or 1\n" if ref($inclusive) || $inclusive !~ /^(?:0|1)$/;

    my $compare = $self->{backend}->compare;
    return [] if $compare->($minimum, $maximum) > 0;
    my $values = $self->to_list;
    my $first = $inclusive
        ? _lower_bound($values, $minimum, $compare)
        : _upper_bound($values, $minimum, $compare);
    my $last = $inclusive
        ? _upper_bound($values, $maximum, $compare)
        : _lower_bound($values, $maximum, $compare);
    return [@$values[$first .. $last - 1]] if $first < $last;
    return [];
}

sub _merge_unique {
    my ($left, $right, $compare) = @_;
    my @result;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        if ($order < 0) {
            push @result, $left->[$left_index++];
        } elsif ($order > 0) {
            push @result, $right->[$right_index++];
        } else {
            push @result, $left->[$left_index++];
            $right_index++;
        }
    }
    push @result, @$left[$left_index .. $#$left] if $left_index < @$left;
    push @result, @$right[$right_index .. $#$right] if $right_index < @$right;
    return \@result;
}

sub _intersection_sorted {
    my ($left, $right, $compare) = @_;
    my @result;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        if ($order < 0) {
            $left_index++;
        } elsif ($order > 0) {
            $right_index++;
        } else {
            push @result, $left->[$left_index++];
            $right_index++;
        }
    }
    return \@result;
}

sub _difference_sorted {
    my ($left, $right, $compare) = @_;
    my @result;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        if ($order < 0) {
            push @result, $left->[$left_index++];
        } elsif ($order > 0) {
            $right_index++;
        } else {
            $left_index++;
            $right_index++;
        }
    }
    push @result, @$left[$left_index .. $#$left] if $left_index < @$left;
    return \@result;
}

sub _symmetric_difference_sorted {
    my ($left, $right, $compare) = @_;
    my @result;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        if ($order < 0) {
            push @result, $left->[$left_index++];
        } elsif ($order > 0) {
            push @result, $right->[$right_index++];
        } else {
            $left_index++;
            $right_index++;
        }
    }
    push @result, @$left[$left_index .. $#$left] if $left_index < @$left;
    push @result, @$right[$right_index .. $#$right] if $right_index < @$right;
    return \@result;
}

sub _is_subset_sorted {
    my ($left, $right, $compare) = @_;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        return 0 if $order < 0;
        if ($order > 0) {
            $right_index++;
        } else {
            $left_index++;
            $right_index++;
        }
    }
    return $left_index == @$left;
}

sub _is_disjoint_sorted {
    my ($left, $right, $compare) = @_;
    my ($left_index, $right_index) = (0, 0);
    while ($left_index < @$left && $right_index < @$right) {
        my $order = $compare->($left->[$left_index], $right->[$right_index]);
        if ($order < 0) {
            $left_index++;
        } elsif ($order > 0) {
            $right_index++;
        } else {
            return 0;
        }
    }
    return 1;
}

sub _from_sorted_operation {
    my ($self, $values) = @_;
    return __PACKAGE__->from_values($values, $self->{backend}->compare);
}

sub union {
    my ($self, $other) = @_;
    _require_set($other);
    return $self->_from_sorted_operation(
        _merge_unique($self->to_list, $other->to_list, $self->{backend}->compare),
    );
}

sub intersection {
    my ($self, $other) = @_;
    _require_set($other);
    return $self->_from_sorted_operation(
        _intersection_sorted($self->to_list, $other->to_list, $self->{backend}->compare),
    );
}

sub difference {
    my ($self, $other) = @_;
    _require_set($other);
    return $self->_from_sorted_operation(
        _difference_sorted($self->to_list, $other->to_list, $self->{backend}->compare),
    );
}

sub symmetric_difference {
    my ($self, $other) = @_;
    _require_set($other);
    return $self->_from_sorted_operation(
        _symmetric_difference_sorted($self->to_list, $other->to_list, $self->{backend}->compare),
    );
}

sub is_subset {
    my ($self, $other) = @_;
    _require_set($other);
    return _is_subset_sorted($self->to_list, $other->to_list, $self->{backend}->compare);
}

sub is_superset {
    my ($self, $other) = @_;
    _require_set($other);
    return $other->is_subset($self);
}

sub is_disjoint {
    my ($self, $other) = @_;
    _require_set($other);
    return _is_disjoint_sorted($self->to_list, $other->to_list, $self->{backend}->compare);
}

sub equals {
    my ($self, $other) = @_;
    return 0 if !_is_set($other) || $self->size != $other->size;
    my $left = $self->to_list;
    my $right = $other->to_list;
    my $compare = $self->{backend}->compare;
    for my $index (0 .. $#$left) {
        return 0 if $compare->($left->[$index], $right->[$index]) != 0;
    }
    return 1;
}

sub to_string {
    my ($self) = @_;
    return 'TreeSet([' . join(', ', @{$self->to_list}) . '])';
}

1;
