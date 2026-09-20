package CodingAdventures::DerTlv;

use strict;
use warnings;
use bytes;
use Exporter qw(import);

our $VERSION = '0.01';
our @EXPORT_OK = qw(decode_one decode_exact new_cursor default_limits);

use constant HOST_MAX => 9_223_372_036_854_775_807;
use constant U32_MAX  => 4_294_967_295;

my @TAG_CLASSES = qw(universal application context-specific private);
my %DEFAULT_LIMITS = (
    max_input_len  => 1_048_576,
    max_value_len  => 1_048_576,
    max_elements   => 4_096,
    max_tag_number => U32_MAX,
);

sub default_limits { return {%DEFAULT_LIMITS}; }

sub _limits {
    my ($provided) = @_;
    my %limits = (%DEFAULT_LIMITS, %{ $provided // {} });
    for my $name (keys %DEFAULT_LIMITS) {
        die "$name must be a non-negative integer"
            if !defined($limits{$name}) || $limits{$name} !~ /\A\d+\z/;
    }
    die "max_tag_number must fit u32" if $limits{max_tag_number} > U32_MAX;
    return \%limits;
}

sub _fail {
    my ($kind, $offset) = @_;
    die CodingAdventures::DerTlv::Error->new($kind, $offset);
}

sub _octet {
    my ($input, $offset) = @_;
    return ord(substr($input, $offset, 1));
}

sub _decode_high_tag {
    my ($input, $start, $available, $limits) = @_;
    my $number = 0;
    my $index  = 1;
    while (1) {
        _fail('truncated-high-tag', $start + $index) if $index >= $available;
        my $octet   = _octet($input, $start + $index);
        my $payload = $octet & 0x7f;
        _fail('non-minimal-tag', $start + $index) if $index == 1 && $payload == 0;
        _fail('tag-overflow', $start + $index)
            if $number > int((U32_MAX - $payload) / 128);
        $number = ($number * 128) + $payload;
        _fail('tag-limit-exceeded', $start + $index)
            if $number > $limits->{max_tag_number};
        ++$index;
        last if ($octet & 0x80) == 0;
    }
    _fail('non-minimal-tag', $start) if $number < 31;
    return ($number, $index);
}

sub _decode_length {
    my ($input, $start, $available, $identifier_len) = @_;
    my $length_offset = $start + $identifier_len;
    _fail('truncated-length', $length_offset) if $identifier_len >= $available;

    my $first = _octet($input, $length_offset);
    return ($first, 1, $length_offset) if $first < 0x80;
    _fail('indefinite-length', $length_offset) if $first == 0x80;
    _fail('reserved-length', $length_offset)   if $first == 0xff;

    my $count = $first & 0x7f;
    _fail('length-too-wide', $length_offset) if $count > 8;
    _fail('truncated-length', $start + $available)
        if $identifier_len + 1 + $count > $available;
    my $leading = _octet($input, $length_offset + 1);
    _fail('non-minimal-length', $length_offset + 1) if $leading == 0;
    _fail('length-host-overflow', $length_offset)
        if $count == 8 && ($leading & 0x80) != 0;

    my $value = 0;
    for my $index (0 .. $count - 1) {
        $value = ($value * 256) + _octet($input, $length_offset + 1 + $index);
    }
    _fail('non-minimal-length', $length_offset) if $value < 128;
    return ($value, $count + 1, $length_offset);
}

sub _decode_at {
    my ($input, $start, $available, $limits) = @_;
    _fail('input-limit-exceeded', $start) if $available > $limits->{max_input_len};
    _fail('empty-input', $start)          if $available == 0;

    my $first       = _octet($input, $start);
    my $tag_class   = $TAG_CLASSES[$first >> 6];
    my $constructed = ($first & 0x20) != 0 ? 1 : 0;
    my $low         = $first & 0x1f;
    my ($number, $identifier_len);
    if ($low != 0x1f) {
        $number         = $low;
        $identifier_len = 1;
        _fail('tag-limit-exceeded', $start) if $number > $limits->{max_tag_number};
    }
    else {
        ($number, $identifier_len) = _decode_high_tag($input, $start, $available, $limits);
    }
    _fail('end-of-contents', $start) if $tag_class eq 'universal' && $number == 0;

    my ($value_len, $length_len, $length_offset) =
        _decode_length($input, $start, $available, $identifier_len);
    _fail('value-limit-exceeded', $length_offset)
        if $value_len > $limits->{max_value_len};
    my $header_len = $identifier_len + $length_len;
    _fail('length-host-overflow', $length_offset) if $value_len > HOST_MAX - $header_len;
    my $encoded_len = $header_len + $value_len;
    _fail('truncated-value', $start + $available) if $encoded_len > $available;

    my $element = CodingAdventures::DerTlv::Element->new(
        tag => {
            class       => $tag_class,
            constructed => $constructed,
            number      => $number,
        },
        input       => $input,
        start       => $start,
        header_len  => $header_len,
        encoded_len => $encoded_len,
    );
    return ($element, $start + $encoded_len);
}

sub decode_one {
    my ($input, $provided) = @_;
    die "input must be a byte string" if ref($input);
    my ($element, $next) = _decode_at($input, 0, length($input), _limits($provided));
    return ($element, substr($input, $next));
}

sub decode_exact {
    my ($input, $provided) = @_;
    die "input must be a byte string" if ref($input);
    my ($element, $next) = _decode_at($input, 0, length($input), _limits($provided));
    _fail('trailing-data', $next) if $next != length($input);
    return $element;
}

sub new_cursor {
    my ($input, $provided) = @_;
    return CodingAdventures::DerTlv::Cursor->new($input, $provided);
}

package CodingAdventures::DerTlv::Error;

use strict;
use warnings;
use overload q{""} => 'message', fallback => 1;

sub new {
    my ($class, $kind, $offset) = @_;
    return bless {kind => $kind, offset => $offset}, $class;
}

sub kind   { return $_[0]->{kind}; }
sub offset { return $_[0]->{offset}; }

sub message {
    my ($self) = @_;
    return "DER framing error $self->{kind} at byte $self->{offset}";
}

package CodingAdventures::DerTlv::Element;

use strict;
use warnings;
use bytes;

sub new {
    my ($class, %fields) = @_;
    return bless \%fields, $class;
}

sub tag { return $_[0]->{tag}; }

sub header {
    my ($self) = @_;
    return substr($self->{input}, $self->{start}, $self->{header_len});
}

sub value {
    my ($self) = @_;
    return substr(
        $self->{input},
        $self->{start} + $self->{header_len},
        $self->{encoded_len} - $self->{header_len},
    );
}

sub encoded {
    my ($self) = @_;
    return substr($self->{input}, $self->{start}, $self->{encoded_len});
}

package CodingAdventures::DerTlv::Cursor;

use strict;
use warnings;
use bytes;

sub new {
    my ($class, $input, $provided) = @_;
    die "input must be a byte string" if ref($input);
    my $limits = CodingAdventures::DerTlv::_limits($provided);
    CodingAdventures::DerTlv::_fail('input-limit-exceeded', 0)
        if length($input) > $limits->{max_input_len};
    return bless {
        input         => $input,
        limits        => $limits,
        offset        => 0,
        elements_read => 0,
    }, $class;
}

sub elements_read { return $_[0]->{elements_read}; }

sub remaining {
    my ($self) = @_;
    return substr($self->{input}, $self->{offset});
}

sub read {
    my ($self) = @_;
    return undef if $self->{offset} == length($self->{input});
    CodingAdventures::DerTlv::_fail('element-limit-exceeded', $self->{offset})
        if $self->{elements_read} >= $self->{limits}->{max_elements};
    my ($element, $next) = CodingAdventures::DerTlv::_decode_at(
        $self->{input},
        $self->{offset},
        length($self->{input}) - $self->{offset},
        $self->{limits},
    );
    $self->{offset} = $next;
    ++$self->{elements_read};
    return $element;
}

sub finish {
    my ($self) = @_;
    CodingAdventures::DerTlv::_fail('trailing-data', $self->{offset})
        if $self->{offset} != length($self->{input});
    return;
}

1;
