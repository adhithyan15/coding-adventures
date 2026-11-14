package CodingAdventures::CanonicalCbor::Codec;

use strict;
use warnings;
use v5.28;
use feature 'bitwise';
no warnings 'experimental::bitwise';
no warnings 'recursion';

our $VERSION = '0.1.0';

use Encode qw(FB_CROAK LEAVE_SRC);
use CodingAdventures::CanonicalCbor::Constants;
use CodingAdventures::CanonicalCbor::Messages;
use CodingAdventures::CanonicalCbor::Types;

# ============================================================================
# coding_adventures.canonical_cbor.codec
# ============================================================================
# Deterministic canonical CBOR encoder and decoder (RFC 8949 CBR01 profile).
#
# Knuth-style literate implementation guaranteeing:
# 1. Length-first canonical map key ordering (§4.2.3).
# 2. Minimal integer argument representation (§4.2.1).
# 3. 64-bit unsigned bounds checks to defeat integer overflow attacks.
# 4. Pre-allocation bounds validation (check_length) to defeat OOM bombs.
# 5. Strict nesting depth caps (MAX_NESTING_DEPTH = 128) against stack overflow.
# 6. Atomic encode_into_checked rollback on failure.
# 7. Zero inline diagnostic strings (all via Messages.pm).
# ============================================================================

# ----------------------------------------------------------------------------
# Internal Buffer for Encoding
# ----------------------------------------------------------------------------

sub _new_buffer {
    return {
        length => 0,
        chunks => [],
    };
}

sub _push_byte {
    my ($buffer, $b) = @_;
    if ($buffer->{length} >= CodingAdventures::CanonicalCbor::Constants::MAX_ENCODED_BYTES) {
        CodingAdventures::CanonicalCbor::Types::raise_error('encode-too-large');
    }
    push @{$buffer->{chunks}}, pack('C', $b);
    $buffer->{length}++;
}

sub _push_bytes {
    my ($buffer, $bytes) = @_;
    my $len = length($bytes);
    if ($buffer->{length} + $len > CodingAdventures::CanonicalCbor::Constants::MAX_ENCODED_BYTES) {
        CodingAdventures::CanonicalCbor::Types::raise_error('encode-too-large');
    }
    push @{$buffer->{chunks}}, $bytes;
    $buffer->{length} += $len;
}

sub _write_type_and_argument {
    my ($buffer, $major, $arg) = @_;
    my $lead = $major << CodingAdventures::CanonicalCbor::Constants::MAJOR_TYPE_SHIFT;

    if ($arg <= CodingAdventures::CanonicalCbor::Constants::INFO_INLINE_MAX) {
        _push_byte($buffer, $lead | $arg);
        return;
    }
    if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT8) {
        _push_byte($buffer, $lead | CodingAdventures::CanonicalCbor::Constants::INFO_1_BYTE);
        _push_byte($buffer, $arg);
        return;
    }
    if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT16) {
        _push_byte($buffer, $lead | CodingAdventures::CanonicalCbor::Constants::INFO_2_BYTES);
        _push_bytes($buffer, pack('n', $arg));
        return;
    }
    if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT32) {
        _push_byte($buffer, $lead | CodingAdventures::CanonicalCbor::Constants::INFO_4_BYTES);
        _push_bytes($buffer, pack('N', $arg));
        return;
    }
    _push_byte($buffer, $lead | CodingAdventures::CanonicalCbor::Constants::INFO_8_BYTES);
    _push_bytes($buffer, pack('Q>', $arg));
}

sub _encode_map {
    my ($entries, $buffer, $depth) = @_;

    # Encode all keys and collect pair descriptors
    my @pairs;
    for my $entry (@$entries) {
        my $key_buf = _new_buffer();
        _encode_value($entry->key, $key_buf, $depth + 1);
        my $k_wire = join('', @{$key_buf->{chunks}});
        push @pairs, [ $k_wire, $entry ];
    }

    # Canonical Length-First order (§4.2.3):
    # 1. Shorter byte length precedes longer byte length.
    # 2. Equal length sorted by lexicographical byte comparison (memcmp).
    @pairs = sort {
        (length($a->[0]) <=> length($b->[0])) ||
        ($a->[0] cmp $b->[0])
    } @pairs;

    # Duplicate key detection (§4.2.3)
    for my $i (1 .. $#pairs) {
        if ($pairs[$i]->[0] eq $pairs[$i - 1]->[0]) {
            CodingAdventures::CanonicalCbor::Types::raise_error('duplicate-map-key');
        }
    }

    _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_MAP, scalar(@pairs));

    for my $pair (@pairs) {
        _push_bytes($buffer, $pair->[0]);
        _encode_value($pair->[1]->value, $buffer, $depth + 1);
    }
}

sub _encode_value {
    my ($v, $buffer, $depth) = @_;
    if ($depth > CodingAdventures::CanonicalCbor::Constants::MAX_NESTING_DEPTH) {
        CodingAdventures::CanonicalCbor::Types::raise_error('encode-too-deep');
    }

    if (!CodingAdventures::CanonicalCbor::Types::is_cbor_value($v)) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
    }

    my $k = $v->kind;
    if ($k eq 'unsigned') {
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_UNSIGNED, $v->value);
        return;
    }
    if ($k eq 'negative') {
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_NEGATIVE, $v->value);
        return;
    }
    if ($k eq 'bytes') {
        my $b = $v->bytes;
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_BYTES, length($b));
        _push_bytes($buffer, $b);
        return;
    }
    if ($k eq 'text') {
        my $octets = $v->octets;
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_TEXT, length($octets));
        _push_bytes($buffer, $octets);
        return;
    }
    if ($k eq 'array') {
        my $items = $v->items;
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_ARRAY, scalar(@$items));
        for my $item (@$items) {
            _encode_value($item, $buffer, $depth + 1);
        }
        return;
    }
    if ($k eq 'map') {
        _encode_map($v->entries, $buffer, $depth);
        return;
    }
    if ($k eq 'tag') {
        _write_type_and_argument($buffer, CodingAdventures::CanonicalCbor::Constants::MAJOR_TAG, $v->tag);
        _encode_value($v->value, $buffer, $depth + 1);
        return;
    }
    if ($k eq 'boolean') {
        _push_byte($buffer, $v->value
            ? CodingAdventures::CanonicalCbor::Constants::WIRE_BYTE_TRUE
            : CodingAdventures::CanonicalCbor::Constants::WIRE_BYTE_FALSE);
        return;
    }
    # k eq 'null'
    _push_byte($buffer, CodingAdventures::CanonicalCbor::Constants::WIRE_BYTE_NULL);
}

# Deterministically encode a CborValue into a canonical CBOR byte string.
sub encode_checked {
    my ($value) = @_;
    my $buf = _new_buffer();
    _encode_value($value, $buf, CodingAdventures::CanonicalCbor::Constants::ROOT_DEPTH);
    return join('', @{$buf->{chunks}});
}

# Deterministically encode a CborValue and append the resulting chunk to dst_array.
# Atomic: dst_array is validated and remains completely unmodified if encoding fails.
sub encode_into_checked {
    my ($value, $dst_array) = @_;
    if (!defined $dst_array || ref($dst_array) ne 'ARRAY' || tied(@$dst_array)) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
    }
    my $encoded = encode_checked($value);
    push @$dst_array, $encoded;
}

# ----------------------------------------------------------------------------
# Decoding Helpers
# ----------------------------------------------------------------------------

sub _read_u8 {
    my ($cursor) = @_;
    if ($cursor->{pos} >= $cursor->{len}) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unexpected-eof');
    }
    my $b = ord(substr($cursor->{data}, $cursor->{pos}, 1));
    $cursor->{pos}++;
    return $b;
}

sub _read_n {
    my ($cursor, $n) = @_;
    if ($cursor->{pos} + $n > $cursor->{len}) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unexpected-eof');
    }
    my $s = substr($cursor->{data}, $cursor->{pos}, $n);
    $cursor->{pos} += $n;
    return $s;
}

sub _read_big_endian_uint {
    my ($cursor, $byte_count) = @_;
    my $bytes = _read_n($cursor, $byte_count);
    return ord($bytes) if $byte_count == CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_1_BYTE;
    return unpack('n', $bytes) if $byte_count == CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_2_BYTES;
    return unpack('N', $bytes) if $byte_count == CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_4_BYTES;
    return unpack('Q>', $bytes);
}

# Verify that declared length does not exceed remaining buffer capacity.
sub _check_length {
    my ($declared, $remaining, $min_bytes_per_unit) = @_;
    my $max_allowed = int($remaining / $min_bytes_per_unit);
    if ($declared > $max_allowed) {
        CodingAdventures::CanonicalCbor::Types::raise_error('length-too-large');
    }
    return $declared;
}

# Verify RFC 8949 Section 4.2.1 minimal integer encoding.
sub _validate_minimal_argument {
    my ($major, $info, $arg) = @_;
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_SIMPLE_OR_FLOAT) {
        if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_1_BYTE) {
            if ($arg < CodingAdventures::CanonicalCbor::Constants::MIN_1_BYTE_SIMPLE_VALUE) {
                CodingAdventures::CanonicalCbor::Types::raise_error('non-minimal-integer');
            }
        }
        return;
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_1_BYTE) {
        if ($arg <= CodingAdventures::CanonicalCbor::Constants::INFO_INLINE_MAX) {
            CodingAdventures::CanonicalCbor::Types::raise_error('non-minimal-integer');
        }
        return;
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_2_BYTES) {
        if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT8) {
            CodingAdventures::CanonicalCbor::Types::raise_error('non-minimal-integer');
        }
        return;
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_4_BYTES) {
        if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT16) {
            CodingAdventures::CanonicalCbor::Types::raise_error('non-minimal-integer');
        }
        return;
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_8_BYTES) {
        if ($arg <= CodingAdventures::CanonicalCbor::Constants::MAX_UINT32) {
            CodingAdventures::CanonicalCbor::Types::raise_error('non-minimal-integer');
        }
        return;
    }
}

sub _read_header {
    my ($cursor) = @_;
    my $header_byte = _read_u8($cursor);
    my $major = ($header_byte >> CodingAdventures::CanonicalCbor::Constants::MAJOR_TYPE_SHIFT)
        & CodingAdventures::CanonicalCbor::Constants::MAJOR_TYPE_MASK;
    my $info = $header_byte & CodingAdventures::CanonicalCbor::Constants::ADDITIONAL_INFO_MASK;

    my $arg;
    if ($info <= CodingAdventures::CanonicalCbor::Constants::INFO_INLINE_MAX) {
        $arg = $info;
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_1_BYTE) {
        $arg = _read_u8($cursor);
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_2_BYTES) {
        $arg = _read_big_endian_uint($cursor, CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_2_BYTES);
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_4_BYTES) {
        $arg = _read_big_endian_uint($cursor, CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_4_BYTES);
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_8_BYTES) {
        $arg = _read_big_endian_uint($cursor, CodingAdventures::CanonicalCbor::Constants::ARG_WIDTH_8_BYTES);
    }
    if ($info >= CodingAdventures::CanonicalCbor::Constants::INFO_RESERVED_28 &&
        $info <= CodingAdventures::CanonicalCbor::Constants::INFO_RESERVED_30) {
        CodingAdventures::CanonicalCbor::Types::raise_error('reserved');
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::INFO_INDEFINITE) {
        CodingAdventures::CanonicalCbor::Types::raise_error('indefinite');
    }

    _validate_minimal_argument($major, $info, $arg);
    return ($major, $info, $arg);
}

sub _decode_value {
    my ($cursor, $depth) = @_;
    if ($depth > CodingAdventures::CanonicalCbor::Constants::MAX_NESTING_DEPTH) {
        CodingAdventures::CanonicalCbor::Types::raise_error('too-deep');
    }

    my ($major, $info, $arg) = _read_header($cursor);

    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_UNSIGNED) {
        return CodingAdventures::CanonicalCbor::Types::CborUnsigned($arg);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_NEGATIVE) {
        return CodingAdventures::CanonicalCbor::Types::CborNegative($arg);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_BYTES) {
        _check_length($arg, $cursor->{len} - $cursor->{pos},
            CodingAdventures::CanonicalCbor::Constants::MIN_BYTES_PER_RAW_BYTE);
        my $b = _read_n($cursor, $arg);
        return CodingAdventures::CanonicalCbor::Types::CborBytes($b);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_TEXT) {
        _check_length($arg, $cursor->{len} - $cursor->{pos},
            CodingAdventures::CanonicalCbor::Constants::MIN_BYTES_PER_RAW_BYTE);
        my $octets = _read_n($cursor, $arg);
        my $str = eval { Encode::decode('UTF-8', $octets, FB_CROAK | LEAVE_SRC) };
        if ($@) {
            CodingAdventures::CanonicalCbor::Types::raise_error('invalid-utf8');
        }
        return CodingAdventures::CanonicalCbor::Types::CborText($str);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_ARRAY) {
        _check_length($arg, $cursor->{len} - $cursor->{pos},
            CodingAdventures::CanonicalCbor::Constants::MIN_BYTES_PER_ARRAY_ITEM);
        my @items;
        for (1 .. $arg) {
            push @items, _decode_value($cursor, $depth + 1);
        }
        return CodingAdventures::CanonicalCbor::Types::CborArray(\@items);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_MAP) {
        my $spanned_entries = _read_spanned_map_entries($cursor, $depth, $arg);
        my @cbor_entries = map {
            CodingAdventures::CanonicalCbor::Types::CborMapEntry($_->key, $_->value)
        } @$spanned_entries;
        return CodingAdventures::CanonicalCbor::Types::CborMap(\@cbor_entries);
    }
    if ($major == CodingAdventures::CanonicalCbor::Constants::MAJOR_TAG) {
        my $inner = _decode_value($cursor, $depth + 1);
        return CodingAdventures::CanonicalCbor::Types::CborTag($arg, $inner);
    }
    # major == MAJOR_SIMPLE_OR_FLOAT
    if ($info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_FALSE) {
        return CodingAdventures::CanonicalCbor::Types::CborBool(0);
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_TRUE) {
        return CodingAdventures::CanonicalCbor::Types::CborBool(1);
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_NULL) {
        return CodingAdventures::CanonicalCbor::Types::CborNull();
    }
    if ($info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_FLOAT_16 ||
        $info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_FLOAT_32 ||
        $info == CodingAdventures::CanonicalCbor::Constants::SIMPLE_FLOAT_64) {
        CodingAdventures::CanonicalCbor::Types::raise_error('float-not-supported');
    }
    CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
}

# Shared parser for map entries with canonical key order and duplicate detection.
sub _read_spanned_map_entries {
    my ($cursor, $depth, $count) = @_;
    _check_length($count, $cursor->{len} - $cursor->{pos},
        CodingAdventures::CanonicalCbor::Constants::MIN_BYTES_PER_MAP_ENTRY);

    my @entries;
    my $prev_key_wire;

    for (1 .. $count) {
        my $key_start = $cursor->{pos};
        my $key = _decode_value($cursor, $depth + 1);
        my $key_finish = $cursor->{pos};
        my $key_wire = substr($cursor->{data}, $key_start, $key_finish - $key_start);

        if (defined $prev_key_wire) {
            my $cmp = (length($prev_key_wire) <=> length($key_wire)) ||
                      ($prev_key_wire cmp $key_wire);
            if ($cmp >= 0) {
                CodingAdventures::CanonicalCbor::Types::raise_error('non-canonical-map-order');
            }
        }
        $prev_key_wire = $key_wire;

        my $val_start = $cursor->{pos};
        my $val = _decode_value($cursor, $depth + 1);
        my $val_finish = $cursor->{pos};
        my $val_span = CodingAdventures::CanonicalCbor::Types::Span($val_start, $val_finish);

        push @entries, CodingAdventures::CanonicalCbor::Types::SpannedMapEntry($key, $val, $val_span);
    }

    return \@entries;
}

# Strict single-item canonical CBOR decoder.
sub decode {
    if (!@_ || tied($_[0])) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
    }
    my $norm = CodingAdventures::CanonicalCbor::Types::_validate_and_normalize_bytes($_[0]);
    my $cursor = {
        data => $norm,
        pos  => 0,
        len  => length($norm),
    };

    my $val = _decode_value($cursor, CodingAdventures::CanonicalCbor::Constants::ROOT_DEPTH);

    if ($cursor->{pos} < $cursor->{len}) {
        CodingAdventures::CanonicalCbor::Types::raise_error('trailing-bytes');
    }

    return $val;
}

# Strict CBR01 map span decoder.
sub decode_map_spans {
    if (!@_ || tied($_[0])) {
        CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
    }
    my $norm = CodingAdventures::CanonicalCbor::Types::_validate_and_normalize_bytes($_[0]);
    my $cursor = {
        data => $norm,
        pos  => 0,
        len  => length($norm),
    };

    my ($major, $info, $arg) = _read_header($cursor);

    if ($major != CodingAdventures::CanonicalCbor::Constants::MAJOR_MAP) {
        # Non-map item: validate that the entire item is well-formed canonical CBOR with no trailing bytes
        $cursor->{pos} = 0;
        _decode_value($cursor, CodingAdventures::CanonicalCbor::Constants::ROOT_DEPTH);
        if ($cursor->{pos} < $cursor->{len}) {
            CodingAdventures::CanonicalCbor::Types::raise_error('trailing-bytes');
        }
        return $CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult->non_map();
    }

    my $entries = _read_spanned_map_entries($cursor, CodingAdventures::CanonicalCbor::Constants::ROOT_DEPTH, $arg);

    if ($cursor->{pos} < $cursor->{len}) {
        CodingAdventures::CanonicalCbor::Types::raise_error('trailing-bytes');
    }

    return $CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult->map($entries);
}

1;
