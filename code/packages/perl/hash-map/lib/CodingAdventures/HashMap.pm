package CodingAdventures::HashMap;

use strict;
use warnings;
use Exporter qw(import);
use Scalar::Util qw(blessed refaddr);
use CodingAdventures::HashFunctions qw(fnv1a_32 murmur3_32 djb2);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(new_map from_entries merge);

my $DEFAULT_CAPACITY = 16;
my $EMPTY = undef;
my $TOMBSTONE = bless {}, 'CodingAdventures::HashMap::Tombstone';

sub _positive_integer {
    my ($value, $name) = @_;
    die "$name must be a positive integer\n"
        if !defined($value) || ref($value) || "$value" !~ /\A\d+\z/ || $value <= 0;
    return 0 + $value;
}

sub _normalize_strategy {
    my ($strategy) = @_;
    $strategy = 'chaining' unless defined $strategy;
    return 'chaining' if $strategy eq 'chaining';
    return 'open_addressing'
        if $strategy eq 'open_addressing'
        || $strategy eq 'open-addressing'
        || $strategy eq 'open';
    die "strategy must be 'chaining' or 'open_addressing'\n";
}

sub _normalize_hash_fn {
    my ($hash_fn) = @_;
    $hash_fn = 'fnv1a' unless defined $hash_fn;
    return 'fnv1a' if $hash_fn eq 'fnv1a' || $hash_fn eq 'fnv1a_32';
    return 'murmur3' if $hash_fn eq 'murmur3' || $hash_fn eq 'murmur3_32';
    return 'djb2' if $hash_fn eq 'djb2';
    die "hash_fn must be 'fnv1a', 'murmur3', or 'djb2'\n";
}

sub _serialize_key {
    my ($key) = @_;
    return 'undef:' unless defined $key;
    return 'reference:' . ref($key) . ':' . refaddr($key) if ref($key);
    return 'scalar:' . $key;
}

sub _keys_equal {
    my ($left, $right) = @_;
    return !defined($left) && !defined($right)
        if !defined($left) || !defined($right);
    if (ref($left) || ref($right)) {
        return 0 unless ref($left) && ref($right);
        return refaddr($left) == refaddr($right);
    }
    return $left eq $right;
}

sub _is_tombstone {
    my ($slot) = @_;
    return ref($slot) && refaddr($slot) == refaddr($TOMBSTONE);
}

sub _apply_hash {
    my ($data, $hash_fn) = @_;
    return murmur3_32($data) if $hash_fn eq 'murmur3';
    return djb2($data) if $hash_fn eq 'djb2';
    return fnv1a_32($data);
}

sub _hash_modulo {
    my ($value, $modulus) = @_;
    if (blessed($value) && $value->isa('Math::BigInt')) {
        return $value->copy->bmod($modulus)->numify;
    }
    return $value % $modulus;
}

sub _initialize_storage {
    my ($self) = @_;
    if ($self->{strategy} eq 'chaining') {
        $self->{buckets} = [map { [] } 1 .. $self->{capacity}];
        delete $self->{slots};
    } else {
        $self->{slots} = [map { $EMPTY } 1 .. $self->{capacity}];
        delete $self->{buckets};
    }
}

sub new {
    my ($class, %args) = @_;
    my $capacity = exists($args{capacity}) ? $args{capacity} : $DEFAULT_CAPACITY;
    $capacity = _positive_integer($capacity, 'capacity');
    my $self = bless {
        capacity => $capacity,
        size     => 0,
        strategy => _normalize_strategy($args{strategy}),
        hash_fn  => _normalize_hash_fn($args{hash_fn}),
    }, $class;
    _initialize_storage($self);
    return $self;
}

sub new_map {
    return __PACKAGE__->new(@_);
}

sub _bucket_index {
    my ($self, $key) = @_;
    my $hash = _apply_hash(_serialize_key($key), $self->{hash_fn});
    return _hash_modulo($hash, $self->{capacity});
}

sub _set_chaining {
    my ($self, $key, $value) = @_;
    my $bucket = $self->{buckets}->[$self->_bucket_index($key)];
    for my $entry (@{$bucket}) {
        if (_keys_equal($entry->[0], $key)) {
            $entry->[1] = $value;
            return;
        }
    }
    push @{$bucket}, [$key, $value];
    $self->{size}++;
}

sub _set_open_addressing {
    my ($self, $key, $value) = @_;
    my $start = $self->_bucket_index($key);
    my $first_tombstone;
    for my $probe (0 .. $self->{capacity} - 1) {
        my $index = ($start + $probe) % $self->{capacity};
        my $slot = $self->{slots}->[$index];
        if (!defined $slot) {
            my $insert_at = defined($first_tombstone) ? $first_tombstone : $index;
            $self->{slots}->[$insert_at] = [$key, $value];
            $self->{size}++;
            return;
        }
        if (_is_tombstone($slot)) {
            $first_tombstone = $index unless defined $first_tombstone;
        } elsif (_keys_equal($slot->[0], $key)) {
            $slot->[1] = $value;
            return;
        }
    }
    if (defined $first_tombstone) {
        $self->{slots}->[$first_tombstone] = [$key, $value];
        $self->{size}++;
        return;
    }
    die "hash map is full; resize should have happened earlier\n";
}

sub _set_without_resize {
    my ($self, $key, $value) = @_;
    if ($self->{strategy} eq 'chaining') {
        $self->_set_chaining($key, $value);
    } else {
        $self->_set_open_addressing($key, $value);
    }
}

sub _needs_resize {
    my ($self) = @_;
    my $threshold = $self->{strategy} eq 'chaining' ? 1.0 : 0.75;
    return $self->load_factor > $threshold;
}

sub _resize {
    my ($self, $new_capacity) = @_;
    my $entries = $self->entries;
    $self->{capacity} = $new_capacity;
    $self->{size} = 0;
    _initialize_storage($self);
    $self->_set_without_resize(@{$_}) for @{$entries};
}

sub set {
    my ($self, $key, $value) = @_;
    die "key must be defined\n" unless defined $key;
    $self->_set_without_resize($key, $value);
    $self->_resize($self->{capacity} * 2) if $self->_needs_resize;
    return $self;
}

sub _find_entry {
    my ($self, $key) = @_;
    return unless defined $key;
    my $start = $self->_bucket_index($key);
    if ($self->{strategy} eq 'chaining') {
        for my $entry (@{$self->{buckets}->[$start]}) {
            return $entry if _keys_equal($entry->[0], $key);
        }
        return;
    }
    for my $probe (0 .. $self->{capacity} - 1) {
        my $slot = $self->{slots}->[($start + $probe) % $self->{capacity}];
        return unless defined $slot;
        next if _is_tombstone($slot);
        return $slot if _keys_equal($slot->[0], $key);
    }
    return;
}

sub get {
    my ($self, $key) = @_;
    my $entry = $self->_find_entry($key);
    return defined($entry) ? $entry->[1] : undef;
}

sub has {
    my ($self, $key) = @_;
    return defined $self->_find_entry($key);
}

sub delete {
    my ($self, $key) = @_;
    return 0 unless defined $key;
    my $start = $self->_bucket_index($key);
    if ($self->{strategy} eq 'chaining') {
        my $bucket = $self->{buckets}->[$start];
        for my $index (0 .. $#{$bucket}) {
            if (_keys_equal($bucket->[$index]->[0], $key)) {
                splice @{$bucket}, $index, 1;
                $self->{size}--;
                return 1;
            }
        }
        return 0;
    }
    for my $probe (0 .. $self->{capacity} - 1) {
        my $index = ($start + $probe) % $self->{capacity};
        my $slot = $self->{slots}->[$index];
        return 0 unless defined $slot;
        next if _is_tombstone($slot);
        if (_keys_equal($slot->[0], $key)) {
            $self->{slots}->[$index] = $TOMBSTONE;
            $self->{size}--;
            return 1;
        }
    }
    return 0;
}

sub entries {
    my ($self) = @_;
    my @entries;
    if ($self->{strategy} eq 'chaining') {
        for my $bucket (@{$self->{buckets}}) {
            push @entries, map { [@{$_}] } @{$bucket};
        }
    } else {
        for my $slot (@{$self->{slots}}) {
            push @entries, [@{$slot}]
                if defined($slot) && !_is_tombstone($slot);
        }
    }
    return \@entries;
}

sub keys {
    my ($self) = @_;
    return [map { $_->[0] } @{$self->entries}];
}

sub values {
    my ($self) = @_;
    return [map { $_->[1] } @{$self->entries}];
}

sub size { return $_[0]->{size}; }
sub capacity { return $_[0]->{capacity}; }
sub strategy { return $_[0]->{strategy}; }
sub hash_fn { return $_[0]->{hash_fn}; }

sub load_factor {
    my ($self) = @_;
    return $self->{size} / $self->{capacity};
}

sub clone {
    my ($self) = @_;
    my $copy = __PACKAGE__->new(
        capacity => $self->{capacity},
        strategy => $self->{strategy},
        hash_fn  => $self->{hash_fn},
    );
    $copy->_set_without_resize(@{$_}) for @{$self->entries};
    return $copy;
}

sub clear {
    my ($self) = @_;
    $self->{size} = 0;
    _initialize_storage($self);
    return $self;
}

sub with_set {
    my ($self, $key, $value) = @_;
    return $self->clone->set($key, $value);
}

sub without {
    my ($self, $key) = @_;
    my $copy = $self->clone;
    $copy->delete($key);
    return $copy;
}

sub from_entries {
    my ($invocant, @rest) = @_;
    my ($entries, %args);
    if (!ref($invocant) && $invocant eq __PACKAGE__) {
        $entries = shift @rest;
        %args = @rest;
    } else {
        $entries = $invocant;
        %args = @rest;
    }
    die "entries must be an array reference of key-value pairs\n"
        unless ref($entries) eq 'ARRAY';
    my $map = __PACKAGE__->new(%args);
    for my $entry (@{$entries}) {
        die "each entry must be a two-item array reference with a defined key\n"
            unless ref($entry) eq 'ARRAY' && @{$entry} == 2 && defined($entry->[0]);
        $map->set(@{$entry});
    }
    return $map;
}

sub merge {
    my ($left, $right) = @_;
    my $capacity = $left->capacity > $right->capacity
        ? $left->capacity
        : $right->capacity;
    my $result = __PACKAGE__->new(
        capacity => $capacity,
        strategy => $left->strategy,
        hash_fn  => $left->hash_fn,
    );
    $result->set(@{$_}) for @{$left->entries};
    $result->set(@{$_}) for @{$right->entries};
    return $result;
}

1;

__END__

=head1 NAME

CodingAdventures::HashMap - chaining and open-addressing hash maps

=head1 SYNOPSIS

  use CodingAdventures::HashMap;
  my $map = CodingAdventures::HashMap->new(strategy => 'open_addressing');
  $map->set(language => 'Perl');
  say $map->get('language');

=head1 DESCRIPTION

Implements DT18 from first principles using the sibling DT17 hash-functions
package. Both separate chaining and linear probing include automatic resizing;
open addressing uses tombstones so deletion preserves probe chains.

=cut
