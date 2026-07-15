package CodingAdventures::SkipList;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number refaddr);
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

use constant MODULUS   => 2_147_483_647;
use constant MULTIPLIER => 48_271;

sub _default_compare {
    my ($left, $right) = @_;
    return 0 if "$left" eq "$right";
    if (looks_like_number($left) && looks_like_number($right)) {
        return $left <=> $right;
    }
    return "$left" cmp "$right";
}

sub _new_node {
    my ($key, $value, $height) = @_;
    return {
        key     => $key,
        value   => $value,
        height  => $height,
        forward => [(undef) x $height],
        span    => [(0) x $height],
    };
}

sub new {
    my ($class, @options) = @_;
    die "options must be key-value pairs\n" if @options % 2 != 0;
    my %options = @options;

    my $max_level = exists $options{max_level} ? $options{max_level} : 16;
    my $probability = exists $options{probability} ? $options{probability} : 0.5;
    my $compare = exists $options{compare} ? $options{compare} : \&_default_compare;
    my $seed = exists $options{seed} ? $options{seed} : 1;

    die "max_level must be a positive integer\n"
        if !defined($max_level) || ref($max_level) || !looks_like_number($max_level)
        || $max_level < 1 || int($max_level) != $max_level;
    die "probability must be between 0 and 1\n"
        if !defined($probability) || ref($probability) || !looks_like_number($probability)
        || $probability <= 0 || $probability >= 1;
    die "compare must be a code reference\n" if ref($compare) ne 'CODE';
    die "seed must be an integer\n"
        if !defined($seed) || ref($seed) || !looks_like_number($seed) || int($seed) != $seed;

    my $normalized_seed = abs(int($seed)) % (MODULUS - 1) + 1;
    my $head = _new_node(undef, undef, $max_level);
    return bless {
        head          => $head,
        max_level     => $max_level,
        probability   => $probability,
        compare       => $compare,
        rng_state     => $normalized_seed,
        current_level => 1,
        size          => 0,
    }, $class;
}

sub from_entries {
    my ($class, $entries, @options) = @_;
    die "entries must be an array reference\n" if ref($entries) ne 'ARRAY';
    my $self = $class->new(@options);
    for my $index (0 .. $#$entries) {
        my $entry = $entries->[$index];
        die "entry at index $index must be an array reference\n" if ref($entry) ne 'ARRAY';
        die "key at index $index must be defined\n" if !defined $entry->[0];
        $self->insert($entry->[0], $entry->[1]);
    }
    return $self;
}

sub _random {
    my ($self) = @_;
    $self->{rng_state} = ($self->{rng_state} * MULTIPLIER) % MODULUS;
    return $self->{rng_state} / MODULUS;
}

sub _random_level {
    my ($self) = @_;
    my $level = 1;
    while ($level < $self->{max_level} && $self->_random < $self->{probability}) {
        $level++;
    }
    return $level;
}

sub _find_predecessors {
    my ($self, $key) = @_;
    my (@update, @ranks);
    my $node = $self->{head};
    my $cumulative_rank = 0;

    for my $level (reverse 0 .. $self->{current_level} - 1) {
        my $next = $node->{forward}[$level];
        while (defined($next) && $self->{compare}->($next->{key}, $key) < 0) {
            $cumulative_rank += $node->{span}[$level];
            $node = $next;
            $next = $node->{forward}[$level];
        }
        $update[$level] = $node;
        $ranks[$level] = $cumulative_rank;
    }
    return (\@update, \@ranks);
}

sub insert {
    my ($self, $key, $value) = @_;
    die "key must be defined\n" if !defined $key;

    my ($update, $ranks) = $self->_find_predecessors($key);
    my $candidate = $update->[0]{forward}[0];
    if (defined($candidate) && $self->{compare}->($candidate->{key}, $key) == 0) {
        $candidate->{value} = $value;
        return 0;
    }

    my $height = $self->_random_level;
    if ($height > $self->{current_level}) {
        for my $level ($self->{current_level} .. $height - 1) {
            $update->[$level] = $self->{head};
            $ranks->[$level] = 0;
            $self->{head}{span}[$level] = $self->{size};
        }
        $self->{current_level} = $height;
    }

    my $node = _new_node($key, $value, $height);
    my $node_rank = $ranks->[0] + 1;
    for my $level (0 .. $height - 1) {
        my $predecessor = $update->[$level];
        my $old_span = $predecessor->{span}[$level];
        my $span_to_node = $node_rank - $ranks->[$level];
        $node->{forward}[$level] = $predecessor->{forward}[$level];
        $node->{span}[$level] = $old_span - $span_to_node + 1;
        $predecessor->{forward}[$level] = $node;
        $predecessor->{span}[$level] = $span_to_node;
    }

    for my $level ($height .. $self->{current_level} - 1) {
        $update->[$level]{span}[$level]++;
    }
    $self->{size}++;
    return 1;
}

sub delete {
    my ($self, $key) = @_;
    return 0 if !defined $key;
    my ($update) = $self->_find_predecessors($key);
    my $target = $update->[0]{forward}[0];
    return 0 if !defined($target) || $self->{compare}->($target->{key}, $key) != 0;

    for my $level (0 .. $self->{current_level} - 1) {
        my $predecessor = $update->[$level];
        my $next = $predecessor->{forward}[$level];
        if (defined($next) && refaddr($next) == refaddr($target)) {
            $predecessor->{span}[$level] += $target->{span}[$level] - 1;
            $predecessor->{forward}[$level] = $target->{forward}[$level];
        } else {
            $predecessor->{span}[$level]--;
        }
    }

    while ($self->{current_level} > 1
        && !defined($self->{head}{forward}[$self->{current_level} - 1])) {
        $self->{current_level}--;
    }
    $self->{size}--;
    return 1;
}

sub remove { return shift->delete(@_); }

sub search {
    my ($self, $key) = @_;
    return undef if !defined $key;
    my ($update) = $self->_find_predecessors($key);
    my $candidate = $update->[0]{forward}[0];
    return $candidate->{value}
        if defined($candidate) && $self->{compare}->($candidate->{key}, $key) == 0;
    return undef;
}

sub get { return shift->search(@_); }

sub contains {
    my ($self, $key) = @_;
    return 0 if !defined $key;
    my ($update) = $self->_find_predecessors($key);
    my $candidate = $update->[0]{forward}[0];
    return defined($candidate) && $self->{compare}->($candidate->{key}, $key) == 0;
}

sub has { return shift->contains(@_); }

sub rank {
    my ($self, $key) = @_;
    return undef if !defined $key;
    my ($update, $ranks) = $self->_find_predecessors($key);
    my $candidate = $update->[0]{forward}[0];
    return $ranks->[0]
        if defined($candidate) && $self->{compare}->($candidate->{key}, $key) == 0;
    return undef;
}

sub by_rank {
    my ($self, $rank) = @_;
    return undef if !defined($rank) || ref($rank) || !looks_like_number($rank)
        || $rank < 0 || int($rank) != $rank || $rank >= $self->{size};

    my $target_rank = $rank + 1;
    my $traversed = 0;
    my $node = $self->{head};
    for my $level (reverse 0 .. $self->{current_level} - 1) {
        while (defined($node->{forward}[$level])
            && $traversed + $node->{span}[$level] <= $target_rank) {
            $traversed += $node->{span}[$level];
            $node = $node->{forward}[$level];
        }
    }
    return $node->{key} if refaddr($node) != refaddr($self->{head}) && $traversed == $target_rank;
    return undef;
}

sub kth_smallest {
    my ($self, $k) = @_;
    return undef if !defined($k) || ref($k) || !looks_like_number($k)
        || $k < 1 || int($k) != $k;
    return $self->by_rank($k - 1);
}

sub range_query {
    my ($self, $minimum, $maximum, $inclusive) = @_;
    die "minimum must be defined\n" if !defined $minimum;
    die "maximum must be defined\n" if !defined $maximum;
    $inclusive = 1 if !defined $inclusive;
    die "inclusive must be 0 or 1\n" if ref($inclusive) || $inclusive !~ /^(?:0|1)$/;
    return [] if $self->{compare}->($minimum, $maximum) > 0;

    my ($update) = $self->_find_predecessors($minimum);
    my $node = $update->[0]{forward}[0];
    if (!$inclusive && defined($node) && $self->{compare}->($node->{key}, $minimum) == 0) {
        $node = $node->{forward}[0];
    }

    my @result;
    while (defined $node) {
        my $upper_order = $self->{compare}->($node->{key}, $maximum);
        last if $upper_order > 0 || (!$inclusive && $upper_order == 0);
        push @result, [$node->{key}, $node->{value}];
        $node = $node->{forward}[0];
    }
    return \@result;
}

sub range { return shift->range_query(@_); }

sub to_list {
    my ($self) = @_;
    my @result;
    my $node = $self->{head}{forward}[0];
    while (defined $node) {
        push @result, $node->{key};
        $node = $node->{forward}[0];
    }
    return \@result;
}

sub to_array { return shift->to_list(@_); }
sub to_sorted_array { return shift->to_list(@_); }

sub entries {
    my ($self) = @_;
    my @result;
    my $node = $self->{head}{forward}[0];
    while (defined $node) {
        push @result, [$node->{key}, $node->{value}];
        $node = $node->{forward}[0];
    }
    return \@result;
}

sub iterator {
    my ($self) = @_;
    my $node = $self->{head}{forward}[0];
    return sub {
        return if !defined $node;
        my ($key, $value) = ($node->{key}, $node->{value});
        $node = $node->{forward}[0];
        return ($key, $value);
    };
}

sub size { return $_[0]{size}; }
sub length { return $_[0]{size}; }
sub is_empty { return $_[0]{size} == 0; }

sub min {
    my ($self) = @_;
    my $node = $self->{head}{forward}[0];
    return defined($node) ? $node->{key} : undef;
}

sub max {
    my ($self) = @_;
    my $node = $self->{head}{forward}[0];
    return undef if !defined $node;
    $node = $node->{forward}[0] while defined $node->{forward}[0];
    return $node->{key};
}

sub max_level { return $_[0]{max_level}; }
sub probability { return $_[0]{probability}; }
sub current_level { return $_[0]{current_level}; }

sub is_valid_skip_list {
    my ($self) = @_;
    my %positions = (refaddr($self->{head}) => 0);
    my $count = 0;
    my $previous;
    my $node = $self->{head}{forward}[0];
    while (defined $node) {
        return 0 if defined($previous) && $self->{compare}->($previous->{key}, $node->{key}) >= 0;
        $count++;
        $positions{refaddr($node)} = $count;
        $previous = $node;
        $node = $node->{forward}[0];
    }
    return 0 if $count != $self->{size};

    for my $level (0 .. $self->{current_level} - 1) {
        $node = $self->{head};
        while (defined $node) {
            my $next = $node->{forward}[$level];
            my $position = $positions{refaddr($node)};
            my $expected_span;
            if (defined $next) {
                return 0 if !exists($positions{refaddr($next)}) || $next->{height} <= $level;
                $expected_span = $positions{refaddr($next)} - $position;
                return 0 if $expected_span <= 0;
            } else {
                $expected_span = $self->{size} - $position;
            }
            return 0 if $node->{span}[$level] != $expected_span;
            $node = $next;
        }
    }
    return 1;
}

sub to_string {
    my ($self) = @_;
    return 'SkipList([' . join(', ', @{$self->to_list}) . '])';
}

1;
