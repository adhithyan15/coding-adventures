use strict;
use warnings;
use Test2::V0;

# Load the module under test and its transitive dependencies.
# The -l flag on prove adds ../lib to @INC, but sibling packages need
# explicit paths. These paths are relative to the package root so they
# work both under "prove -l" from the package dir and from the build tool.
use lib 'lib';
use lib '../hmac/lib';
use lib '../sha256/lib';
use lib '../sha512/lib';
use lib '../sha1/lib';
use lib '../md5/lib';

use CodingAdventures::Scrypt qw(scrypt scrypt_hex);

# ──────────────────────────────────────────────────────────────────────────────
# RFC 7914 § 11 Test Vectors
# ──────────────────────────────────────────────────────────────────────────────
#
# NOTE on expected values: The expected hex strings below were verified against
# Python 3's hashlib.scrypt (which uses OpenSSL's scrypt implementation).
# The RFC 7914 Appendix B also contains a third vector (N=16384, r=8, p=1)
# which is not included here for performance reasons in CI.
#
# Python verification:
#   import hashlib
#   hashlib.scrypt(b'', salt=b'', n=16, r=1, p=1, dklen=64).hex()
#   => "77d6576238657b203b19ca42c18a0497f16b4844..."
#   hashlib.scrypt(b'password', salt=b'NaCl', n=1024, r=8, p=16, dklen=64).hex()
#   => "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cb..."

# Vector 1: password="", salt="", N=16, r=1, p=1, dkLen=64.
#
# This vector is specifically designed to test that scrypt handles empty
# passwords correctly. Our PBKDF2 package rejects empty passwords, so scrypt
# uses its own inline PBKDF2 implementation.
is(
    scrypt_hex("", "", 16, 1, 1, 64),
    "77d6576238657b203b19ca42c18a0497" .
    "f16b4844e3074ae8dfdffa3fede21442" .
    "fcd0069ded0948f8326a753a0fc81f17" .
    "e8d3e0fb2e0d3628cf35e20c38d18906",
    "RFC 7914 vector 1: empty password and salt, N=16, r=1, p=1"
);

# Vector 2: password="password", salt="NaCl", N=1024, r=8, p=16, dkLen=64.
#
# This vector exercises the parallelism parameter (p=16) and a larger N.
# It is the most commonly cited scrypt test vector and validates the
# full BlockMix/ROMix pipeline with realistic parameters.
is(
    scrypt_hex("password", "NaCl", 1024, 8, 16, 64),
    "fdbabe1c9d3472007856e7190d01e9fe" .
    "7c6ad7cbc8237830e77376634b373162" .
    "2eaf30d92e22a3886ff109279d9830da" .
    "c727afb94a83ee6d8360cbdfa2cc0640",
    "RFC 7914 vector 2: password/NaCl, N=1024, r=8, p=16"
);

# ──────────────────────────────────────────────────────────────────────────────
# scrypt vs scrypt_hex consistency
# ──────────────────────────────────────────────────────────────────────────────

# The raw and hex variants must agree.
{
    my $raw = scrypt("password", "salt", 16, 1, 1, 32);
    my $hex = scrypt_hex("password", "salt", 16, 1, 1, 32);
    my $expected = join('', map { sprintf('%02x', ord($_)) } split(//, $raw));
    is($hex, $expected, "scrypt_hex matches manual hex encoding of scrypt");
}

# ──────────────────────────────────────────────────────────────────────────────
# Output length
# ──────────────────────────────────────────────────────────────────────────────

# dk_len is respected exactly.
is(length(scrypt("key", "salt", 16, 1, 1, 32)),  32, "output length 32");
is(length(scrypt("key", "salt", 16, 1, 1, 16)),  16, "output length 16");
is(length(scrypt("key", "salt", 16, 1, 1, 64)),  64, "output length 64");
is(length(scrypt("key", "salt", 16, 1, 1,  1)),   1, "output length 1");

# A 64-byte result must start with the 32-byte result (PBKDF2 truncation).
{
    my $dk64 = scrypt("password", "salt", 16, 1, 1, 64);
    my $dk32 = scrypt("password", "salt", 16, 1, 1, 32);
    is(substr($dk64, 0, 32), $dk32, "longer output starts with shorter output");
}

# ──────────────────────────────────────────────────────────────────────────────
# Determinism
# ──────────────────────────────────────────────────────────────────────────────

# Same inputs must always yield the same output (no randomness).
{
    my $a = scrypt("secret", "nacl", 16, 1, 1, 32);
    my $b = scrypt("secret", "nacl", 16, 1, 1, 32);
    is($a, $b, "deterministic: same inputs produce same output");
}

# ──────────────────────────────────────────────────────────────────────────────
# Sensitivity to inputs
# ──────────────────────────────────────────────────────────────────────────────

# Different passwords produce different keys.
{
    my $a = scrypt("password1", "salt", 16, 1, 1, 32);
    my $b = scrypt("password2", "salt", 16, 1, 1, 32);
    isnt($a, $b, "different passwords produce different keys");
}

# Different salts produce different keys.
{
    my $a = scrypt("password", "salt1", 16, 1, 1, 32);
    my $b = scrypt("password", "salt2", 16, 1, 1, 32);
    isnt($a, $b, "different salts produce different keys");
}

# Different N values produce different keys.
{
    my $a = scrypt("password", "salt", 16, 1, 1, 32);
    my $b = scrypt("password", "salt", 32, 1, 1, 32);
    isnt($a, $b, "different N produce different keys");
}

# ──────────────────────────────────────────────────────────────────────────────
# Empty password allowed (RFC 7914 explicitly uses it)
# ──────────────────────────────────────────────────────────────────────────────

{
    my $dk = scrypt("", "salt", 16, 1, 1, 32);
    is(length($dk), 32, "empty password allowed, returns 32 bytes");
}

# Empty salt also allowed.
{
    my $dk = scrypt("password", "", 16, 1, 1, 32);
    is(length($dk), 32, "empty salt allowed, returns 32 bytes");
}

# ──────────────────────────────────────────────────────────────────────────────
# Parameter validation
# ──────────────────────────────────────────────────────────────────────────────

# N must be a power of 2.
ok(
    do { local $@; eval { scrypt("p", "s", 3, 1, 1, 32) }; $@ =~ /power of 2/ },
    "N=3 (not power of 2) dies"
);

ok(
    do { local $@; eval { scrypt("p", "s", 1, 1, 1, 32) }; $@ =~ /power of 2/ },
    "N=1 dies (must be >= 2)"
);

# N too large.
ok(
    do { local $@; eval { scrypt("p", "s", 2**21, 1, 1, 32) }; $@ =~ /2\^20/ },
    "N=2^21 exceeds limit"
);

# r must be >= 1.
ok(
    do { local $@; eval { scrypt("p", "s", 16, 0, 1, 32) }; $@ =~ /r must be a positive integer/ },
    "r=0 dies"
);

# p must be >= 1.
ok(
    do { local $@; eval { scrypt("p", "s", 16, 1, 0, 32) }; $@ =~ /p must be a positive integer/ },
    "p=0 dies"
);

# dk_len must be >= 1.
ok(
    do { local $@; eval { scrypt("p", "s", 16, 1, 1, 0) }; $@ =~ /dk_len must be between/ },
    "dk_len=0 dies"
);

# dk_len must not exceed 2^20.
ok(
    do { local $@; eval { scrypt("p", "s", 16, 1, 1, 2**20 + 1) }; $@ =~ /dk_len must be between/ },
    "dk_len > 2^20 dies"
);

done_testing;
