use strict;
use warnings;
use Test::More;
use FindBin;
use Math::BigInt;
use File::Temp qw(tempfile);
use lib "$FindBin::Bin/../lib";
use CodingAdventures::Sha512;

my $path = "$FindBin::Bin/../../../../specs/fixtures/sha512-v1/cases.txt";

sub parse_fixture {
    my ($source) = @_;
    die "fixture byte size outside bounds" unless defined($source) && length($source) > 0 && length($source) <= 4096;
    my @parsed = grep { length($_) && substr($_, 0, 1) ne '#' } split /\r?\n/, $source;
    die "fixture row count outside bounds" unless @parsed > 0 && @parsed <= 64;
    for my $row (@parsed) {
        my @fields = split /\|/, $row, -1;
        die "malformed fixture row" unless @fields == 5 && $fields[1] =~ /\A[a-z0-9]+(?:-[a-z0-9]+)*\z/;
        if ($fields[0] eq 'V') {
            die "malformed vector row" unless length($fields[2]) <= 8192 && $fields[2] =~ /\A(?:[0-9a-f]{2})*\z/
                && $fields[4] =~ /\A[0-9a-f]{128}\z/;
            my $byte_count = length($fields[2]) / 2;
            die "malformed vector size" if $byte_count > 4096 || (($byte_count == 0) != ($fields[3] eq ''));
            my @chunks = $fields[3] eq '' ? () : split /,/, $fields[3], -1;
            die "malformed chunks" if @chunks > 64 || ($fields[3] ne '' && $fields[3] !~ /\A[1-9][0-9]*(?:,[1-9][0-9]*)*\z/);
            my $sum = 0;
            for my $chunk (@chunks) {
                die "malformed chunk" if length($chunk) > 4 || $chunk > 4096;
                $sum += $chunk;
            }
            die "malformed chunks" unless $sum == $byte_count;
        }
        elsif ($fields[0] eq 'L') {
            die "malformed length row" unless $fields[2] =~ /\A[0-9a-f]{16}\z/ && $fields[3] =~ /\A[0-9a-f]{16}\z/
                && ($fields[4] eq 'ERROR' || $fields[4] =~ /\A[0-9a-f]{32}\z/);
        }
        else { die "unknown fixture row tag"; }
    }
    return @parsed;
}

sub load_fixture {
    my ($fixture_path) = @_;
    open my $handle, '<:raw', $fixture_path or die "open $fixture_path: $!";
    my $raw = '';
    my $read = read $handle, $raw, 4097;
    die "read $fixture_path: $!" unless defined $read;
    close $handle or die "close $fixture_path: $!";
    return parse_fixture($raw);
}

my @rows = load_fixture($path);
is(scalar(grep { /^V\|/ } @rows), 9, 'fixture retains nine vector rows');
is(scalar(grep { /^L\|/ } @rows), 6, 'fixture retains six length rows');

my $zero_digest = '0' x 128;
for my $bad ('X|id|||', "V|UPPER|00|1|$zero_digest", "V|bad|0|1|$zero_digest", "V|bad|00|2|$zero_digest",
             'L|bad|000000000000000G|0000000000000000|ERROR', 'L|bad|0000000000000000|0000000000000000|ABC') {
    my $ok = eval { parse_fixture($bad); 1 };
    ok(!$ok, "rejects malformed fixture row: $bad");
}

my ($oversized, $oversized_path) = tempfile();
binmode $oversized;
print {$oversized} 'x' x 4097;
close $oversized;
my $oversized_ok = eval { load_fixture($oversized_path); 1 };
ok(!$oversized_ok, 'rejects oversized fixture through real file reader');
unlink $oversized_path;

for my $row (grep { /^V\|/ } @rows) {
    my (undef, $id, $input_hex, undef, $expected) = split /\|/, $row, -1;
    my $input = pack('H*', $input_hex);
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

my $text = "\x{e9}";
utf8::upgrade($text);
my $text_ok = eval { CodingAdventures::Sha512::digest($text); 1 };
ok(!$text_ok, 'rejects UTF-8-flagged text');
like($@, qr/sha512\.digest: expected byte string/, 'UTF-8 text has stable byte-string error');
my $octets = pack('C*', 0xc3, 0xa9);
is(CodingAdventures::Sha512::hex($octets), CodingAdventures::Sha512::hex(pack('H*', 'c3a9')), 'accepts equivalent octets');

done_testing();
