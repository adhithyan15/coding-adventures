package CodingAdventures::Heap;

use strict;
use warnings;

our $VERSION = '0.1.0';

sub default_compare {
    my ($left, $right) = @_;
    return 0 if $left == $right;
    return $left < $right ? -1 : 1;
}

1;

package CodingAdventures::Heap::Heap;

use strict;
use warnings;

sub new {
    my ($class, $compare) = @_;
    $compare ||= \&CodingAdventures::Heap::default_compare;
    return bless {
        compare => $compare,
        items   => [],
    }, $class;
}

sub from_iterable {
    my ($class, $items, $compare) = @_;
    my $heap = $class->new($compare);
    $heap->push($_) for @{ $items || [] };
    return $heap;
}

sub size {
    my ($self) = @_;
    return scalar @{ $self->{items} };
}

sub is_empty {
    my ($self) = @_;
    return $self->size() == 0;
}

sub peek {
    my ($self) = @_;
    return $self->{items}[0];
}

sub to_array {
    my ($self) = @_;
    return [@{ $self->{items} }];
}

sub push {
    my ($self, $value) = @_;
    push @{ $self->{items} }, $value;
    $self->_sift_up($#{ $self->{items} });
    return $self;
}

sub pop {
    my ($self) = @_;
    return undef if $self->is_empty();

    my $items = $self->{items};
    my $top = $items->[0];

    if (@$items == 1) {
        pop @$items;
        return $top;
    }

    $items->[0] = pop @$items;
    $self->_sift_down(0);
    return $top;
}

sub _higher_priority {
    die "_higher_priority must be implemented by subclasses\n";
}

sub _sift_up {
    my ($self, $index) = @_;
    my $items = $self->{items};

    while ($index > 0) {
        my $parent = int(($index - 1) / 2);
        if ($self->_higher_priority($items->[$index], $items->[$parent])) {
            @{$items}[$index, $parent] = @{$items}[$parent, $index];
            $index = $parent;
        } else {
            last;
        }
    }
}

sub _sift_down {
    my ($self, $index) = @_;
    my $items = $self->{items};
    my $size = scalar @$items;

    while (1) {
        my $left = 2 * $index + 1;
        my $right = $left + 1;
        my $best = $index;

        if ($left < $size && $self->_higher_priority($items->[$left], $items->[$best])) {
            $best = $left;
        }
        if ($right < $size && $self->_higher_priority($items->[$right], $items->[$best])) {
            $best = $right;
        }

        last if $best == $index;

        @{$items}[$index, $best] = @{$items}[$best, $index];
        $index = $best;
    }
}

1;

package CodingAdventures::Heap::MinHeap;

use strict;
use warnings;

our @ISA = ('CodingAdventures::Heap::Heap');

sub _higher_priority {
    my ($self, $left, $right) = @_;
    return $self->{compare}->($left, $right) < 0;
}

1;

package CodingAdventures::Heap::MaxHeap;

use strict;
use warnings;

our @ISA = ('CodingAdventures::Heap::Heap');

sub _higher_priority {
    my ($self, $left, $right) = @_;
    return $self->{compare}->($left, $right) > 0;
}

1;
