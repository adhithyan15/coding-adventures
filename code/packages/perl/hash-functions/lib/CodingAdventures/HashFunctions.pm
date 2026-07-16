package CodingAdventures::HashFunctions;

use strict;
use warnings;
use Encode qw(encode FB_CROAK);
use Exporter qw(import);
use Math::BigInt;
use Scalar::Util qw(blessed);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    fnv1a_32
    fnv1a_64
    djb2
    polynomial_rolling
    murmur3_32
    avalanche_score
    distribution_test
    uint64_hex
);

my $UINT32_MASK = 0xffffffff;
my $FNV32_OFFSET_BASIS = 0x811c9dc5;
my $FNV32_PRIME = 0x01000193;
my $FNV64_OFFSET_BASIS = Math::BigInt->from_hex('0xcbf29ce484222325');
my $FNV64_PRIME = Math::BigInt->from_hex('0x00000100000001b3');
my $UINT64_MODULUS = Math::BigInt->new(2)->bpow(64);
my $POLYNOMIAL_DEFAULT_BASE = Math::BigInt->new(31);
my $POLYNOMIAL_DEFAULT_MODULUS = Math::BigInt->new(2)->bpow(61)->bsub(1);
my $MURMUR3_C1 = 0xcc9e2d51;
my $MURMUR3_C2 = 0x1b873593;

sub _to_bytes {
    my ($data) = @_;
    die "data must be a scalar\n" if !defined($data) || ref($data);
    return utf8::is_utf8($data) ? encode('UTF-8', $data, FB_CROAK) : "$data";
}

sub _native_integer {
    my ($value, $name) = @_;
    die "$name must be an integer\n"
        if !defined($value) || ref($value) || "$value" !~ /\A[+-]?\d+\z/;
    return 0 + $value;
}

sub _big_integer {
    my ($value, $name) = @_;
    if (blessed($value) && $value->isa('Math::BigInt')) {
        return $value->copy;
    }
    die "$name must be an integer\n"
        if !defined($value) || ref($value) || "$value" !~ /\A[+-]?\d+\z/;
    return Math::BigInt->new("$value");
}

sub fnv1a_32 {
    my ($data) = @_;
    my $hash = $FNV32_OFFSET_BASIS;
    for my $byte (unpack('C*', _to_bytes($data))) {
        $hash = (($hash ^ $byte) * $FNV32_PRIME) & $UINT32_MASK;
    }
    return $hash;
}

sub fnv1a_64 {
    my ($data) = @_;
    my $hash = $FNV64_OFFSET_BASIS->copy;
    for my $byte (unpack('C*', _to_bytes($data))) {
        $hash->bxor($byte);
        $hash->bmul($FNV64_PRIME)->bmod($UINT64_MODULUS);
    }
    return $hash;
}

sub djb2 {
    my ($data) = @_;
    my $hash = Math::BigInt->new(5381);
    for my $byte (unpack('C*', _to_bytes($data))) {
        $hash->bmul(33)->badd($byte)->bmod($UINT64_MODULUS);
    }
    return $hash;
}

sub uint64_hex {
    my ($value) = @_;
    my $word = _big_integer($value, 'value')->bmod($UINT64_MODULUS);
    my $hex = $word->as_hex;
    $hex =~ s/\A0x//;
    return ('0' x (16 - length($hex))) . lc($hex);
}

sub polynomial_rolling {
    my ($data, $base, $modulus) = @_;
    my $raw = _to_bytes($data);
    $base = defined($base) ? _big_integer($base, 'base') : $POLYNOMIAL_DEFAULT_BASE->copy;
    $modulus = defined($modulus)
        ? _big_integer($modulus, 'modulus')
        : $POLYNOMIAL_DEFAULT_MODULUS->copy;
    die "modulus must be positive\n" if $modulus->is_zero || $modulus->is_neg;

    $base->bmod($modulus);
    my $hash = Math::BigInt->bzero;
    for my $byte (unpack('C*', $raw)) {
        $hash->bmul($base)->badd($byte)->bmod($modulus);
    }
    return $hash;
}

sub _rotate_left_32 {
    my ($value, $count) = @_;
    return (($value << $count) | ($value >> (32 - $count))) & $UINT32_MASK;
}

sub _fmix32 {
    my ($hash) = @_;
    $hash = ($hash ^ ($hash >> 16)) & $UINT32_MASK;
    $hash = ($hash * 0x85ebca6b) & $UINT32_MASK;
    $hash = ($hash ^ ($hash >> 13)) & $UINT32_MASK;
    $hash = ($hash * 0xc2b2ae35) & $UINT32_MASK;
    return ($hash ^ ($hash >> 16)) & $UINT32_MASK;
}

sub murmur3_32 {
    my ($data, $seed) = @_;
    my @bytes = unpack('C*', _to_bytes($data));
    $seed = defined($seed) ? _native_integer($seed, 'seed') : 0;
    my $hash = $seed & $UINT32_MASK;
    my $length = scalar(@bytes);
    my $block_count = int($length / 4);

    for (my $block_index = 0; $block_index < $block_count; $block_index++) {
        my $offset = $block_index * 4;
        my $k = $bytes[$offset]
            | ($bytes[$offset + 1] << 8)
            | ($bytes[$offset + 2] << 16)
            | ($bytes[$offset + 3] << 24);

        $k = ($k * $MURMUR3_C1) & $UINT32_MASK;
        $k = _rotate_left_32($k, 15);
        $k = ($k * $MURMUR3_C2) & $UINT32_MASK;

        $hash ^= $k;
        $hash = _rotate_left_32($hash, 13);
        $hash = ($hash * 5 + 0xe6546b64) & $UINT32_MASK;
    }

    my $tail_offset = $block_count * 4;
    my $remaining = $length & 3;
    my $k = 0;
    $k ^= $bytes[$tail_offset + 2] << 16 if $remaining >= 3;
    $k ^= $bytes[$tail_offset + 1] << 8 if $remaining >= 2;
    if ($remaining >= 1) {
        $k ^= $bytes[$tail_offset];
        $k = ($k * $MURMUR3_C1) & $UINT32_MASK;
        $k = _rotate_left_32($k, 15);
        $k = ($k * $MURMUR3_C2) & $UINT32_MASK;
        $hash ^= $k;
    }

    $hash ^= $length;
    return _fmix32($hash);
}

sub _deterministic_bytes {
    my ($sample_index) = @_;
    my $state = (0x9e3779b9 ^ $sample_index) & $UINT32_MASK;
    my @bytes;
    for (1 .. 8) {
        $state = ($state * 1664525 + 1013904223) & $UINT32_MASK;
        push @bytes, $state & 0xff;
    }
    return @bytes;
}

sub _normalized_hash {
    my ($value, $width) = @_;
    my $hash = _big_integer($value, 'hash function result');
    my $modulus = Math::BigInt->new(2)->bpow($width);
    return $hash->bmod($modulus);
}

sub _popcount {
    my ($value) = @_;
    my $binary = $value->as_bin;
    $binary =~ s/\A0b//;
    return $binary =~ tr/1/1/;
}

sub avalanche_score {
    my ($hash_fn, $output_bits, $sample_size) = @_;
    die "hash_fn must be a code reference\n" unless ref($hash_fn) eq 'CODE';
    $output_bits = _native_integer($output_bits, 'output_bits');
    die "output_bits must be in 1..64\n"
        if $output_bits < 1 || $output_bits > 64;
    $sample_size = defined($sample_size)
        ? _native_integer($sample_size, 'sample_size')
        : 100;
    die "sample_size must be positive\n" if $sample_size <= 0;

    my $total_bit_flips = 0;
    my $total_trials = 0;
    for my $sample_index (0 .. $sample_size - 1) {
        my @bytes = _deterministic_bytes($sample_index);
        my $original = _normalized_hash($hash_fn->(pack('C*', @bytes)), $output_bits);
        for my $bit_position (0 .. 63) {
            my $byte_index = int($bit_position / 8);
            my $bit_mask = 1 << ($bit_position & 7);
            $bytes[$byte_index] ^= $bit_mask;
            my $changed = _normalized_hash(
                $hash_fn->(pack('C*', @bytes)),
                $output_bits,
            );
            $bytes[$byte_index] ^= $bit_mask;
            $total_bit_flips += _popcount($original->copy->bxor($changed));
            $total_trials += $output_bits;
        }
    }
    return $total_bit_flips / $total_trials;
}

sub distribution_test {
    my ($hash_fn, $inputs, $num_buckets) = @_;
    die "hash_fn must be a code reference\n" unless ref($hash_fn) eq 'CODE';
    die "inputs must be a non-empty array reference\n"
        unless ref($inputs) eq 'ARRAY' && @{$inputs};
    $num_buckets = _native_integer($num_buckets, 'num_buckets');
    die "num_buckets must be positive\n" if $num_buckets <= 0;

    my @counts = (0) x $num_buckets;
    for my $input (@{$inputs}) {
        my $hash = _normalized_hash($hash_fn->(_to_bytes($input)), 64);
        my $bucket = $hash->bmod($num_buckets)->numify;
        $counts[$bucket]++;
    }

    my $expected = @{$inputs} / $num_buckets;
    my $chi_squared = 0.0;
    for my $observed (@counts) {
        my $difference = $observed - $expected;
        $chi_squared += $difference * $difference / $expected;
    }
    return $chi_squared;
}

1;

__END__

=head1 NAME

CodingAdventures::HashFunctions - pure non-cryptographic hash functions

=head1 SYNOPSIS

  use CodingAdventures::HashFunctions qw(fnv1a_32 murmur3_32);
  my $hash = fnv1a_32("hello");

=head1 DESCRIPTION

Implements FNV-1a, DJB2, polynomial rolling hash, MurmurHash3, and small
deterministic quality-analysis helpers. These functions are not suitable for
passwords, signatures, or other cryptographic uses.

=cut
