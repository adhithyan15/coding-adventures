use strict;
use warnings;
use Test2::V0;
use lib '../lib';
use lib '../sha256/lib';
use lib '../sha512/lib';
use lib '../md5/lib';
use lib '../sha1/lib';

use CodingAdventures::HMAC qw(
    hmac_md5     hmac_md5_hex
    hmac_sha1    hmac_sha1_hex
    hmac_sha256  hmac_sha256_hex
    hmac_sha512  hmac_sha512_hex
);

# ─── RFC 4231 — HMAC-SHA256 ───────────────────────────────────────────────────

subtest 'HMAC-SHA256 (RFC 4231)' => sub {
    is(
        hmac_sha256_hex("\x0b" x 20, "Hi There"),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        "TC1: 20-byte key, 'Hi There'"
    );

    is(
        hmac_sha256_hex("Jefe", "what do ya want for nothing?"),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        "TC2: 'Jefe'"
    );

    is(
        hmac_sha256_hex("\xaa" x 20, "\xdd" x 50),
        "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
        "TC3: 0xaa key, 0xdd data"
    );

    is(
        hmac_sha256_hex("\xaa" x 131, "Test Using Larger Than Block-Size Key - Hash Key First"),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
        "TC6: 131-byte key"
    );

    is(
        hmac_sha256_hex(
            "\xaa" x 131,
            "This is a test using a larger than block-size key and a larger than block-size data. " .
            "The key needs to be hashed before being used by the HMAC algorithm."
        ),
        "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
        "TC7: 131-byte key, large data"
    );
};

# ─── RFC 4231 — HMAC-SHA512 ───────────────────────────────────────────────────

subtest 'HMAC-SHA512 (RFC 4231)' => sub {
    is(
        hmac_sha512_hex("\x0b" x 20, "Hi There"),
        "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
        "TC1: 20-byte key"
    );

    is(
        hmac_sha512_hex("Jefe", "what do ya want for nothing?"),
        "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
        "TC2: 'Jefe'"
    );

    is(
        hmac_sha512_hex("\xaa" x 131, "Test Using Larger Than Block-Size Key - Hash Key First"),
        "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598",
        "TC6: 131-byte key"
    );
};

# ─── RFC 2202 — HMAC-MD5 ─────────────────────────────────────────────────────

subtest 'HMAC-MD5 (RFC 2202)' => sub {
    is(hmac_md5_hex("\x0b" x 16, "Hi There"), "9294727a3638bb1c13f48ef8158bfc9d", "TC1");
    is(hmac_md5_hex("Jefe", "what do ya want for nothing?"), "750c783e6ab0b503eaa86e310a5db738", "TC2");
    is(
        hmac_md5_hex("\xaa" x 80, "Test Using Larger Than Block-Size Key - Hash Key First"),
        "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd",
        "TC6"
    );
};

# ─── RFC 2202 — HMAC-SHA1 ────────────────────────────────────────────────────

subtest 'HMAC-SHA1 (RFC 2202)' => sub {
    is(hmac_sha1_hex("\x0b" x 20, "Hi There"), "b617318655057264e28bc0b6fb378c8ef146be00", "TC1");
    is(
        hmac_sha1_hex("Jefe", "what do ya want for nothing?"),
        "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79",
        "TC2"
    );
    is(
        hmac_sha1_hex("\xaa" x 80, "Test Using Larger Than Block-Size Key - Hash Key First"),
        "aa4ae5e15272d00e95705637ce8a3b55ed402112",
        "TC6"
    );
};

# ─── Return lengths ───────────────────────────────────────────────────────────

subtest 'Return lengths' => sub {
    is(scalar @{ hmac_md5("k", "m") },    16, "HMAC-MD5 → 16 bytes");
    is(scalar @{ hmac_sha1("k", "m") },   20, "HMAC-SHA1 → 20 bytes");
    is(scalar @{ hmac_sha256("k", "m") }, 32, "HMAC-SHA256 → 32 bytes");
    is(scalar @{ hmac_sha512("k", "m") }, 64, "HMAC-SHA512 → 64 bytes");
};

# ─── Key handling ─────────────────────────────────────────────────────────────

subtest 'Key handling' => sub {
    is(scalar @{ hmac_sha256("", "") }, 32, "empty key and message SHA-256");
    is(scalar @{ hmac_sha512("", "") }, 64, "empty key and message SHA-512");

    my $k65 = "\x01" x 65;
    my $k66 = "\x01" x 66;
    isnt(hmac_sha256_hex($k65, "msg"), hmac_sha256_hex($k66, "msg"), "different long keys differ");
};

# ─── Authentication properties ────────────────────────────────────────────────

subtest 'Authentication properties' => sub {
    is(hmac_sha256_hex("k", "m"), hmac_sha256_hex("k", "m"), "deterministic");
    isnt(hmac_sha256_hex("k1", "m"), hmac_sha256_hex("k2", "m"), "key sensitivity");
    isnt(hmac_sha256_hex("k", "m1"), hmac_sha256_hex("k", "m2"), "message sensitivity");

    my @tag = @{ hmac_sha256("k", "m") };
    my $hex = join('', map { sprintf('%02x', $_) } @tag);
    is(hmac_sha256_hex("k", "m"), $hex, "hex matches bytes");
};

done_testing;
