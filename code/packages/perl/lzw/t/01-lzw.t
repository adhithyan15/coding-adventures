use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LZW qw();

# ---- Constants ---------------------------------------------------------------

is(CodingAdventures::LZW::CLEAR_CODE,        256, 'CLEAR_CODE == 256');
is(CodingAdventures::LZW::STOP_CODE,         257, 'STOP_CODE == 257');
is(CodingAdventures::LZW::INITIAL_NEXT_CODE, 258, 'INITIAL_NEXT_CODE == 258');
is(CodingAdventures::LZW::INITIAL_CODE_SIZE, 9,   'INITIAL_CODE_SIZE == 9');
is(CodingAdventures::LZW::MAX_CODE_SIZE,     16,  'MAX_CODE_SIZE == 16');

# ---- encode_codes ------------------------------------------------------------

{
    my ($codes, $orig) = CodingAdventures::LZW::encode_codes('');
    is($orig,      0,                                'encode empty: orig');
    is($codes->[0], CodingAdventures::LZW::CLEAR_CODE, 'encode empty: first = CLEAR');
    is($codes->[-1], CodingAdventures::LZW::STOP_CODE, 'encode empty: last = STOP');
    is(scalar @$codes, 2,                            'encode empty: len=2');
}

{
    my ($codes, $orig) = CodingAdventures::LZW::encode_codes('A');
    is($orig, 1, 'encode single: orig');
    is($codes->[0],  CodingAdventures::LZW::CLEAR_CODE, 'encode single: first = CLEAR');
    is($codes->[-1], CodingAdventures::LZW::STOP_CODE,  'encode single: last = STOP');
    ok((grep { $_ == 65 } @$codes), 'encode single: contains 65');
}

{
    my ($codes, undef) = CodingAdventures::LZW::encode_codes('AB');
    is($codes, [CodingAdventures::LZW::CLEAR_CODE, 65, 66, CodingAdventures::LZW::STOP_CODE],
       'encode two distinct');
}

{
    my ($codes, undef) = CodingAdventures::LZW::encode_codes('ABABAB');
    is($codes, [CodingAdventures::LZW::CLEAR_CODE, 65, 66, 258, 258, CodingAdventures::LZW::STOP_CODE],
       'encode ABABAB');
}

{
    my ($codes, undef) = CodingAdventures::LZW::encode_codes('AAAAAAA');
    is($codes, [CodingAdventures::LZW::CLEAR_CODE, 65, 258, 259, 65, CodingAdventures::LZW::STOP_CODE],
       'encode AAAAAAA');
}

# ---- decode_codes ------------------------------------------------------------

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, CodingAdventures::LZW::STOP_CODE]),
   '', 'decode empty stream');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 65, CodingAdventures::LZW::STOP_CODE]),
   'A', 'decode single byte');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 65, 66, CodingAdventures::LZW::STOP_CODE]),
   'AB', 'decode two distinct');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 65, 66, 258, 258, CodingAdventures::LZW::STOP_CODE]),
   'ABABAB', 'decode ABABAB');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 65, 258, 259, 65, CodingAdventures::LZW::STOP_CODE]),
   'AAAAAAA', 'decode AAAAAAA tricky token');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 65, CodingAdventures::LZW::CLEAR_CODE, 66, CodingAdventures::LZW::STOP_CODE]),
   'AB', 'decode clear mid-stream');

is(CodingAdventures::LZW::decode_codes([CodingAdventures::LZW::CLEAR_CODE, 9999, 65, CodingAdventures::LZW::STOP_CODE]),
   'A', 'decode invalid code skipped');

# ---- pack / unpack -----------------------------------------------------------

{
    my $packed = CodingAdventures::LZW::pack_codes([CodingAdventures::LZW::CLEAR_CODE, CodingAdventures::LZW::STOP_CODE], 42);
    my ($stored) = unpack('N', substr($packed, 0, 4));
    is($stored, 42, 'pack: header stores original_length');
}

{
    my @codes = (CodingAdventures::LZW::CLEAR_CODE, 65, 66, 258, 258, CodingAdventures::LZW::STOP_CODE);
    my $packed = CodingAdventures::LZW::pack_codes(\@codes, 6);
    my ($unpacked, $orig) = CodingAdventures::LZW::unpack_codes($packed);
    is($orig, 6, 'pack/unpack ABABAB: orig');
    is($unpacked, \@codes, 'pack/unpack ABABAB: codes');
}

{
    my @codes = (CodingAdventures::LZW::CLEAR_CODE, 65, 258, 259, 65, CodingAdventures::LZW::STOP_CODE);
    my $packed = CodingAdventures::LZW::pack_codes(\@codes, 7);
    my ($unpacked, $orig) = CodingAdventures::LZW::unpack_codes($packed);
    is($orig, 7, 'pack/unpack AAAAAAA: orig');
    is($unpacked, \@codes, 'pack/unpack AAAAAAA: codes');
}

{
    my ($codes, $orig) = CodingAdventures::LZW::unpack_codes("\x00\x00");
    ok(ref($codes) eq 'ARRAY', 'unpack short: codes is arrayref');
    ok(!ref($orig), 'unpack short: orig is scalar');
}

# ---- compress / decompress ---------------------------------------------------

sub rt {
    my ($data) = @_;
    my $compressed = CodingAdventures::LZW::compress($data);
    return CodingAdventures::LZW::decompress($compressed);
}

is(rt(''),        '', 'compress empty');
is(rt('A'),       'A', 'compress single byte');
is(rt('AB'),      'AB', 'compress two distinct');
is(rt('ABABAB'),  'ABABAB', 'compress ABABAB');
is(rt('AAAAAAA'), 'AAAAAAA', 'compress AAAAAAA tricky token');
is(rt('AABABC'),  'AABABC', 'compress AABABC');

{
    my $data = 'the quick brown fox jumps over the lazy dog ' x 20;
    is(rt($data), $data, 'compress long string');
}

{
    my $data = join('', map { chr($_) } 0..255) x 2;
    is(rt($data), $data, 'compress binary data');
}

{
    my $data = "\x00" x 100;
    is(rt($data), $data, 'compress all zeros');
}

{
    my $data = "\xFF" x 100;
    is(rt($data), $data, 'compress all 0xFF');
}

{
    my $data = 'ABCABC' x 100;
    my $compressed = CodingAdventures::LZW::compress($data);
    ok(length($compressed) < length($data), 'compress: repetitive data shrinks');
}

{
    my $data = 'hello world';
    my $compressed = CodingAdventures::LZW::compress($data);
    my ($stored) = unpack('N', substr($compressed, 0, 4));
    is($stored, length($data), 'compress: header stores original_length');
}

done_testing();
