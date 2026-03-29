use strict;
use warnings;
use Test2::V0;

use CodingAdventures::UUID qw(
    generate_v1  generate_v3  generate_v4  generate_v5  generate_v7
    parse  validate  nil_uuid
);

# ============================================================================
# Helpers
# ============================================================================

# uuid_version_char($u) — extract the version digit at canonical position 15
# In "xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx", M is the 15th character (1-indexed).
sub uuid_version_char {
    return substr($_[0], 14, 1);
}

# uuid_variant_ok($u) — check RFC 4122 variant bits
# N (char at position 20) must be 8, 9, a, or b.
sub uuid_variant_ok {
    my $c = lc(substr($_[0], 19, 1));
    return $c eq '8' || $c eq '9' || $c eq 'a' || $c eq 'b';
}

# ============================================================================
# nil_uuid
# ============================================================================

subtest 'nil_uuid' => sub {
    is nil_uuid(), "00000000-0000-0000-0000-000000000000",
        'nil_uuid returns all-zeros UUID';
    ok validate(nil_uuid()), 'nil_uuid passes validate()';
};

# ============================================================================
# validate
# ============================================================================

subtest 'validate' => sub {
    ok validate("550e8400-e29b-41d4-a716-446655440000"),
        'accepts lowercase valid UUID';
    ok validate("550E8400-E29B-41D4-A716-446655440000"),
        'accepts uppercase valid UUID';
    ok validate("00000000-0000-0000-0000-000000000000"),
        'accepts nil UUID';

    ok !validate("550e8400-e29b-41d4-a716-44665544000"),   # 11 chars in last group
        'rejects UUID that is too short';
    ok !validate("550e8400e29b-41d4-a716-446655440000"),   # wrong dash positions
        'rejects UUID with wrong dashes';
    ok !validate("550e8400-e29b-41d4-a716-44665544000g"),  # 'g' not hex
        'rejects UUID with invalid hex char';
    ok !validate(undef),  'rejects undef';
    ok !validate(42),     'rejects non-string';
    ok !validate(""),     'rejects empty string';
};

# ============================================================================
# parse
# ============================================================================

subtest 'parse' => sub {
    # Invalid input
    my ($result, $err) = parse("not-a-uuid");
    ok !defined $result, 'parse returns undef for invalid UUID';
    ok defined $err,     'parse returns error string for invalid UUID';

    # v4 UUID
    my $u4   = generate_v4();
    my $info = parse($u4);
    is $info->{version}, 4,          'parse: version = 4';
    is $info->{variant}, "rfc4122",  'parse: variant = rfc4122';
    ok ref($info->{bytes}) eq 'ARRAY', 'parse: bytes is arrayref';
    ok scalar(@{$info->{bytes}}) == 16, 'parse: bytes has 16 elements';

    # All bytes in range
    for my $b (@{$info->{bytes}}) {
        ok $b >= 0 && $b <= 255, "byte $b in [0,255]";
    }

    # v5 test vector
    my $u5 = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    my $i5 = parse($u5);
    is $i5->{version}, 5,         'parse v5: version = 5';
    is $i5->{variant}, "rfc4122", 'parse v5: variant = rfc4122';

    # Nil UUID: version 0, NCS variant
    my $nil_info = parse(nil_uuid());
    is $nil_info->{version}, 0,     'parse nil: version = 0';
    is $nil_info->{variant}, "ncs", 'parse nil: variant = ncs';
};

# ============================================================================
# generate_v4
# ============================================================================

subtest 'generate_v4' => sub {
    my $u = generate_v4();
    ok validate($u),            'v4 UUID passes validate()';
    is uuid_version_char($u), '4', 'v4 has version digit 4';
    ok uuid_variant_ok($u),     'v4 has RFC 4122 variant bits';

    # Uniqueness: generate 20 and check no duplicates
    my %seen;
    for (1..20) {
        my $uu = generate_v4();
        ok !$seen{$uu}, "v4 UUID is unique: $uu";
        $seen{$uu} = 1;
    }
};

# ============================================================================
# generate_v1
# ============================================================================

subtest 'generate_v1' => sub {
    my $u = generate_v1();
    ok validate($u),            'v1 UUID passes validate()';
    is uuid_version_char($u), '1', 'v1 has version digit 1';
    ok uuid_variant_ok($u),     'v1 has RFC 4122 variant bits';

    my %seen;
    for (1..10) {
        my $uu = generate_v1();
        ok !$seen{$uu}, "v1 UUID is unique: $uu";
        $seen{$uu} = 1;
    }
};

# ============================================================================
# generate_v3 — RFC 4122 test vectors (MUST match exactly)
# ============================================================================

subtest 'generate_v3' => sub {
    # THE AUTHORITATIVE TEST VECTOR from RFC 4122 Appendix B
    my $result = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    is $result, "5df41881-3aed-3515-88a7-2f4a814cf09e",
        'v3(NAMESPACE_DNS, "www.example.com") matches RFC 4122 test vector';

    # Format and version checks
    ok validate($result),            'v3 result passes validate()';
    is uuid_version_char($result), '3', 'v3 has version digit 3';
    ok uuid_variant_ok($result),     'v3 has RFC 4122 variant bits';

    # Determinism
    my $u1 = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "test.example.org");
    my $u2 = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "test.example.org");
    is $u1, $u2, 'v3 is deterministic: same inputs → same output';

    # Different names → different UUIDs
    my $ua = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "foo.example.com");
    my $ub = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "bar.example.com");
    isnt $ua, $ub, 'v3 different names → different UUIDs';

    # Different namespaces → different UUIDs
    my $uc = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "example.com");
    my $ud = generate_v3($CodingAdventures::UUID::NAMESPACE_URL, "example.com");
    isnt $uc, $ud, 'v3 different namespaces → different UUIDs';

    # Empty name
    my $empty = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "");
    ok validate($empty), 'v3 handles empty name';

    # Invalid namespace
    my ($bad, $err) = generate_v3("not-a-uuid", "name");
    ok !defined $bad, 'v3 returns undef for invalid namespace';
    ok defined $err,  'v3 returns error for invalid namespace';
};

# ============================================================================
# generate_v5 — RFC 4122 test vectors (MUST match exactly)
# ============================================================================

subtest 'generate_v5' => sub {
    # THE AUTHORITATIVE TEST VECTOR, verified against Python's uuid.uuid5()
    # reference implementation (RFC 4122 §Appendix B).
    my $result = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    is $result, "2ed6657d-e927-568b-95e1-2665a8aea6a2",
        'v5(NAMESPACE_DNS, "www.example.com") matches RFC 4122 test vector';

    # Format and version checks
    ok validate($result),            'v5 result passes validate()';
    is uuid_version_char($result), '5', 'v5 has version digit 5';
    ok uuid_variant_ok($result),     'v5 has RFC 4122 variant bits';

    # Determinism
    my $u1 = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "test.example.org");
    my $u2 = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "test.example.org");
    is $u1, $u2, 'v5 is deterministic';

    # Different names → different UUIDs
    my $ua = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "foo.example.com");
    my $ub = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "bar.example.com");
    isnt $ua, $ub, 'v5 different names → different UUIDs';

    # Different namespaces → different UUIDs
    my $uc = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "example.com");
    my $ud = generate_v5($CodingAdventures::UUID::NAMESPACE_URL, "example.com");
    isnt $uc, $ud, 'v5 different namespaces → different UUIDs';

    # v5 differs from v3 for same inputs
    my $v3 = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    my $v5 = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
    isnt $v3, $v5, 'v5 differs from v3 for same inputs';

    # Invalid namespace
    my ($bad, $err) = generate_v5("bad-uuid", "name");
    ok !defined $bad, 'v5 returns undef for invalid namespace';
    ok defined $err,  'v5 returns error for invalid namespace';
};

# ============================================================================
# generate_v7
# ============================================================================

subtest 'generate_v7' => sub {
    my $u = generate_v7();
    ok validate($u),            'v7 UUID passes validate()';
    is uuid_version_char($u), '7', 'v7 has version digit 7';
    ok uuid_variant_ok($u),     'v7 has RFC 4122 variant bits';

    # Uniqueness
    my %seen;
    for (1..10) {
        my $uu = generate_v7();
        ok !$seen{$uu}, "v7 UUID is unique: $uu";
        $seen{$uu} = 1;
    }

    # Sanity: the timestamp in the high bytes should be within ±1 day of now
    my $hex_ts = substr($u, 0, 8) . substr($u, 9, 4);  # 12 hex chars = 48 bits
    my $ms     = hex($hex_ts);
    my $now    = time() * 1000;
    my $day_ms = 86400 * 1000;
    ok abs($ms - $now) < $day_ms,
        'v7 timestamp is within ±1 day of current time';
};

# ============================================================================
# Namespace constants
# ============================================================================

subtest 'namespace_constants' => sub {
    is $CodingAdventures::UUID::NAMESPACE_DNS,
       "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
       'NAMESPACE_DNS has correct well-known value';
    is $CodingAdventures::UUID::NAMESPACE_URL,
       "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
       'NAMESPACE_URL has correct well-known value';

    ok validate($CodingAdventures::UUID::NAMESPACE_DNS),  'NAMESPACE_DNS validates';
    ok validate($CodingAdventures::UUID::NAMESPACE_URL),  'NAMESPACE_URL validates';
    ok validate($CodingAdventures::UUID::NAMESPACE_OID),  'NAMESPACE_OID validates';
    ok validate($CodingAdventures::UUID::NAMESPACE_X500), 'NAMESPACE_X500 validates';
};

done_testing();
