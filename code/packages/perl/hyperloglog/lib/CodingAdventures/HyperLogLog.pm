package CodingAdventures::HyperLogLog;

use strict;
use warnings;
use Encode qw(encode);
use Scalar::Util qw(blessed);
use overload '""' => 'as_string', fallback => 1;

our $VERSION = '0.1.0';

my $UINT32_MASK = 0xffffffff;
my $FNV_OFFSET = 2166136261;
my $FNV_PRIME = 16777619;
my $SECOND_SEED = ($FNV_OFFSET ^ 0x9e3779b9) & $UINT32_MASK;

sub new {
    my ($class, @arguments) = @_;
    die "constructor expects key/value options\n" if @arguments % 2;
    my %options = @arguments;
    my $precision = exists $options{precision} ? $options{precision} : 10;
    _validate_precision($precision);

    my $register_count = 1 << $precision;
    return bless {
        precision      => 0 + $precision,
        register_count => $register_count,
        registers      => [(0) x $register_count],
    }, $class;
}

sub add {
    my ($self, $value) = @_;
    my ($high, $low) = _hash64($value);
    my $bucket = $high & ($self->{register_count} - 1);
    my $upper = $high >> $self->{precision};
    my $upper_width = 32 - $self->{precision};
    my $zero_count;

    if ($upper != 0) {
        $zero_count = _leading_zeros32($upper) - $self->{precision};
    } else {
        $zero_count = $upper_width + _leading_zeros32($low);
    }

    my $rank = $zero_count + 1;
    $self->{registers}[$bucket] = $rank
        if $rank > $self->{registers}[$bucket];
    return $self;
}

sub count {
    my ($self) = @_;
    my $indicator = 0.0;
    my $empty_registers = 0;

    for my $register (@{$self->{registers}}) {
        $indicator += 2.0 ** (-$register);
        $empty_registers++ if $register == 0;
    }

    my $m = $self->{register_count};
    my $estimate = _alpha($m) * $m * $m / $indicator;
    if ($estimate <= 2.5 * $m && $empty_registers > 0) {
        $estimate = $m * log($m / $empty_registers);
    }
    return int($estimate + 0.5);
}

sub merge {
    my ($self, $other) = @_;
    my $merged = ref($self)->new(precision => $self->{precision});
    @{$merged->{registers}} = @{$self->{registers}};
    return $merged->merge_in_place($other);
}

sub merge_in_place {
    my ($self, $other) = @_;
    die "other must be a CodingAdventures::HyperLogLog\n"
        unless blessed($other) && $other->isa(__PACKAGE__);
    die "cannot merge HyperLogLog sketches with different precisions\n"
        unless $other->{precision} == $self->{precision};

    for my $index (0 .. $self->{register_count} - 1) {
        my $candidate = $other->{registers}[$index];
        $self->{registers}[$index] = $candidate
            if $candidate > $self->{registers}[$index];
    }
    return $self;
}

sub clear {
    my ($self) = @_;
    @{$self->{registers}} = (0) x $self->{register_count};
    return $self;
}

sub is_empty {
    my ($self) = @_;
    for my $register (@{$self->{registers}}) {
        return 0 if $register != 0;
    }
    return 1;
}

sub precision {
    my ($self) = @_;
    return $self->{precision};
}

sub num_registers {
    my ($self) = @_;
    return $self->{register_count};
}

sub error_rate {
    my ($self) = @_;
    return 1.04 / sqrt($self->{register_count});
}

sub memory_bytes {
    my ($self) = @_;
    return int($self->{register_count} * 6 / 8);
}

sub registers {
    my ($self) = @_;
    return [@{$self->{registers}}];
}

sub as_string {
    my ($self) = @_;
    return sprintf(
        'HyperLogLog(precision=%d, registers=%d, estimate=%d)',
        $self->{precision},
        $self->{register_count},
        $self->count,
    );
}

sub _validate_precision {
    my ($precision) = @_;
    die "precision must be an integer between 4 and 16\n"
        unless defined($precision)
            && !ref($precision)
            && $precision =~ /\A\d+\z/
            && $precision >= 4
            && $precision <= 16;
}

sub _alpha {
    my ($register_count) = @_;
    return 0.673 if $register_count == 16;
    return 0.697 if $register_count == 32;
    return 0.709 if $register_count == 64;
    return 0.7213 / (1 + 1.079 / $register_count);
}

sub _hash64 {
    my ($value) = @_;
    my $payload = encode('UTF-8', defined($value) ? "$value" : 'undef');
    return (
        _fnv1a32($payload, $FNV_OFFSET, 0),
        _fnv1a32($payload, $SECOND_SEED, 1),
    );
}

sub _fnv1a32 {
    my ($payload, $seed, $reverse) = @_;
    my @bytes = unpack('C*', $payload);
    @bytes = reverse @bytes if $reverse;
    my $hash = $seed;

    for my $byte (@bytes) {
        $hash = (($hash ^ $byte) * $FNV_PRIME) & $UINT32_MASK;
    }
    $hash = (($hash ^ (scalar(@bytes) & 0xff)) * $FNV_PRIME) & $UINT32_MASK;
    return _avalanche32($hash);
}

sub _avalanche32 {
    my ($hash) = @_;
    $hash = ($hash ^ ($hash >> 16)) & $UINT32_MASK;
    $hash = ($hash * 1597334677) & $UINT32_MASK;
    $hash = ($hash ^ ($hash >> 15)) & $UINT32_MASK;
    $hash = ($hash * 1226822519) & $UINT32_MASK;
    return ($hash ^ ($hash >> 16)) & $UINT32_MASK;
}

sub _leading_zeros32 {
    my ($value) = @_;
    return 32 if $value == 0;
    my $zeros = 0;
    my $bit = 0x80000000;
    while (($value & $bit) == 0) {
        $zeros++;
        $bit >>= 1;
    }
    return $zeros;
}

1;

__END__

=head1 NAME

CodingAdventures::HyperLogLog - dependency-free approximate distinct counter

=head1 SYNOPSIS

  my $sketch = CodingAdventures::HyperLogLog->new(precision => 10);
  $sketch->add($_) for @values;
  print $sketch->count;

=cut
