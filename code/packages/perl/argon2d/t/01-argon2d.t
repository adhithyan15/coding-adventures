use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Argon2d;

sub A_d     { CodingAdventures::Argon2d::argon2d(@_)     }
sub A_d_hex { CodingAdventures::Argon2d::argon2d_hex(@_) }

# ─── RFC 9106 §5.1 gold-standard vector ──────────────────────────────────
#   password = 32 × 0x01
#   salt     = 16 × 0x02
#   key      =  8 × 0x03
#   ad       = 12 × 0x04
#   t=3, m=32, p=4, T=32
#   tag      = 512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb
my $RFC_PASSWORD = "\x01" x 32;
my $RFC_SALT     = "\x02" x 16;
my $RFC_KEY      = "\x03" x 8;
my $RFC_AD       = "\x04" x 12;
my $RFC_EXPECTED =
  '512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb';

is(
    A_d_hex(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    ),
    $RFC_EXPECTED,
    'RFC 9106 §5.1 vector',
);

# ─── hex == binary ──────────────────────────────────────────────────────
{
    my $raw = A_d(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    );
    my $hex = A_d_hex(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    );
    is(unpack("H*", $raw), $hex, 'hex matches binary');
}

# ─── Validation rejections ──────────────────────────────────────────────
like(
    dies { A_d('pw', 'short', 1, 8, 1, 32) },
    qr/salt/i,
    'rejects short salt',
);
like(
    dies { A_d('pw', 'a' x 8, 0, 8, 1, 32) },
    qr/time_cost/i,
    'rejects zero time_cost',
);
like(
    dies { A_d('pw', 'a' x 8, 1, 8, 1, 3) },
    qr/tag_length/i,
    'rejects tag_length < 4',
);
like(
    dies { A_d('pw', 'a' x 8, 1, 7, 1, 32) },
    qr/memory_cost/i,
    'rejects memory < 8*p',
);
like(
    dies { A_d('pw', 'a' x 8, 1, 8, 0, 32) },
    qr/parallelism/i,
    'rejects zero parallelism',
);
like(
    dies { A_d('pw', 'a' x 8, 1, 8, 1, 32, version => 0x10) },
    qr/v1\.3|0x13/i,
    'rejects unsupported version',
);

# ─── Determinism and separation ─────────────────────────────────────────
is(
    A_d_hex('pw', 'a' x 8, 1, 8, 1, 32),
    A_d_hex('pw', 'a' x 8, 1, 8, 1, 32),
    'deterministic',
);

isnt(
    A_d_hex('pw1', 'a' x 8, 1, 8, 1, 32),
    A_d_hex('pw2', 'a' x 8, 1, 8, 1, 32),
    'differs on password',
);

isnt(
    A_d_hex('pw', 'a' x 8, 1, 8, 1, 32),
    A_d_hex('pw', 'b' x 8, 1, 8, 1, 32),
    'differs on salt',
);

# ─── Key / AD binding ───────────────────────────────────────────────────
{
    my $no_key = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32);
    my $k1     = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32, key => 'k1');
    my $k2     = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32, key => 'k2');
    isnt($no_key, $k1, 'key binds (vs none)');
    isnt($k1,     $k2, 'key binds (k1 vs k2)');
}
{
    my $no_ad = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32);
    my $ad1   = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32, associated_data => 'x');
    my $ad2   = A_d_hex('pw', 'a' x 8, 1, 8, 1, 32, associated_data => 'y');
    isnt($no_ad, $ad1, 'AD binds (vs none)');
    isnt($ad1,   $ad2, 'AD binds (x vs y)');
}

# ─── Tag length variants ────────────────────────────────────────────────
is(length A_d('pw', 'a' x 8, 1, 8, 1,   4),   4, 'tag_length 4');
is(length A_d('pw', 'a' x 8, 1, 8, 1,  16),  16, 'tag_length 16');
is(length A_d('pw', 'a' x 8, 1, 8, 1,  65),  65, 'tag_length 65 (H\' boundary)');
is(length A_d('pw', 'a' x 8, 1, 8, 1, 128), 128, 'tag_length 128');

# ─── Parallelism / multi-pass ───────────────────────────────────────────
is(
    length A_d('pw', 'a' x 8, 1, 16, 2, 32),
    32,
    'multi-lane tag size',
);
isnt(
    A_d_hex('pw', 'a' x 8, 1, 8, 1, 32),
    A_d_hex('pw', 'a' x 8, 2, 8, 1, 32),
    'multi-pass differs from single-pass',
);

done_testing;
