use strict;
use warnings;
use Test2::V0;

use lib 'lib';
use lib '../hmac/lib';
use lib '../sha256/lib';
use lib '../sha512/lib';
use lib '../sha1/lib';
use lib '../md5/lib';

use CodingAdventures::PBKDF2 qw(
    pbkdf2_hmac_sha1   pbkdf2_hmac_sha1_hex
    pbkdf2_hmac_sha256 pbkdf2_hmac_sha256_hex
    pbkdf2_hmac_sha512 pbkdf2_hmac_sha512_hex
);

# Helper: decode hex to binary string.
sub from_hex {
    my ($s) = @_;
    $s =~ s/([0-9a-f]{2})/chr(hex($1))/ge;
    return $s;
}

# ──────────────────────────────────────────────────────────────────────────────
# RFC 6070 — PBKDF2-HMAC-SHA1
# ──────────────────────────────────────────────────────────────────────────────

is(
    pbkdf2_hmac_sha1_hex("password", "salt", 1, 20),
    "0c60c80f961f0e71f3a9b524af6012062fe037a6",
    "RFC 6070 vector 1 — c=1"
);

is(
    pbkdf2_hmac_sha1_hex("password", "salt", 4096, 20),
    "4b007901b765489abead49d926f721d065a429c1",
    "RFC 6070 vector 2 — c=4096"
);

is(
    pbkdf2_hmac_sha1_hex(
        "passwordPASSWORDpassword",
        "saltSALTsaltSALTsaltSALTsaltSALTsalt",
        4096,
        25
    ),
    "3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038",
    "RFC 6070 vector 3 — long password and salt"
);

is(
    pbkdf2_hmac_sha1_hex("pass\x00word", "sa\x00lt", 4096, 16),
    "56fa6aa75548099dcc37d7f03425e0c3",
    "RFC 6070 vector 4 — null bytes"
);

# ──────────────────────────────────────────────────────────────────────────────
# RFC 7914 — PBKDF2-HMAC-SHA256
# ──────────────────────────────────────────────────────────────────────────────

is(
    pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 64),
    "55ac046e56e3089fec1691c22544b605" .
    "f94185216dde0465e68b9d57c20dacbc" .
    "49ca9cccf179b645991664b39d77ef31" .
    "7c71b845b1e30bd509112041d3a19783",
    "RFC 7914 Appendix B — c=1, dkLen=64"
);

is(length(pbkdf2_hmac_sha256("key", "salt", 1, 32)), 32, "SHA256 output length 32");

{
    my $short = pbkdf2_hmac_sha256("key", "salt", 1, 16);
    my $full  = pbkdf2_hmac_sha256("key", "salt", 1, 32);
    is($short, substr($full, 0, 16), "SHA256 truncation consistent");
}

{
    my $dk64 = pbkdf2_hmac_sha256("password", "salt", 1, 64);
    my $dk32 = pbkdf2_hmac_sha256("password", "salt", 1, 32);
    is(length($dk64), 64, "SHA256 multi-block length");
    is(substr($dk64, 0, 32), $dk32, "SHA256 multi-block block1 matches single");
}

# ──────────────────────────────────────────────────────────────────────────────
# SHA-512 sanity checks
# ──────────────────────────────────────────────────────────────────────────────

is(length(pbkdf2_hmac_sha512("secret", "nacl", 1, 64)), 64, "SHA512 output length");

{
    my $short = pbkdf2_hmac_sha512("secret", "nacl", 1, 32);
    my $full  = pbkdf2_hmac_sha512("secret", "nacl", 1, 64);
    is($short, substr($full, 0, 32), "SHA512 truncation");
}

is(length(pbkdf2_hmac_sha512("key", "salt", 1, 128)), 128, "SHA512 multi-block 128 bytes");

# ──────────────────────────────────────────────────────────────────────────────
# Hex variants
# ──────────────────────────────────────────────────────────────────────────────

is(
    pbkdf2_hmac_sha1_hex("password", "salt", 1, 20),
    "0c60c80f961f0e71f3a9b524af6012062fe037a6",
    "SHA1 hex matches RFC 6070 vector 1"
);

{
    my $raw = pbkdf2_hmac_sha256("passwd", "salt", 1, 32);
    my $hex = pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 32);
    my $expected = join('', map { sprintf('%02x', ord($_)) } split(//, $raw));
    is($hex, $expected, "SHA256 hex matches bytes");
}

{
    my $raw = pbkdf2_hmac_sha512("secret", "nacl", 1, 64);
    my $hex = pbkdf2_hmac_sha512_hex("secret", "nacl", 1, 64);
    my $expected = join('', map { sprintf('%02x', ord($_)) } split(//, $raw));
    is($hex, $expected, "SHA512 hex matches bytes");
}

# ──────────────────────────────────────────────────────────────────────────────
# Validation
# ──────────────────────────────────────────────────────────────────────────────

ok(
    do { local $@; eval { pbkdf2_hmac_sha256("", "salt", 1, 32) }; $@ =~ /password must not be empty/ },
    "empty password SHA256 dies"
);

ok(
    do { local $@; eval { pbkdf2_hmac_sha1("", "salt", 1, 20) }; $@ =~ /password must not be empty/ },
    "empty password SHA1 dies"
);

ok(
    do { local $@; eval { pbkdf2_hmac_sha256("pw", "salt", 0, 32) }; $@ =~ /iterations must be a positive integer/ },
    "zero iterations dies"
);

ok(
    do { local $@; eval { pbkdf2_hmac_sha256("pw", "salt", 1, 0) }; $@ =~ /key_length must be a positive integer/ },
    "zero key_length dies"
);

is(length(pbkdf2_hmac_sha256("password", "", 1, 32)), 32, "empty salt allowed");

{
    my $a = pbkdf2_hmac_sha256("secret", "nacl", 100, 32);
    my $b = pbkdf2_hmac_sha256("secret", "nacl", 100, 32);
    is($a, $b, "deterministic");
}

{
    my $a = pbkdf2_hmac_sha256("password", "salt1", 1, 32);
    my $b = pbkdf2_hmac_sha256("password", "salt2", 1, 32);
    isnt($a, $b, "different salts produce different keys");
}

{
    my $a = pbkdf2_hmac_sha256("password1", "salt", 1, 32);
    my $b = pbkdf2_hmac_sha256("password2", "salt", 1, 32);
    isnt($a, $b, "different passwords produce different keys");
}

{
    my $a = pbkdf2_hmac_sha256("password", "salt", 1, 32);
    my $b = pbkdf2_hmac_sha256("password", "salt", 2, 32);
    isnt($a, $b, "different iterations produce different keys");
}

done_testing;
