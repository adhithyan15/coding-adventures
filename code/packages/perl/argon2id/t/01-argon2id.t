use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Argon2id;

sub A     { CodingAdventures::Argon2id::argon2id(@_)     }
sub A_hex { CodingAdventures::Argon2id::argon2id_hex(@_) }

# ─── RFC 9106 §5.3 gold-standard vector ──────────────────────────────────
my $RFC_PASSWORD = "\x01" x 32;
my $RFC_SALT     = "\x02" x 16;
my $RFC_KEY      = "\x03" x 8;
my $RFC_AD       = "\x04" x 12;
my $RFC_EXPECTED =
  '0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659';

is(
    A_hex(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    ),
    $RFC_EXPECTED,
    'RFC 9106 §5.3 vector',
);

{
    my $raw = A(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    );
    my $hex = A_hex(
        $RFC_PASSWORD, $RFC_SALT, 3, 32, 4, 32,
        key => $RFC_KEY, associated_data => $RFC_AD,
    );
    is(unpack("H*", $raw), $hex, 'hex matches binary');
}

# ─── Validation rejections ──────────────────────────────────────────────
like(dies { A('pw', 'short',   1, 8, 1, 32) }, qr/salt/i,       'rejects short salt');
like(dies { A('pw', 'a' x 8,   0, 8, 1, 32) }, qr/time_cost/i,  'rejects zero time_cost');
like(dies { A('pw', 'a' x 8,   1, 8, 1,  3) }, qr/tag_length/i, 'rejects tag_length < 4');
like(dies { A('pw', 'a' x 8,   1, 7, 1, 32) }, qr/memory_cost/i,'rejects memory < 8*p');
like(dies { A('pw', 'a' x 8,   1, 8, 0, 32) }, qr/parallelism/i,'rejects zero parallelism');
like(dies { A('pw', 'a' x 8,   1, 8, 1, 32, version => 0x10) },
     qr/v1\.3|0x13/i, 'rejects unsupported version');

# ─── Determinism and separation ─────────────────────────────────────────
is(A_hex('pw', 'a' x 8, 1, 8, 1, 32),
   A_hex('pw', 'a' x 8, 1, 8, 1, 32),
   'deterministic');
isnt(A_hex('pw1', 'a' x 8, 1, 8, 1, 32),
     A_hex('pw2', 'a' x 8, 1, 8, 1, 32),
     'differs on password');
isnt(A_hex('pw', 'a' x 8, 1, 8, 1, 32),
     A_hex('pw', 'b' x 8, 1, 8, 1, 32),
     'differs on salt');

# ─── Key / AD binding ───────────────────────────────────────────────────
{
    my $no_key = A_hex('pw', 'a' x 8, 1, 8, 1, 32);
    my $k1     = A_hex('pw', 'a' x 8, 1, 8, 1, 32, key => 'k1');
    my $k2     = A_hex('pw', 'a' x 8, 1, 8, 1, 32, key => 'k2');
    isnt($no_key, $k1, 'key binds (vs none)');
    isnt($k1,     $k2, 'key binds (k1 vs k2)');
}
{
    my $no_ad = A_hex('pw', 'a' x 8, 1, 8, 1, 32);
    my $ad1   = A_hex('pw', 'a' x 8, 1, 8, 1, 32, associated_data => 'x');
    my $ad2   = A_hex('pw', 'a' x 8, 1, 8, 1, 32, associated_data => 'y');
    isnt($no_ad, $ad1, 'AD binds (vs none)');
    isnt($ad1,   $ad2, 'AD binds (x vs y)');
}

# ─── Tag length variants ────────────────────────────────────────────────
is(length A('pw', 'a' x 8, 1, 8, 1,   4),   4, 'tag_length 4');
is(length A('pw', 'a' x 8, 1, 8, 1,  16),  16, 'tag_length 16');
is(length A('pw', 'a' x 8, 1, 8, 1,  65),  65, 'tag_length 65');
is(length A('pw', 'a' x 8, 1, 8, 1, 128), 128, 'tag_length 128');

# ─── Parallelism / multi-pass ───────────────────────────────────────────
is(length A('pw', 'a' x 8, 1, 16, 2, 32), 32, 'multi-lane tag size');
isnt(A_hex('pw', 'a' x 8, 1, 8, 1, 32),
     A_hex('pw', 'a' x 8, 2, 8, 1, 32),
     'multi-pass differs from single-pass');

done_testing;
