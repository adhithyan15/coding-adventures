package CodingAdventures::BloomFilter;

use strict;
use warnings;
use Encode qw(encode FB_CROAK);
use Exporter qw(import);
use POSIX qw(ceil floor);
use Scalar::Util qw(looks_like_number refaddr);
use CodingAdventures::HashFunctions qw(fnv1a_32 djb2);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(optimal_m optimal_k capacity_for_memory);

use overload q{""} => 'to_string', fallback => 1;

my $DEFAULT_EXPECTED_ITEMS = 1_000;
my $DEFAULT_FALSE_POSITIVE_RATE = 0.01;
my $UINT32_MASK = 0xffffffff;

sub _positive_integer {
    my ($value, $name) = @_;
    die "$name must be a positive integer\n"
        if !defined($value) || ref($value) || "$value" !~ /\A\d+\z/ || $value <= 0;
    return 0 + $value;
}

sub _nonnegative_integer {
    my ($value, $name) = @_;
    die "$name must be a nonnegative integer\n"
        if !defined($value) || ref($value) || "$value" !~ /\A\d+\z/;
    return 0 + $value;
}

sub _false_positive_rate {
    my ($value) = @_;
    die "false_positive_rate must be in the open interval (0, 1)\n"
        if !defined($value)
        || ref($value)
        || !looks_like_number($value)
        || $value != $value
        || $value <= 0
        || $value >= 1;
    return 0.0 + $value;
}

sub _scalar_bytes {
    my ($value) = @_;
    return 'undef' if !defined($value);
    return utf8::is_utf8($value) ? encode('UTF-8', $value, FB_CROAK) : "$value";
}

sub _stable_encode {
    my ($value, $seen) = @_;
    return _scalar_bytes($value) unless ref($value);

    my $kind = ref($value);
    die "element must be undef, a scalar, an array reference, or a hash reference\n"
        unless $kind eq 'ARRAY' || $kind eq 'HASH' || $kind eq 'SCALAR';
    my $address = refaddr($value);
    die "element references must not contain cycles\n" if $seen->{$address};
    $seen->{$address} = 1;

    my $encoded;
    if ($kind eq 'ARRAY') {
        my @items = map {
            my $item = _stable_encode($_, $seen);
            length($item) . ':' . $item;
        } @{$value};
        $encoded = '[' . join(',', @items) . ']';
    } elsif ($kind eq 'HASH') {
        my @items;
        for my $key (keys %{$value}) {
            my $encoded_key = _scalar_bytes($key);
            my $encoded_value = _stable_encode($value->{$key}, $seen);
            push @items,
                length($encoded_key) . ':' . $encoded_key
                . '='
                . length($encoded_value) . ':' . $encoded_value;
        }
        $encoded = '{' . join(',', sort @items) . '}';
    } else {
        my $item = _stable_encode(${$value}, $seen);
        $encoded = '\\' . length($item) . ':' . $item;
    }

    delete $seen->{$address};
    return $encoded;
}

sub _element_bytes {
    my ($value) = @_;
    return _stable_encode($value, {});
}

sub _fmix32 {
    my ($value) = @_;
    $value = ($value ^ ($value >> 16)) & $UINT32_MASK;
    $value = ($value * 0x85ebca6b) & $UINT32_MASK;
    $value = ($value ^ ($value >> 13)) & $UINT32_MASK;
    $value = ($value * 0xc2b2ae35) & $UINT32_MASK;
    return ($value ^ ($value >> 16)) & $UINT32_MASK;
}

sub optimal_m {
    my ($expected_items, $false_positive_rate) = @_;
    $expected_items = _positive_integer($expected_items, 'expected_items');
    $false_positive_rate = _false_positive_rate($false_positive_rate);
    return ceil(
        -$expected_items * log($false_positive_rate) / (log(2) ** 2)
    );
}

sub optimal_k {
    my ($bit_count, $expected_items) = @_;
    $bit_count = _positive_integer($bit_count, 'bit_count');
    $expected_items = _positive_integer($expected_items, 'expected_items');
    my $rounded = floor(($bit_count / $expected_items) * log(2) + 0.5);
    return $rounded > 1 ? $rounded : 1;
}

sub capacity_for_memory {
    my ($memory_bytes, $false_positive_rate) = @_;
    $memory_bytes = _nonnegative_integer($memory_bytes, 'memory_bytes');
    $false_positive_rate = _false_positive_rate($false_positive_rate);
    return floor(
        -($memory_bytes * 8) * (log(2) ** 2) / log($false_positive_rate)
    );
}

sub _from_parts {
    my ($class, $bit_count, $hash_count, $expected_items) = @_;
    my $byte_count = int(($bit_count + 7) / 8);
    return bless {
        bit_count      => $bit_count,
        hash_count     => $hash_count,
        expected_items => $expected_items,
        bits           => "\0" x $byte_count,
        bits_set       => 0,
        items_added    => 0,
    }, $class;
}

sub new {
    my ($class, %args) = @_;
    my $expected_items = exists($args{expected_items})
        ? $args{expected_items}
        : $DEFAULT_EXPECTED_ITEMS;
    my $false_positive_rate = exists($args{false_positive_rate})
        ? $args{false_positive_rate}
        : $DEFAULT_FALSE_POSITIVE_RATE;
    $expected_items = _positive_integer($expected_items, 'expected_items');
    $false_positive_rate = _false_positive_rate($false_positive_rate);
    my $bit_count = optimal_m($expected_items, $false_positive_rate);
    my $hash_count = optimal_k($bit_count, $expected_items);
    return _from_parts($class, $bit_count, $hash_count, $expected_items);
}

sub from_params {
    my ($class, $bit_count, $hash_count) = @_;
    $bit_count = _positive_integer($bit_count, 'bit_count');
    $hash_count = _positive_integer($hash_count, 'hash_count');
    return _from_parts($class, $bit_count, $hash_count, 0);
}

sub _hash_indices {
    my ($self, $element) = @_;
    my $raw = _element_bytes($element);
    my $first = _fmix32(fnv1a_32($raw));
    my $second_word = djb2($raw);
    my $folded = $second_word->copy;
    $folded->bxor($second_word->copy->brsft(32))->bmod(4_294_967_296);
    my $second = _fmix32($folded->numify) | 1;
    my @indices;
    for my $index (0 .. $self->{hash_count} - 1) {
        push @indices,
            ($first + $index * $second) % $self->{bit_count};
    }
    return @indices;
}

sub add {
    my ($self, $element) = @_;
    for my $bit_index ($self->_hash_indices($element)) {
        if (!vec($self->{bits}, $bit_index, 1)) {
            vec($self->{bits}, $bit_index, 1) = 1;
            $self->{bits_set}++;
        }
    }
    $self->{items_added}++;
    return;
}

sub contains {
    my ($self, $element) = @_;
    for my $bit_index ($self->_hash_indices($element)) {
        return 0 unless vec($self->{bits}, $bit_index, 1);
    }
    return 1;
}

sub bit_count { return $_[0]->{bit_count}; }
sub hash_count { return $_[0]->{hash_count}; }
sub bits_set { return $_[0]->{bits_set}; }
sub size_bytes { return length($_[0]->{bits}); }

sub fill_ratio {
    my ($self) = @_;
    return $self->{bits_set} / $self->{bit_count};
}

sub estimated_false_positive_rate {
    my ($self) = @_;
    return 0.0 if $self->{bits_set} == 0;
    return $self->fill_ratio ** $self->{hash_count};
}

sub is_over_capacity {
    my ($self) = @_;
    return $self->{expected_items} > 0
        && $self->{items_added} > $self->{expected_items};
}

sub to_string {
    my ($self) = @_;
    return sprintf(
        'BloomFilter(m=%d, k=%d, bits_set=%d/%d (%.2f%%), ~fp=%.4f%%)',
        $self->{bit_count},
        $self->{hash_count},
        $self->{bits_set},
        $self->{bit_count},
        $self->fill_ratio * 100,
        $self->estimated_false_positive_rate * 100,
    );
}

1;

__END__

=head1 NAME

CodingAdventures::BloomFilter - probabilistic set membership

=head1 SYNOPSIS

  my $filter = CodingAdventures::BloomFilter->new(
      expected_items => 1000,
      false_positive_rate => 0.01,
  );
  $filter->add('hello');
  say $filter->contains('hello');

=head1 DESCRIPTION

Implements a compact Bloom filter using FNV-1a and DJB2 double hashing. A
negative lookup is definitive; a positive lookup may be a false positive at
the configured rate.

=cut
