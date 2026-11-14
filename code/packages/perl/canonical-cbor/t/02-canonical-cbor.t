use strict;
use warnings;
use v5.28;
use feature 'bitwise';
no warnings 'experimental::bitwise';
no warnings 'recursion';

use Test2::V0;
use CodingAdventures::CanonicalCbor qw(:all);
use CodingAdventures::CanonicalCbor::Constants;
use CodingAdventures::CanonicalCbor::Messages;
use CodingAdventures::CanonicalCbor::Types;

# ============================================================================
# t/02-canonical-cbor.t
# ============================================================================
# Comprehensive specification, security, and stress test suite covering:
# 1. 7-Class Perl CVE Resilience Showcase
# 2. Non-minimal integer header widening across major types 0..6
# 3. Convenience wrappers and atomic rollback
# 4. DecodeMapSpans exact 0-based wire spans and slice invariant
# 5. Deterministic PRNG property round-trip and map permutations
# 6. 10,000-iteration adversarial byte-mutation fuzz loop
# 7. Exhaustive coverage of equality, stringification, and defensive branches
# ============================================================================

sub make_prng {
    my ($seed) = @_;
    my $state = $seed & 0xFFFFFFFF;
    my $self;
    $self = {
        next_u32 => sub {
            $state = (($state * 1664525) + 1013904223) & 0xFFFFFFFF;
            return $state;
        },
        next_int => sub {
            my ($max) = @_;
            return 0 if $max <= 0;
            return $self->{next_u32}->() % $max;
        },
        next_bytes => sub {
            my ($len) = @_;
            my @b;
            for (1 .. $len) {
                push @b, pack('C', $self->{next_int}->(256));
            }
            return join('', @b);
        },
    };
    return $self;
}

# ============================================================================
# 1. 7-Class Perl CVE Resilience Showcase
# ============================================================================

subtest 'CVE-1: 64-bit integer wraparound and NV float precision trap' => sub {
    # Rejects float numbers (NV) in integer constructors
    my $v1 = eval { CborUnsigned(1.5) };
    ok(!defined $v1, "CborUnsigned rejects float 1.5");
    is($@->id, 'unsupported-simple', "Error is unsupported-simple");

    my $v2 = eval { CborNegative(2.5) };
    ok(!defined $v2, "CborNegative rejects float 2.5");
    is($@->id, 'unsupported-simple', "Error is unsupported-simple");

    my $v3 = eval { CborTag(1.5, CborNull()) };
    ok(!defined $v3, "CborTag rejects float tag");
    is($@->id, 'unsupported-simple', "Error is unsupported-simple");

    # Dualvar and non-canonical numeric string rejection
    require Scalar::Util;
    my $dv = Scalar::Util::dualvar(42, "evil");
    my $res_dv = eval { CborUnsigned($dv) };
    ok(!defined $res_dv, "CborUnsigned rejects dualvar(42, 'evil')");
    is($@->id, 'unsupported-simple', "Dualvar raises unsupported-simple");

    my $num_str = "0042";
    my $dummy = $num_str + 0; # ensure IOK is populated alongside POK
    my $res_num = eval { CborUnsigned($num_str) };
    ok(!defined $res_num, "CborUnsigned rejects non-canonical numeric string '0042'");
    is($@->id, 'unsupported-simple', "Non-canonical string raises unsupported-simple");

    # 64-bit boundaries (0, 2^32-1, 2^32, 2^64-1)
    my @boundaries = (
        0,
        23,
        24,
        255,
        256,
        65535,
        65536,
        4294967295,
        4294967296,
        ~0, # 18446744073709551615
    );

    for my $b (@boundaries) {
        my $u = CborUnsigned($b);
        my $enc = encode_checked($u);
        my $dec = decode($enc);
        ok($u == $dec, "CborUnsigned($b) preserves exact 64-bit value through roundtrip");
        is($dec->value, $b, "Decoded value matches exactly $b");

        my $n = CborNegative($b);
        my $enc_n = encode_checked($n);
        my $dec_n = decode($enc_n);
        ok($n == $dec_n, "CborNegative($b) preserves exact 64-bit value through roundtrip");
        is($dec_n->value, $b, "Decoded negative argument matches exactly $b");
    }
};

subtest 'CVE-2: Pre-allocation OOM expansion bomb defense' => sub {
    # Declares 2^64 - 1 bytes with 0 remaining
    my $hostile_bytes = "\x5b\xff\xff\xff\xff\xff\xff\xff\xff";
    my $res1 = eval { decode($hostile_bytes) };
    my $err1 = $@;
    ok(!defined $res1, "Hostile bytes length rejected before allocation");
    is($err1->id, 'length-too-large', "Error is length-too-large");

    # Declares 2^64 - 1 text length with 0 remaining
    my $hostile_text = "\x7b\xff\xff\xff\xff\xff\xff\xff\xff";
    my $res2 = eval { decode($hostile_text) };
    my $err2 = $@;
    ok(!defined $res2, "Hostile text length rejected before allocation");
    is($err2->id, 'length-too-large', "Error is length-too-large");

    # Declares array of 100 items with only 1 byte remaining
    my $hostile_array = "\x98\x64\x01";
    my $res3 = eval { decode($hostile_array) };
    my $err3 = $@;
    ok(!defined $res3, "Hostile array length rejected before allocation");
    is($err3->id, 'length-too-large', "Error is length-too-large");

    # Declares map of 100 entries (needs at least 200 bytes) with 2 bytes remaining
    my $hostile_map = "\xb8\x64\x00\x01";
    my $res4 = eval { decode($hostile_map) };
    my $err4 = $@;
    ok(!defined $res4, "Hostile map length rejected before allocation");
    is($err4->id, 'length-too-large', "Error is length-too-large");
};

subtest 'CVE-3: Deep nesting recursion stack exhaustion (depth 128 vs 129)' => sub {
    # Depth 128 succeeds on encode and decode
    my $cur = CborArray([]);
    for (1 .. 128) {
        $cur = CborArray([$cur]);
    }
    my $enc = eval { encode_checked($cur) };
    is($@, '', "Depth 128 encodes cleanly");
    my $dec = eval { decode($enc) };
    is($@, '', "Depth 128 decodes cleanly");
    ok($cur == $dec, "Depth 128 equals decoded");

    # Depth 129 fails on encode
    my $deep_129 = CborArray([$cur]);
    my $enc_fail = eval { encode_checked($deep_129) };
    my $err_enc = $@;
    ok(!defined $enc_fail, "Depth 129 fails on encode");
    is($err_enc->id, 'encode-too-deep', "Error is encode-too-deep");

    # Depth 129 fails on decode
    my $wire_129 = ("\x81" x 129) . "\x80";
    my $dec_fail = eval { decode($wire_129) };
    my $err_dec = $@;
    ok(!defined $dec_fail, "Depth 129 fails on decode");
    is($err_dec->id, 'too-deep', "Error is too-deep");

    # Tag nesting: 128 tags succeed, 129 tags fail
    my $cur_tag = CborUnsigned(1);
    for (1 .. 128) {
        $cur_tag = CborTag(0, $cur_tag);
    }
    my $enc_tag = eval { encode_checked($cur_tag) };
    is($@, '', "128 nested tags encode cleanly");
    my $dec_tag = eval { decode($enc_tag) };
    is($@, '', "128 nested tags decode cleanly");

    my $tag_129 = CborTag(0, $cur_tag);
    my $enc_tag_fail = eval { encode_checked($tag_129) };
    is($@->id, 'encode-too-deep', "129 nested tags fail encode with encode-too-deep");

    # Map key nesting: 128 nested map keys succeed, 129 nested map keys fail
    my $cur_map_key = CborUnsigned(1);
    for (1 .. 128) {
        $cur_map_key = CborMap([ CborMapEntry($cur_map_key, CborUnsigned(0)) ]);
    }
    my $enc_mk = eval { encode_checked($cur_map_key) };
    is($@, '', "128 nested map keys encode cleanly");
    my $dec_mk = eval { decode($enc_mk) };
    is($@, '', "128 nested map keys decode cleanly");
    ok($cur_map_key == $dec_mk, "128 nested map keys roundtrip identically");

    my $map_key_129 = CborMap([ CborMapEntry($cur_map_key, CborUnsigned(0)) ]);
    my $enc_mk_fail = eval { encode_checked($map_key_129) };
    ok(!defined $enc_mk_fail, "129 nested map keys fail encode");
    is($@->id, 'encode-too-deep', "129 nested map keys fail encode with encode-too-deep");

    # 129 nested map keys on wire decode
    my $bad_mk_wire = ("\xa1" x 129) . "\x01" . ("\x00" x 129);
    my $dec_mk_fail = eval { decode($bad_mk_wire) };
    ok(!defined $dec_mk_fail, "129 nested map keys fail decode");
    is($@->id, 'too-deep', "129 nested map keys fail decode with too-deep");
};

subtest 'CVE-4: Canonical length-first map key ordering & duplicate key defense' => sub {
    my $k1 = CborUnsigned(1);         # wire "01" (1 byte)
    my $k2 = CborUnsigned(24);        # wire "1818" (2 bytes)
    my $k3 = CborText("a");           # wire "6161" (2 bytes)
    my $k4 = CborText("b");           # wire "6162" (2 bytes)
    my $k5 = CborBytes("\x01\x02\x03\x04\x05"); # wire "450102030405" (6 bytes)

    # Shuffled input order
    my $map_val = CborMap([
        CborMapEntry($k5, CborNull()),
        CborMapEntry($k4, CborBool(0)),
        CborMapEntry($k1, CborUnsigned(100)),
        CborMapEntry($k3, CborBool(1)),
        CborMapEntry($k2, CborText("val")),
    ]);

    my $enc = encode_checked($map_val);
    my $dec = decode($enc);
    my $expected_canonical = CborMap([
        CborMapEntry($k1, CborUnsigned(100)),
        CborMapEntry($k2, CborText("val")),
        CborMapEntry($k3, CborBool(1)),
        CborMapEntry($k4, CborBool(0)),
        CborMapEntry($k5, CborNull()),
    ]);
    ok($expected_canonical == $dec, "Map decoded from wire is in exact canonical order");
    is(encode_checked($dec), $enc, "Decoded map re-encodes to exact same canonical wire");

    # Duplicate key in CborMap
    my $dup_map = CborMap([
        CborMapEntry(CborText("key"), CborUnsigned(1)),
        CborMapEntry(CborText("key"), CborUnsigned(2)),
    ]);
    my $res_dup = eval { encode_checked($dup_map) };
    is($@->id, 'duplicate-map-key', "Duplicate map key raises duplicate-map-key on encode");

    # Out of order keys in wire decode
    # Wire for {"b": 1, "a": 2}: 0xa2 6162 01 6161 02
    my $bad_order_wire = "\xa2\x61\x62\x01\x61\x61\x02";
    my $res_order = eval { decode($bad_order_wire) };
    is($@->id, 'non-canonical-map-order', "Out-of-order map keys raise non-canonical-map-order");

    # Duplicate keys in wire decode: 0xa2 6161 01 6161 02
    my $dup_wire = "\xa2\x61\x61\x01\x61\x61\x02";
    my $res_dup_wire = eval { decode($dup_wire) };
    is($@->id, 'non-canonical-map-order', "Duplicate map keys on wire raise non-canonical-map-order");
};

subtest 'CVE-5: Strict UTF-8 vs surrogate smuggling & wide-char byte confusion' => sub {
    # Surrogates (U+D800, U+DFFF)
    my $res1 = eval { CborText("\xED\xA0\x80") };
    is($@->id, 'invalid-utf8', "Surrogate byte sequence rejected");

    my $res2 = eval { CborText("\x{D800}") };
    is($@->id, 'invalid-utf8', "Decoded surrogate rejected");

    # Overlong NUL
    my $res3 = eval { CborText("\xC0\x80") };
    is($@->id, 'invalid-utf8', "Overlong NUL rejected");

    # Out of range codepoint (> U+10FFFF)
    my $res4 = eval { CborText("\xF4\x90\x80\x80") };
    is($@->id, 'invalid-utf8', "Out of range UTF-8 rejected");

    # Wide-character scalar passed to CborBytes
    my $res5 = eval { CborBytes("\x{100}") };
    is($@->id, 'unsupported-simple', "Wide character in CborBytes rejected");

    # CborText with non-UTF8-flagged UTF-8 octets normalizes text to decoded characters
    my $t_latin = CborText("\xC3\xA9");
    is($t_latin->text, "\x{e9}", "CborText normalizes UTF-8 bytes to decoded character string in text");
    is($t_latin->octets, "\xC3\xA9", "CborText preserves UTF-8 octets in octets");
    my $t_round = decode(encode_checked($t_latin));
    is($t_round->text, "\x{e9}", "Decoded CborText->text matches original character string");
    is($t_round->octets, "\xC3\xA9", "Decoded CborText->octets matches original UTF-8 octets");
    ok($t_latin == $t_round, "CborText equality holds across roundtrip");
};

subtest 'CVE-6: Trailing bytes and bless/tie TOCTOU forgery defense' => sub {
    # Trailing bytes on single item
    my $res1 = eval { decode("\x00\x00") };
    is($@->id, 'trailing-bytes', "Trailing bytes on integer rejected");

    my $res2 = eval { decode("\xa0\xff") };
    is($@->id, 'trailing-bytes', "Trailing bytes on empty map rejected");

    # Forged blessed reference attempting to pass as CborValue
    my $forged = bless { kind => 'unsigned', value => 42 }, 'CodingAdventures::CanonicalCbor::Types::CborValue';
    ok(!is_cbor_value($forged), "Forged object fails is_cbor_value check");
    my $enc_forged = eval { encode_checked($forged) };
    is($@->id, 'unsupported-simple', "Forged object rejected by encode_checked");

    # Untrusted object in equality comparison
    my $genuine = CborUnsigned(42);
    ok(!($genuine == $forged), "Genuine value != forged object without invoking metamethods");
    ok(!($genuine eq $forged), "Genuine value not eq forged object");
    ok(!($genuine == "not a reference"), "Genuine value != string");

    # Tied array defense
    package MyTiedArray {
        sub TIEARRAY { bless [], $_[0] }
        sub FETCH    { 1 }
        sub FETCHSIZE { 1 }
    }
    my @tied_arr;
    tie @tied_arr, 'MyTiedArray';
    my $res_arr = eval { CborArray(\@tied_arr) };
    is($@->id, 'unsupported-simple', "Tied array rejected by CborArray");

    my $res_into = eval { encode_into_checked($genuine, \@tied_arr) };
    is($@->id, 'unsupported-simple', "Tied array rejected by encode_into_checked");

    # Re-bless polymorphic spoofing blocked by SvREADONLY on referent SV
    my $rebless_res = eval { bless $genuine, 'EvilPkg'; 1 };
    ok(!$rebless_res, "Re-blessing genuine CborValue is blocked by SvREADONLY");

    # Tied scalar defense
    package MyTiedScalar {
        sub TIESCALAR { bless \my $val, $_[0] }
        sub FETCH     { "dummy" }
        sub STORE     { ${$_[0]} = $_[1] }
    }
    my $tied_scalar;
    tie $tied_scalar, 'MyTiedScalar';

    ok(!defined eval { CborUnsigned($tied_scalar) }, "CborUnsigned rejects tied scalar");
    is($@->id, 'unsupported-simple', "CborUnsigned error is unsupported-simple");

    ok(!defined eval { CborNegative($tied_scalar) }, "CborNegative rejects tied scalar");
    is($@->id, 'unsupported-simple', "CborNegative error is unsupported-simple");

    ok(!defined eval { CborBytes($tied_scalar) }, "CborBytes rejects tied scalar");
    is($@->id, 'unsupported-simple', "CborBytes error is unsupported-simple");

    ok(!defined eval { CborText($tied_scalar) }, "CborText rejects tied scalar");
    is($@->id, 'unsupported-simple', "CborText error is unsupported-simple");

    ok(!defined eval { CborBool($tied_scalar) }, "CborBool rejects tied scalar");
    is($@->id, 'unsupported-simple', "CborBool error is unsupported-simple");

    ok(!defined eval { decode($tied_scalar) }, "decode rejects tied scalar");
    is($@->id, 'unsupported-simple', "decode error is unsupported-simple");

    ok(!defined eval { decode_map_spans($tied_scalar) }, "decode_map_spans rejects tied scalar");
    is($@->id, 'unsupported-simple', "decode_map_spans error is unsupported-simple");
};

subtest 'CVE-7: Payload-blind errors and SpannedMapEntry redaction' => sub {
    my $secret_key = CborText("API_SECRET_KEY_12345");
    my $secret_val = CborBytes("super_confidential_token_payload");
    my $span = Span(10, 42);
    my $sme = SpannedMapEntry($secret_key, $secret_val, $span);

    my $str = "$sme";
    ok(index($str, "API_SECRET_KEY_12345") == -1, "Secret key is NOT present in stringification");
    ok(index($str, "super_confidential_token_payload") == -1, "Secret value is NOT present in stringification");
    like($str, qr/SpannedMapEntry\(key: <redacted>, value: <redacted>, value_span: \[10, 42\)\)/, "Contains redacted placeholders");

    # Immutability of Messages.pm globals
    my $tamper_res = eval { $CodingAdventures::CanonicalCbor::Messages::ERROR_MESSAGES{'unexpected-eof'} = 'hacked'; 1 };
    ok(!$tamper_res, "Modifying ERROR_MESSAGES hash is blocked by SvREADONLY");
    my $tamper_val_res = eval { $CodingAdventures::CanonicalCbor::Messages::REDACTED_PLACEHOLDER = 'foo'; 1 };
    ok(!$tamper_val_res, "Modifying REDACTED_PLACEHOLDER is blocked by SvREADONLY");

    # All 14 error messages are fixed strings with zero payload reflection
    for my $id (keys %CodingAdventures::CanonicalCbor::Messages::ERROR_MESSAGES) {
        my $msg = CodingAdventures::CanonicalCbor::Messages::message_for($id);
        like($msg, qr/^canonical-cbor: /, "Error message starts with canonical-cbor prefix");
        my $err = CborError($id);
        is("$err", $msg, "CborError stringifies to fixed message");
    }

    # Unknown ID error does not reflect invalid argument
    my $unknown = eval { CodingAdventures::CanonicalCbor::Messages::message_for("SECRET_INTERNAL_ID") };
    is($@, "$CodingAdventures::CanonicalCbor::Messages::UNKNOWN_ERROR_ID_MESSAGE\n", "Unknown error ID does not reflect argument");
};

# ============================================================================
# 2. Non-Minimal Integer Header Widening Across Major Types 0..6
# ============================================================================

subtest 'Non-minimal integer header widening across major types 0..6' => sub {
    # Major types: 0=uint, 1=neg, 2=bytes, 3=text, 4=array, 6=tag
    my @majors = (0, 1, 2, 3, 4, 6);

    for my $major (@majors) {
        my $lead = $major << 5;

        # 1-byte header with argument <= 23 (e.g. arg = 0)
        my $w1 = pack('C*', $lead | 24, 0);
        $w1 .= "\x00" if $major == 6; # tag needs 1 item
        my $r1 = eval { decode($w1) };
        is($@->id, 'non-minimal-integer', "Major $major: info 24 with arg 0 is non-minimal");

        # 2-byte header with argument <= 255 (e.g. arg = 1)
        my $w2 = pack('C*', $lead | 25, 0, 1);
        $w2 .= "\x00" if $major == 6;
        my $r2 = eval { decode($w2) };
        is($@->id, 'non-minimal-integer', "Major $major: info 25 with arg 1 is non-minimal");

        # 4-byte header with argument <= 65535 (e.g. arg = 1)
        my $w4 = pack('C*', $lead | 26, 0, 0, 0, 1);
        $w4 .= "\x00" if $major == 6;
        my $r4 = eval { decode($w4) };
        is($@->id, 'non-minimal-integer', "Major $major: info 26 with arg 1 is non-minimal");

        # 8-byte header with argument <= 4294967295 (e.g. arg = 1)
        my $w8 = pack('C*', $lead | 27, 0, 0, 0, 0, 0, 0, 0, 1);
        $w8 .= "\x00" if $major == 6;
        my $r8 = eval { decode($w8) };
        is($@->id, 'non-minimal-integer', "Major $major: info 27 with arg 1 is non-minimal");
    }
};

# ============================================================================
# 3. Atomic encode_into_checked Rollback
# ============================================================================

subtest 'Atomic encode_into_checked rollback' => sub {
    my $dst = [ "initial_entry" ];

    # Normal append
    encode_into_checked(CborUnsigned(42), $dst);
    is(scalar(@$dst), 2, "Destination array has 2 elements");
    is($dst->[1], "\x18\x2a", "Appended correct wire byte chunk");

    # Failure: duplicate map key
    my $dup_map = CborMap([
        CborMapEntry(CborUnsigned(1), CborText("a")),
        CborMapEntry(CborUnsigned(1), CborText("b")),
    ]);
    my $res1 = eval { encode_into_checked($dup_map, $dst) };
    is($@->id, 'duplicate-map-key', "Fails with duplicate-map-key");
    is(scalar(@$dst), 2, "Destination array unmodified after failure");
    is($dst->[0], "initial_entry", "Initial entry preserved");

    # Failure: encode-too-deep
    my $deep = CborUnsigned(1);
    for (1 .. 129) {
        $deep = CborArray([$deep]);
    }
    my $res2 = eval { encode_into_checked($deep, $dst) };
    is($@->id, 'encode-too-deep', "Fails with encode-too-deep");
    is(scalar(@$dst), 2, "Destination array unmodified after depth failure");

    # Failure: encode-too-large
    my $huge = CborBytes("\x00" x 1048572);
    my $res3 = eval { encode_into_checked($huge, $dst) };
    is($@->id, 'encode-too-large', "Fails with encode-too-large");
    is(scalar(@$dst), 2, "Destination array unmodified after size failure");

    # Invalid dst argument
    my $res4 = eval { encode_into_checked(CborNull(), "not an array") };
    is($@->id, 'unsupported-simple', "Non-array destination rejected");
};

# ============================================================================
# 4. DecodeMapSpans Wire Spans and Slice Invariant
# ============================================================================

subtest 'DecodeMapSpans wire spans and slice invariant' => sub {
    # Non-map input
    my $res_non_map = decode_map_spans("\x01");
    ok(is_decode_map_spans_result($res_non_map), "Returns DecodeMapSpansResult");
    is($res_non_map->is_map, 0, "is_map is false for integer");
    is($res_non_map->entries, undef, "entries is undef for non-map");

    # Non-map with trailing bytes
    my $res_nm_trail = eval { decode_map_spans("\x01\xff") };
    is($@->id, 'trailing-bytes', "Non-map with trailing bytes rejected");

    # Valid map: {"a": 1, "b": [true, null]}
    my $map_val = CborMap([
        CborMapEntry(CborText("a"), CborUnsigned(1)),
        CborMapEntry(CborText("b"), CborArray([ CborBool(1), CborNull() ])),
    ]);
    my $wire = encode_checked($map_val);

    my $res_map = decode_map_spans($wire);
    is($res_map->is_map, 1, "is_map is true for map");
    my $entries = $res_map->entries;
    is(scalar(@$entries), 2, "Exactly 2 spanned entries decoded");

    for my $e (@$entries) {
        ok(is_spanned_map_entry($e), "Entry is SpannedMapEntry");
        my $span = $e->value_span;
        ok(is_span($span), "value_span is Span");
        my $slice = substr($wire, $span->start, $span->length);
        my $expected_val_wire = encode_checked($e->value);
        is($slice, $expected_val_wire, "CBR01 Invariant holds: wire slice eq encode_checked(entry->value)");
    }

    # Map with trailing bytes
    my $res_m_trail = eval { decode_map_spans($wire . "\x00") };
    is($@->id, 'trailing-bytes', "Map with trailing bytes rejected");

    # Malformed map header
    my $res_indef = eval { decode_map_spans("\xbf\xff") };
    is($@->id, 'indefinite', "Indefinite map header rejected");

    my $res_nonmin = eval { decode_map_spans("\xb8\x00") };
    is($@->id, 'non-minimal-integer', "Non-minimal map header rejected");

    my $res_res = eval { decode_map_spans("\xbc") };
    is($@->id, 'reserved', "Reserved map header rejected");
};

# ============================================================================
# 5. Deterministic PRNG Property Round-Trip & Map Permutations
# ============================================================================

subtest 'Deterministic PRNG round-trip and map-permutation property tests' => sub {
    my $prng = make_prng(424242);

    sub gen_random_value {
        my ($prng, $depth) = @_;
        if ($depth >= 5) {
            return CborUnsigned($prng->{next_int}->(1000));
        }
        my $choice = $prng->{next_int}->(9);
        if ($choice == 0) {
            return CborUnsigned($prng->{next_u32}->());
        } elsif ($choice == 1) {
            return CborNegative($prng->{next_u32}->());
        } elsif ($choice == 2) {
            return CborBytes($prng->{next_bytes}->($prng->{next_int}->(16)));
        } elsif ($choice == 3) {
            my $ascii = "str_" . $prng->{next_int}->(1000);
            return CborText($ascii);
        } elsif ($choice == 4) {
            my $len = $prng->{next_int}->(4);
            my @items;
            for (1 .. $len) {
                push @items, gen_random_value($prng, $depth + 1);
            }
            return CborArray(\@items);
        } elsif ($choice == 5) {
            my $len = $prng->{next_int}->(4);
            my %seen;
            my @entries;
            for (1 .. $len) {
                my $k_num = $prng->{next_int}->(1000);
                next if $seen{$k_num}++;
                my $k = CborUnsigned($k_num);
                my $v = gen_random_value($prng, $depth + 1);
                push @entries, CborMapEntry($k, $v);
            }
            return decode(encode_checked(CborMap(\@entries)));
        } elsif ($choice == 6) {
            return CborTag($prng->{next_int}->(100), gen_random_value($prng, $depth + 1));
        } elsif ($choice == 7) {
            return CborBool($prng->{next_int}->(2));
        } else {
            return CborNull();
        }
    }

    # 500 property round-trips
    for (1 .. 500) {
        my $val = gen_random_value($prng, 0);
        my $wire = encode_checked($val);
        my $dec = decode($wire);
        ok($val == $dec, "PRNG value roundtrips with deep equality");
        my $wire2 = encode_checked($dec);
        is($wire, $wire2, "Wire bytes identical across roundtrip");
    }

    # Map permutations: 10 random shuffles of the same 5 map entries
    my @entries = (
        CborMapEntry(CborText("alpha"), CborUnsigned(1)),
        CborMapEntry(CborText("beta"), CborUnsigned(2)),
        CborMapEntry(CborText("gamma"), CborUnsigned(3)),
        CborMapEntry(CborText("delta"), CborUnsigned(4)),
        CborMapEntry(CborText("epsilon"), CborUnsigned(5)),
    );
    my $canonical_wire = encode_checked(CborMap(\@entries));

    for (1 .. 10) {
        my @shuffled = sort { $prng->{next_int}->(3) - 1 } @entries;
        my $shuffled_map = CborMap(\@shuffled);
        my $w = encode_checked($shuffled_map);
        is($w, $canonical_wire, "Map permutation $_ yields exact identical canonical wire");
    }
};

# ============================================================================
# 6. 10,000-Iteration Byte-Mutation Fuzz Loop
# ============================================================================

subtest '10,000-iteration byte-mutation fuzz loop' => sub {
    my $prng = make_prng(99999);
    for (1 .. 10000) {
        my $len = $prng->{next_int}->(64);
        my $raw = $prng->{next_bytes}->($len);

        my $res = eval { decode($raw) };
        if ($@) {
            my $err = $@;
            ok(is_cbor_error($err), "Fuzz failure is CborError");
            like($err->message, qr/^canonical-cbor: /, "Error message prefix valid");
            ok(CodingAdventures::CanonicalCbor::Messages::is_valid_id($err->id), "Error ID is valid closed CBR01 ID");
        } else {
            my $re_enc = eval { encode_checked($res) };
            if (!$@) {
                is($re_enc, $raw, "Decoded valid raw bytes re-encode identically");
                my $res2 = decode($re_enc);
                ok($res == $res2, "Re-decoded value deeply equals original");
            }
        }
    }
};

# ============================================================================
# 7. Exhaustive Coverage of Equality, Stringification, and Defensive Branches
# ============================================================================

subtest 'Exhaustive coverage of equality, stringification, and defensive branches' => sub {
    # 1. Facade exports in CanonicalCbor.pm
    my $factory = DecodeMapSpansResult();
    ok(defined $factory, "DecodeMapSpansResult factory export");
    my $fac_err = eval { raise_error('unsupported-simple') };
    is($@->id, 'unsupported-simple', "raise_error export works");
    my $entry = CborMapEntry(CborUnsigned(1), CborNull());
    ok(is_cbor_map_entry($entry), "is_cbor_map_entry export works");
    ok(!is_cbor_map_entry("not entry"), "is_cbor_map_entry returns false for string");

    # 2. CborError equality and methods
    my $err1 = CborError('too-deep');
    my $err2 = CborError('too-deep');
    my $err3 = CborError('reserved');
    ok($err1 == $err2, "CborError equality with same id");
    ok(!($err1 == $err3), "CborError inequality with different id");
    ok(!($err1 == "not error"), "CborError inequality with non-error");
    is($err1->id, 'too-deep', "Error id accessor");
    is($err1->message, CodingAdventures::CanonicalCbor::Messages::message_for('too-deep'), "Error message accessor");

    # 3. Span getters and equality
    my $s1 = Span(10, 20);
    my $s2 = Span(10, 20);
    my $s3 = Span(10, 30);
    is($s1->start, 10, "Span start");
    is($s1->finish, 20, "Span finish");
    is($s1->length, 10, "Span length");
    ok($s1 == $s2, "Span equality");
    ok(!($s1 == $s3), "Span inequality");
    ok(!($s1 == "not span"), "Span inequality with non-span");
    is("$s1", "[10, 20)", "Span stringification");

    # Span constructor error checks
    eval { Span(-1, 5) };
    is($@->id, 'unsupported-simple', "Span rejects negative start");
    eval { Span(10, 5) };
    is($@->id, 'unsupported-simple', "Span rejects start > finish");
    eval { Span("abc", 5) };
    is($@->id, 'unsupported-simple', "Span rejects non-int start");
    eval { Span(0, "abc") };
    is($@->id, 'unsupported-simple', "Span rejects non-int finish");

    # 4. SpannedMapEntry methods, stringification, equality
    my $sme1 = SpannedMapEntry(CborUnsigned(1), CborText("a"), $s1);
    my $sme2 = SpannedMapEntry(CborUnsigned(1), CborText("a"), $s2);
    my $sme3 = SpannedMapEntry(CborUnsigned(2), CborText("a"), $s1);
    ok(is_spanned_map_entry($sme1), "is_spanned_map_entry is true");
    ok(!is_spanned_map_entry("not"), "is_spanned_map_entry is false for non-entry");
    ok($sme1 == $sme2, "SpannedMapEntry equality");
    ok(!($sme1 == $sme3), "SpannedMapEntry inequality");
    ok(!($sme1 == "not"), "SpannedMapEntry inequality with non-entry");
    ok($sme1->key == CborUnsigned(1), "SpannedMapEntry key");
    ok($sme1->value == CborText("a"), "SpannedMapEntry value");
    ok($sme1->value_span == $s1, "SpannedMapEntry value_span");

    # SpannedMapEntry invalid constructor args
    eval { SpannedMapEntry("not val", CborNull(), $s1) };
    is($@->id, 'unsupported-simple', "SpannedMapEntry rejects invalid key");
    eval { SpannedMapEntry(CborNull(), "not val", $s1) };
    is($@->id, 'unsupported-simple', "SpannedMapEntry rejects invalid value");
    eval { SpannedMapEntry(CborNull(), CborNull(), "not span") };
    is($@->id, 'unsupported-simple', "SpannedMapEntry rejects invalid span");

    # 5. DecodeMapSpansResult factory, methods, equality, stringification
    my $dm_non = DecodeMapSpansResult->non_map();
    my $dm_non2 = DecodeMapSpansResult->non_map();
    my $dm_map = DecodeMapSpansResult->map([$sme1]);
    my $dm_map2 = DecodeMapSpansResult->map([$sme1]);
    my $dm_map_diff = DecodeMapSpansResult->map([$sme3]);
    ok(is_decode_map_spans_result($dm_non), "is_decode_map_spans_result non_map");
    ok(is_decode_map_spans_result($dm_map), "is_decode_map_spans_result map");
    ok(!is_decode_map_spans_result("foo"), "is_decode_map_spans_result false");
    is($dm_non->is_map, 0, "non_map is_map is 0");
    is($dm_non->entries, undef, "non_map entries is undef");
    is($dm_map->is_map, 1, "map is_map is 1");
    is(scalar(@{$dm_map->entries}), 1, "map entries count");
    ok($dm_non == $dm_non2, "non_map equals non_map");
    ok($dm_map == $dm_map2, "map equals map");
    ok(!($dm_map == $dm_non), "map != non_map");
    ok(!($dm_map == $dm_map_diff), "map != different map");
    ok(!($dm_map == "not res"), "map != non-result");
    is("$dm_non", "DecodeMapSpansResult.non_map", "non_map stringification");
    is("$dm_map", "DecodeMapSpansResult.map(#1)", "map stringification");

    # DecodeMapSpansResultFactory invalid args
    eval { DecodeMapSpansResult->map("not array") };
    is($@->id, 'unsupported-simple', "map factory rejects non-array");
    eval { DecodeMapSpansResult->map(["not sme"]) };
    is($@->id, 'unsupported-simple', "map factory rejects non-sme item");

    # 6. CborValue stringification for all kinds
    is("" . CborUnsigned(42), "CborUnsigned(42)", "CborUnsigned stringification");
    is("" . CborNegative(10), "CborNegative(10)", "CborNegative stringification");
    is("" . CborBytes("xyz"), "CborBytes(#3)", "CborBytes stringification");
    is("" . CborText("hello"), "CborText(\"hello\")", "CborText stringification");
    is("" . CborArray([CborNull()]), "CborArray(#1)", "CborArray stringification");
    is("" . CborMap([$entry]), "CborMap(#1)", "CborMap stringification");
    is("" . CborTag(5, CborNull()), "CborTag(5, CborNull)", "CborTag stringification");
    is("" . CborBool(1), "CborBool(true)", "CborBool(1) stringification");
    is("" . CborBool(0), "CborBool(false)", "CborBool(0) stringification");
    is("" . CborNull(), "CborNull", "CborNull stringification");
    is(CborNull()->items, undef, "CborNull->items is undef");
    is(CborNull()->entries, undef, "CborNull->entries is undef");

    # 7. CborValue equality across kinds and types
    my $v_b1 = CborBytes("abc");
    my $v_b2 = CborBytes("abc");
    my $v_b3 = CborBytes("xyz");
    ok($v_b1 == $v_b2, "CborBytes equal");
    ok(!($v_b1 == $v_b3), "CborBytes not equal");

    my $v_t1 = CborText("abc");
    my $v_t2 = CborText("abc");
    my $v_t3 = CborText("xyz");
    ok($v_t1 == $v_t2, "CborText equal");
    ok(!($v_t1 == $v_t3), "CborText not equal");

    my $v_bool1 = CborBool(1);
    my $v_bool2 = CborBool(1);
    my $v_bool0 = CborBool(0);
    ok($v_bool1 == $v_bool2, "CborBool equal");
    ok(!($v_bool1 == $v_bool0), "CborBool not equal");

    my $v_null1 = CborNull();
    my $v_null2 = CborNull();
    ok($v_null1 == $v_null2, "CborNull equal");

    my $v_tag1 = CborTag(1, CborNull());
    my $v_tag2 = CborTag(1, CborNull());
    my $v_tag3 = CborTag(2, CborNull());
    ok($v_tag1 == $v_tag2, "CborTag equal");
    ok(!($v_tag1 == $v_tag3), "CborTag not equal");

    my $v_arr1 = CborArray([CborUnsigned(1)]);
    my $v_arr2 = CborArray([CborUnsigned(1), CborUnsigned(2)]);
    ok(!($v_arr1 == $v_arr2), "CborArray different lengths not equal");

    my $v_map1 = CborMap([$entry]);
    my $v_map2 = CborMap([$entry, CborMapEntry(CborUnsigned(2), CborNull())]);
    ok(!($v_map1 == $v_map2), "CborMap different counts not equal");

    ok(!($v_b1 == $v_t1), "CborBytes != CborText");
    ok(!($v_b1 == "not cbor"), "CborValue != string");

    # 8. CborMapEntry stringification and equality
    my $me1 = CborMapEntry(CborUnsigned(1), CborText("v"));
    my $me2 = CborMapEntry(CborUnsigned(1), CborText("v"));
    my $me3 = CborMapEntry(CborUnsigned(2), CborText("v"));
    is("$me1", "CborMapEntry(CborUnsigned(1) => CborText(\"v\"))", "CborMapEntry stringification");
    ok($me1 == $me2, "CborMapEntry equal");
    ok(!($me1 == $me3), "CborMapEntry not equal");
    ok(!($me1 == "not entry"), "CborMapEntry not equal non-entry");

    # CborMapEntry invalid constructor
    eval { CborMapEntry("not val", CborNull()) };
    is($@->id, 'unsupported-simple', "CborMapEntry rejects non-cbor key");
    eval { CborMapEntry(CborNull(), "not val") };
    is($@->id, 'unsupported-simple', "CborMapEntry rejects non-cbor value");

    # 9. Type constructor error branches
    eval { CborBytes(undef) };
    is($@->id, 'unsupported-simple', "CborBytes rejects undef");
    eval { CborBytes([]) };
    is($@->id, 'unsupported-simple', "CborBytes rejects ref");

    eval { CborText(undef) };
    is($@->id, 'unsupported-simple', "CborText rejects undef");
    eval { CborText([]) };
    is($@->id, 'unsupported-simple', "CborText rejects ref");

    eval { CborArray("not array") };
    is($@->id, 'unsupported-simple', "CborArray rejects non-array");
    eval { CborArray([ "not cbor" ]) };
    is($@->id, 'unsupported-simple', "CborArray rejects array with non-cbor");

    eval { CborMap("not array") };
    is($@->id, 'unsupported-simple', "CborMap rejects non-array");
    eval { CborMap([ "not entry" ]) };
    is($@->id, 'unsupported-simple', "CborMap rejects array with non-entry");

    eval { CborTag(-1, CborNull()) };
    is($@->id, 'unsupported-simple', "CborTag rejects negative tag");
    eval { CborTag(1, "not cbor") };
    is($@->id, 'unsupported-simple', "CborTag rejects non-cbor value");

    eval { CborBool(undef) };
    is($@->id, 'unsupported-simple', "CborBool rejects undef");
    eval { CborBool([]) };
    is($@->id, 'unsupported-simple', "CborBool rejects ref");

    # 10. Natural buffer overflow in _push_byte (encode-too-large)
    my $payload = "\x00" x (CodingAdventures::CanonicalCbor::Constants::MAX_ENCODED_BYTES - 6);
    my $overflow_val = CborArray([
        CborBytes($payload),
        CborNull(),
    ]);
    my $res_overflow = eval { encode_checked($overflow_val) };
    is($@->id, 'encode-too-large', "Triggers natural buffer overflow via push_byte");

    # 11. Encoding headers with 1-byte, 2-byte, 4-byte lengths
    my $txt_30 = CborText("a" x 30);
    is(length(encode_checked($txt_30)), 32, "Text with 1-byte length header encodes cleanly");

    my $txt_300 = CborText("a" x 300);
    is(length(encode_checked($txt_300)), 303, "Text with 2-byte length header encodes cleanly");

    my $txt_70k = CborText("a" x 70000);
    is(length(encode_checked($txt_70k)), 70005, "Text with 4-byte length header encodes cleanly");

    # 12. Non-minimal simple values and minimal argument checks
    eval { decode("\xf8\x14") };
    is($@->id, 'non-minimal-integer', "Simple value info 24 with arg 20 is non-minimal");

    eval { decode("\x19\x00\x17") };
    is($@->id, 'non-minimal-integer', "Major 0: 2-byte header with arg <= 23 is non-minimal");

    eval { decode("\x1a\x00\x00\x00\xff") };
    is($@->id, 'non-minimal-integer', "Major 0: 4-byte header with arg <= 255 is non-minimal");

    eval { decode("\x1b\x00\x00\x00\x00\x00\x00\xff\xff") };
    is($@->id, 'non-minimal-integer', "Major 0: 8-byte header with arg <= 65535 is non-minimal");

    # 13. Simple values decode
    ok(decode("\xf4") == CborBool(0), "Decode false (0xf4)");
    ok(decode("\xf5") == CborBool(1), "Decode true (0xf5)");
    ok(decode("\xf6") == CborNull(), "Decode null (0xf6)");

    eval { decode("\xe0") };
    is($@->id, 'unsupported-simple', "Reserved simple value 0 rejected");

    eval { decode("\xf7") };
    is($@->id, 'unsupported-simple', "Undefined simple value 23 rejected");

    eval { decode("\xf8\x20") };
    is($@->id, 'unsupported-simple', "Unassigned simple value 32 rejected");
};

done_testing;
