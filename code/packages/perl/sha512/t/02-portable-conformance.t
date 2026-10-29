use strict;
use warnings;
use Test::More;
use FindBin;
use Math::BigInt;
use lib "$FindBin::Bin/../lib";
use CodingAdventures::Sha512;

my $path = "$FindBin::Bin/../../../../specs/fixtures/sha512-v1/cases.txt";
open my $handle, '<:raw', $path or die "open $path: $!";
local $/;
my $raw = <$handle>;
close $handle;
ok(length($raw) > 0 && length($raw) <= 4096, 'fixture file is bounded');
my @rows = grep { length($_) && substr($_, 0, 1) ne '#' } split /\r?\n/, $raw;
ok(@rows > 0 && @rows <= 64, 'fixture row count is bounded');

for my $row (grep { /^V\|/ } @rows) {
    my (undef, $id, $input_hex, undef, $expected) = split /\|/, $row, -1;
    ok(length($input_hex) <= 8192 && length($expected) == 128, "$id fields are bounded");
    my $input = pack('H*', $input_hex);
    ok(length($input) <= 4096, "$id input is bounded");
    is(CodingAdventures::Sha512::hex($input), $expected, "$id digest");
}

for my $row (grep { /^L\|/ } @rows) {
    my (undef, $id, $high_hex, $low_hex, $expected) = split /\|/, $row, -1;
    my $byte_count = Math::BigInt->from_hex("0x$high_hex")->blsft(64)
        ->bior(Math::BigInt->from_hex("0x$low_hex"));
    if ($expected eq 'ERROR') {
        my $ok = eval { CodingAdventures::Sha512::_encode_bit_length($byte_count); 1 };
        ok(!$ok, "$id rejects overflow");
        like($@, qr/less than 2\^128 bits/, "$id has stable error");
    } else {
        is(unpack('H*', CodingAdventures::Sha512::_encode_bit_length($byte_count)), $expected, "$id length");
    }
}

my $invalid = eval { CodingAdventures::Sha512::digest([]); 1 };
ok(!$invalid, 'rejects non-byte-string input');
like($@, qr/expected byte string/, 'invalid input has stable error');

done_testing();
