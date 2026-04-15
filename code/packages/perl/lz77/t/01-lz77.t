use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LZ77 qw(encode decode compress decompress);

# ---- Helpers ----

sub all_literal {
    my @tokens = @_;
    for my $t (@tokens) {
        return 0 unless $t->{offset} == 0 && $t->{length} == 0;
    }
    return 1;
}

sub rt {
    my ($str) = @_;
    return decompress(compress($str));
}

# ---- Specification Test Vectors ----

subtest 'empty input' => sub {
    my @tokens = encode('');
    is scalar @tokens, 0, 'empty input produces no tokens';
    is decode([]), '', 'empty tokens decode to empty string';
};

subtest 'no repetition - all literals' => sub {
    my @tokens = encode('ABCDE');
    is scalar @tokens, 5, '5 tokens for ABCDE';
    ok all_literal(@tokens), 'all literal tokens';
};

subtest 'all identical bytes exploit overlap' => sub {
    my @tokens = encode('AAAAAAA');
    is scalar @tokens, 2, '2 tokens for AAAAAAA';
    is $tokens[0]{offset},    0,  'first token offset 0';
    is $tokens[0]{length},    0,  'first token length 0';
    is $tokens[0]{next_char}, 65, 'first token next_char A';
    is $tokens[1]{offset},    1,  'second token offset 1';
    is $tokens[1]{length},    5,  'second token length 5';
    is $tokens[1]{next_char}, 65, 'second token next_char A';
    is decode(\@tokens), 'AAAAAAA', 'round-trip AAAAAAA';
};

subtest 'repeated pair uses backreference' => sub {
    my @tokens = encode('ABABABAB');
    is scalar @tokens, 3, '3 tokens for ABABABAB';
    is $tokens[0]{next_char}, 65, 'A literal';
    is $tokens[1]{next_char}, 66, 'B literal';
    is $tokens[2]{offset},    2,  'backreference offset 2';
    is $tokens[2]{length},    5,  'backreference length 5';
    is $tokens[2]{next_char}, 66, 'backreference next_char B';
    is decode(\@tokens), 'ABABABAB', 'round-trip ABABABAB';
};

subtest 'AABCBBABC with min_match=3 is all literals' => sub {
    my @tokens = encode('AABCBBABC');
    is scalar @tokens, 9, '9 literal tokens';
    ok all_literal(@tokens), 'all literal';
    is decode(\@tokens), 'AABCBBABC', 'round-trip';
};

subtest 'min_match=2 allows shorter matches' => sub {
    my @tokens = encode('AABCBBABC', 4096, 255, 2);
    is decode(\@tokens), 'AABCBBABC', 'round-trip with min_match=2';
};

# ---- Round-Trip Tests ----

subtest 'round-trip various strings' => sub {
    my @cases = ('', 'A', 'hello world', 'the quick brown fox',
                 'ababababab', 'aaaaaaaaaa');
    for my $s (@cases) {
        is rt($s), $s, "round-trip: $s";
    }
};

subtest 'round-trip binary data' => sub {
    is rt("\x00\x00\x00"), "\x00\x00\x00", 'null bytes';
    is rt("\xff\xff\xff"), "\xff\xff\xff", '0xFF bytes';
};

# ---- Parameter Tests ----

subtest 'window_size limit' => sub {
    my $data = 'X' . ('Y' x 5000) . 'X';
    my @tokens = encode($data, 100);
    for my $t (@tokens) {
        ok $t->{offset} <= 100, "offset $t->{offset} <= 100";
    }
};

subtest 'max_match limit' => sub {
    my $data = 'A' x 1000;
    my @tokens = encode($data, 4096, 50);
    for my $t (@tokens) {
        ok $t->{length} <= 50, "length $t->{length} <= 50";
    }
};

subtest 'min_match threshold' => sub {
    my @tokens = encode('AABAA', 4096, 255, 2);
    for my $t (@tokens) {
        ok $t->{length} == 0 || $t->{length} >= 2, "length ok";
    }
};

# ---- Edge Cases ----

subtest 'single byte literal' => sub {
    my @tokens = encode('X');
    is scalar @tokens, 1, 'one token';
    is $tokens[0]{offset},    0,  'offset 0';
    is $tokens[0]{length},    0,  'length 0';
    is $tokens[0]{next_char}, 88, 'next_char X';
};

subtest 'overlapping match decode' => sub {
    my @tokens = (
        { offset => 0, length => 0, next_char => 65 },  # A
        { offset => 0, length => 0, next_char => 66 },  # B
        { offset => 2, length => 5, next_char => 90 },  # overlap -> ABABAB + Z
    );
    is decode(\@tokens), 'ABABABAZ', 'overlapping match gives ABABABAZ';
};

subtest 'binary with nulls' => sub {
    my $data = "\x00\x00\x00\xff\xff";
    is rt($data), $data, 'binary with nulls round-trip';
};

subtest 'long run of identical bytes compresses well' => sub {
    my $data = 'A' x 10_000;
    my @tokens = encode($data);
    ok scalar @tokens < 50, 'less than 50 tokens for 10000 As';
    is decode(\@tokens), $data, 'round-trip';
};

# ---- Serialisation Tests ----

subtest 'serialised format size' => sub {
    my @tokens = (
        { offset => 0, length => 0, next_char => 65 },
        { offset => 2, length => 5, next_char => 66 },
    );
    my $serialised = CodingAdventures::LZ77::_serialise_tokens(\@tokens);
    is length($serialised), 4 + 2 * 4, 'correct byte length';
};

subtest 'compress/decompress all spec vectors' => sub {
    for my $s ('', 'ABCDE', 'AAAAAAA', 'ABABABAB', 'AABCBBABC') {
        is rt($s), $s, "compress/decompress: $s";
    }
};

# ---- Behaviour Tests ----

subtest 'incompressible data size bound' => sub {
    my $data = pack 'C*', 0..255;
    my $compressed = compress($data);
    ok length($compressed) <= 4 * length($data) + 10, 'size bound holds';
};

subtest 'repetitive data compresses' => sub {
    my $data = 'ABC' x 100;
    my $compressed = compress($data);
    ok length($compressed) < length($data), 'compressed < original';
};

subtest 'deterministic compression' => sub {
    my $data = 'hello world test';
    is compress($data), compress($data), 'deterministic';
};

done_testing;
