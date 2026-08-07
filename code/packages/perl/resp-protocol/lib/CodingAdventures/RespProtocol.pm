package CodingAdventures::RespProtocol;

use strict;
use warnings;
use utf8;
use Encode ();
use Exporter 'import';
use Scalar::Util qw(blessed looks_like_number);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    simple_string error_value integer bulk_string null_bulk_string
    array_value null_array encode encode_many encode_simple_string
    encode_error encode_integer encode_bulk_string encode_array decode
    decode_all equal
);

use constant MAX_BULK_LENGTH  => 512 * 1024 * 1024;
use constant MAX_ARRAY_LENGTH => 1_000_000;
use constant MAX_NESTING_DEPTH => 128;

our %VALID_KIND = map { $_ => 1 } qw(
    simple_string error integer bulk_string array
);

sub _is_value {
    my ($value) = @_;
    return blessed($value) && $value->isa('CodingAdventures::RespProtocol::Value');
}

sub _assert_scalar {
    my ($value, $name) = @_;
    die "$name must be a string\n" if !defined($value) || ref($value);
}

sub _to_bytes {
    my ($value, $name) = @_;
    _assert_scalar($value, $name);
    return utf8::is_utf8($value) ? Encode::encode_utf8($value) : $value;
}

sub _decode_text {
    my ($bytes, $context) = @_;
    my $value = eval { Encode::decode('UTF-8', $bytes, Encode::FB_CROAK) };
    die "invalid UTF-8 in $context\n" if !defined($value) || $@;
    return $value;
}

sub _parse_i64 {
    my ($text, $context) = @_;
    die "invalid $context: $text\n" if !defined($text) || $text !~ /^-?\d+$/;

    my $negative = $text =~ /^-/ ? 1 : 0;
    my $digits = $negative ? substr($text, 1) : $text;
    $digits =~ s/^0+(?=\d)//;
    my $limit = $negative ? '9223372036854775808' : '9223372036854775807';
    if (length($digits) > length($limit)
        || (length($digits) == length($limit) && $digits gt $limit)) {
        die "$context is outside the signed 64-bit range\n";
    }
    my $normalized = $negative && $digits ne '0' ? "-$digits" : $digits;
    return 0 + $normalized;
}

sub simple_string { return 'CodingAdventures::RespProtocol::Value'->simple_string(@_); }
sub error_value   { return 'CodingAdventures::RespProtocol::Value'->error(@_); }
sub integer       { return 'CodingAdventures::RespProtocol::Value'->integer(@_); }
sub bulk_string   { return 'CodingAdventures::RespProtocol::Value'->bulk_string(@_); }
sub null_bulk_string { return 'CodingAdventures::RespProtocol::Value'->null_bulk_string(@_); }
sub array_value   { return 'CodingAdventures::RespProtocol::Value'->array(@_); }
sub null_array    { return 'CodingAdventures::RespProtocol::Value'->null_array(@_); }

sub equal {
    my ($left, $right) = @_;
    return 0 if !_is_value($left) || !_is_value($right);
    return 0 if $left->kind ne $right->kind || $left->is_null != $right->is_null;
    return 1 if $left->is_null;
    if ($left->kind ne 'array') {
        return $left->value eq $right->value ? 1 : 0;
    }
    my $left_values = $left->value;
    my $right_values = $right->value;
    return 0 if @$left_values != @$right_values;
    for my $index (0 .. $#$left_values) {
        return 0 if !equal($left_values->[$index], $right_values->[$index]);
    }
    return 1;
}

sub encode_simple_string {
    my ($value) = @_;
    _assert_scalar($value, 'value');
    die "simple string must not contain CR or LF\n" if $value =~ /[\r\n]/;
    return '+' . _to_bytes($value, 'value') . "\r\n";
}

sub encode_error {
    my ($value) = @_;
    _assert_scalar($value, 'value');
    die "error string must not contain CR or LF\n" if $value =~ /[\r\n]/;
    return '-' . _to_bytes($value, 'value') . "\r\n";
}

sub encode_integer {
    my ($value) = @_;
    die "value must be an integer\n"
        if !defined($value) || ref($value) || "$value" !~ /^-?\d+$/;
    my $number = _parse_i64("$value", 'integer');
    return ':' . $number . "\r\n";
}

sub encode_bulk_string {
    my ($value) = @_;
    return "\$-1\r\n" if !defined $value;
    my $bytes = _to_bytes($value, 'value');
    die "bulk string exceeds maximum length\n" if length($bytes) > MAX_BULK_LENGTH;
    return '$' . length($bytes) . "\r\n" . $bytes . "\r\n";
}

sub encode_array {
    my ($values) = @_;
    return "*-1\r\n" if !defined $values;
    die "values must be an array reference or undef\n" if ref($values) ne 'ARRAY';
    die "array exceeds maximum length\n" if @$values > MAX_ARRAY_LENGTH;
    return '*' . scalar(@$values) . "\r\n" . join('', map { encode($_) } @$values);
}

sub _encode_value {
    my ($value) = @_;
    return encode_simple_string($value->value) if $value->kind eq 'simple_string';
    return encode_error($value->value) if $value->kind eq 'error';
    return encode_integer($value->value) if $value->kind eq 'integer';
    if ($value->kind eq 'bulk_string') {
        return encode_bulk_string($value->is_null ? undef : $value->value);
    }
    if ($value->kind eq 'array') {
        return encode_array($value->is_null ? undef : $value->value);
    }
    die 'invalid RESP kind: ' . $value->kind . "\n";
}

sub encode {
    my ($value) = @_;
    return _encode_value($value) if _is_value($value);
    return encode_bulk_string(undef) if !defined $value;
    return encode_array($value) if ref($value) eq 'ARRAY';
    if (!ref($value)) {
        return encode_integer($value)
            if looks_like_number($value) && "$value" =~ /^-?\d+$/;
        return encode_bulk_string($value);
    }
    die 'cannot encode value of type ' . ref($value) . "\n";
}

sub encode_many {
    my ($values) = @_;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';
    return join('', map { encode($_) } @$values);
}

sub _read_line {
    my ($data, $offset) = @_;
    my $end = index($data, "\r\n", $offset);
    return undef if $end < 0;
    return [substr($data, $offset, $end - $offset), $end + 2];
}

sub _decode_value {
    my ($data, $offset, $depth) = @_;
    die "RESP array nesting exceeds maximum depth\n" if $depth > MAX_NESTING_DEPTH;
    return undef if $offset >= length($data);

    my $prefix = substr($data, $offset, 1);
    if ($prefix eq '+' || $prefix eq '-' || $prefix eq ':') {
        my $line = _read_line($data, $offset + 1);
        return undef if !defined $line;
        my ($payload, $next) = @$line;
        if ($prefix eq '+') {
            return [simple_string(_decode_text($payload, 'simple string')), $next];
        }
        if ($prefix eq '-') {
            return [error_value(_decode_text($payload, 'error string')), $next];
        }
        return [integer(_parse_i64($payload, 'RESP integer')), $next];
    }

    if ($prefix eq '$') {
        my $line = _read_line($data, $offset + 1);
        return undef if !defined $line;
        my ($length_text, $payload_offset) = @$line;
        my $length = _parse_i64($length_text, 'bulk length');
        return [null_bulk_string(), $payload_offset] if $length == -1;
        die "invalid negative bulk length: $length\n" if $length < -1;
        die "bulk length exceeds maximum\n" if $length > MAX_BULK_LENGTH;

        my $terminator_offset = $payload_offset + $length;
        return undef if $terminator_offset + 2 > length($data);
        die "bulk string missing trailing CRLF\n"
            if substr($data, $terminator_offset, 2) ne "\r\n";
        my $payload = substr($data, $payload_offset, $length);
        return [bulk_string($payload), $terminator_offset + 2];
    }

    if ($prefix eq '*') {
        my $line = _read_line($data, $offset + 1);
        return undef if !defined $line;
        my ($count_text, $cursor) = @$line;
        my $count = _parse_i64($count_text, 'array length');
        return [null_array(), $cursor] if $count == -1;
        die "invalid negative array length: $count\n" if $count < -1;
        die "array length exceeds maximum\n" if $count > MAX_ARRAY_LENGTH;

        my @values;
        for (1 .. $count) {
            my $decoded = _decode_value($data, $cursor, $depth + 1);
            return undef if !defined $decoded;
            push @values, $decoded->[0];
            $cursor = $decoded->[1];
        }
        return [array_value(\@values), $cursor];
    }

    my $line = _read_line($data, $offset);
    return undef if !defined $line;
    my ($payload, $next) = @$line;
    my @tokens = grep { length } split /\s+/, $payload;
    return [array_value([map { bulk_string($_) } @tokens]), $next];
}

sub decode {
    my ($data, $offset) = @_;
    $data = _to_bytes($data, 'data');
    $offset = 0 if !defined $offset;
    die "offset must be an integer\n" if ref($offset) || "$offset" !~ /^\d+$/;
    die "offset is outside data\n" if $offset < 0 || $offset > length($data);
    return _decode_value($data, $offset, 0);
}

sub decode_all {
    my ($data, $offset) = @_;
    $data = _to_bytes($data, 'data');
    $offset = 0 if !defined $offset;
    die "offset must be an integer\n" if ref($offset) || "$offset" !~ /^\d+$/;
    die "offset is outside data\n" if $offset < 0 || $offset > length($data);

    my @values;
    my $cursor = $offset;
    while ($cursor < length($data)) {
        my $decoded = _decode_value($data, $cursor, 0);
        last if !defined $decoded;
        push @values, $decoded->[0];
        $cursor = $decoded->[1];
    }
    return [\@values, $cursor];
}

package CodingAdventures::RespProtocol::Value;

use strict;
use warnings;
use overload '""' => 'as_string', fallback => 1;

sub new {
    my ($class, $kind, $value) = @_;
    die 'invalid RESP kind: ' . (defined($kind) ? $kind : 'undef') . "\n"
        if !$CodingAdventures::RespProtocol::VALID_KIND{$kind};

    my $self = { kind => $kind, value => $value, is_null => 0 };
    if ($kind eq 'simple_string' || $kind eq 'error') {
        CodingAdventures::RespProtocol::_assert_scalar($value, "$kind value");
        die "$kind value must not contain CR or LF\n" if $value =~ /[\r\n]/;
        if ($kind eq 'error') {
            my ($error_type, $detail) = split / /, $value, 2;
            $self->{error_type} = defined($error_type) ? $error_type : '';
            $self->{detail} = defined($detail) ? $detail : '';
        }
    } elsif ($kind eq 'integer') {
        die "integer value must be an integer\n"
            if !defined($value) || ref($value) || "$value" !~ /^-?\d+$/;
        $self->{value} = CodingAdventures::RespProtocol::_parse_i64("$value", 'integer');
    } elsif ($kind eq 'bulk_string') {
        if (!defined $value) {
            $self->{is_null} = 1;
        } else {
            CodingAdventures::RespProtocol::_assert_scalar($value, 'bulk_string value');
        }
    } elsif ($kind eq 'array') {
        if (!defined $value) {
            $self->{is_null} = 1;
        } else {
            die "array value must be an array reference or undef\n" if ref($value) ne 'ARRAY';
            my @copy;
            for my $index (0 .. $#$value) {
                die "array value[$index] must be a RESP Value\n"
                    if !CodingAdventures::RespProtocol::_is_value($value->[$index]);
                push @copy, $value->[$index];
            }
            $self->{value} = \@copy;
        }
    }
    return bless $self, $class;
}

sub simple_string { my ($class, $value) = @_; return $class->new('simple_string', $value); }
sub error         { my ($class, $value) = @_; return $class->new('error', $value); }
sub integer       { my ($class, $value) = @_; return $class->new('integer', $value); }
sub bulk_string   { my ($class, $value) = @_; return $class->new('bulk_string', $value); }
sub null_bulk_string { my ($class) = @_; return $class->new('bulk_string', undef); }
sub array         { my ($class, $value) = @_; return $class->new('array', $value); }
sub null_array    { my ($class) = @_; return $class->new('array', undef); }

sub kind       { return $_[0]->{kind}; }
sub value      { return $_[0]->{value}; }
sub is_null    { return $_[0]->{is_null}; }
sub error_type { return $_[0]->{error_type}; }
sub detail     { return $_[0]->{detail}; }

sub as_string {
    my ($self) = @_;
    return $self->{kind} . '(null)' if $self->{is_null};
    return sprintf('array(%d values)', scalar @{$self->{value}}) if $self->{kind} eq 'array';
    return $self->{kind} . '(' . $self->{value} . ')';
}

package CodingAdventures::RespProtocol::Decoder;

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless { buffer => '', queue => [] }, $class;
}

sub feed {
    my ($self, $data) = @_;
    $self->{buffer} .= CodingAdventures::RespProtocol::_to_bytes($data, 'data');
    my ($values, $next) = @{CodingAdventures::RespProtocol::decode_all($self->{buffer})};
    push @{$self->{queue}}, @$values;
    $self->{buffer} = substr($self->{buffer}, $next) if $next > 0;
    return $self;
}

sub has_message { return @{$_[0]->{queue}} ? 1 : 0; }

sub get_message {
    my ($self) = @_;
    die "no decoded message is available\n" if !@{$self->{queue}};
    return shift @{$self->{queue}};
}

sub drain {
    my ($self) = @_;
    my $result = $self->{queue};
    $self->{queue} = [];
    return $result;
}

sub decode_all {
    my ($self, $data) = @_;
    $self->feed($data);
    return $self->drain;
}

sub pending_bytes { return length($_[0]->{buffer}); }

1;

__END__

=head1 NAME

CodingAdventures::RespProtocol - typed RESP2 codec and streaming decoder

=cut
