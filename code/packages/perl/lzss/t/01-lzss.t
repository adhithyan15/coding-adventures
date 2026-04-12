use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LZSS qw(encode decode compress decompress make_literal make_match);

# ---- Helpers ----

sub all_literal {
    my @tokens = @_;
    for my $t (@tokens) {
        return 0 unless $t->{kind} eq 'literal';
    }
    return 1;
}

sub rt {
    my ($str) = @_;
    return decompress(compress($str));
}

# ---- Spec vectors ----

subtest 'encode empty' => sub {
    my @tokens = encode('');
    is scalar @tokens, 0, 'empty input → no tokens';
};

subtest 'encode single byte' => sub {
    my @tokens = encode('A');
    is scalar @tokens, 1, 'one token';
    is $tokens[0]{kind}, 'literal', 'kind is literal';
    is $tokens[0]{byte}, 65, 'byte is 65 (A)';
};

subtest 'encode no repetition → all literals' => sub {
    my @tokens = encode('ABCDE');
    is scalar @tokens, 5, '5 tokens';
    ok all_literal(@tokens), 'all literals';
};

subtest 'encode AABCBBABC → 7 tokens, last is Match(5,3)' => sub {
    my @tokens = encode('AABCBBABC');
    is scalar @tokens, 7, '7 tokens';
    is $tokens[6]{kind},   'match', 'last token is match';
    is $tokens[6]{offset}, 5,       'offset 5';
    is $tokens[6]{length}, 3,       'length 3';
};

subtest 'encode ABABAB → [Lit(A), Lit(B), Match(2,4)]' => sub {
    my @tokens = encode('ABABAB');
    is scalar @tokens, 3, '3 tokens';
    is $tokens[0]{kind}, 'literal', 'first is literal';
    is $tokens[0]{byte}, 65,        'byte A';
    is $tokens[1]{kind}, 'literal', 'second is literal';
    is $tokens[1]{byte}, 66,        'byte B';
    is $tokens[2]{kind},   'match', 'third is match';
    is $tokens[2]{offset}, 2,       'offset 2';
    is $tokens[2]{length}, 4,       'length 4';
};

subtest 'encode AAAAAAA → [Lit(A), Match(1,6)]' => sub {
    my @tokens = encode('AAAAAAA');
    is scalar @tokens, 2, '2 tokens';
    is $tokens[0]{kind}, 'literal', 'first literal';
    is $tokens[0]{byte}, 65,        'byte A';
    is $tokens[1]{kind},   'match', 'second match';
    is $tokens[1]{offset}, 1,       'offset 1';
    is $tokens[1]{length}, 6,       'length 6';
};

# ---- Encode properties ----

subtest 'match offset >= 1' => sub {
    my @tokens = encode('ABABABAB');
    for my $t (@tokens) {
        next unless $t->{kind} eq 'match';
        ok $t->{offset} >= 1, "offset $t->{offset} >= 1";
    }
};

subtest 'match length >= min_match (3)' => sub {
    my @tokens = encode('ABABABABABAB');
    for my $t (@tokens) {
        next unless $t->{kind} eq 'match';
        ok $t->{length} >= 3, "length $t->{length} >= 3";
    }
};

subtest 'large min_match forces all literals' => sub {
    my @tokens = encode('ABABAB', 4096, 255, 100);
    ok all_literal(@tokens), 'all literals when min_match=100';
};

# ---- Decode ----

subtest 'decode empty' => sub {
    is decode([], 0), '', 'empty decode';
};

subtest 'decode single literal' => sub {
    is decode([make_literal(65)], 1), 'A', 'decode A';
};

subtest 'decode overlapping match AAAAAAA' => sub {
    my @tokens = (make_literal(65), make_match(1, 6));
    is decode(\@tokens, 7), 'AAAAAAA', 'overlapping match';
};

subtest 'decode ABABAB' => sub {
    my @tokens = (make_literal(65), make_literal(66), make_match(2, 4));
    is decode(\@tokens, 6), 'ABABAB', 'ABABAB decode';
};

# ---- Round-trip ----

subtest 'round-trip empty' => sub {
    is rt(''), '', 'empty';
};

subtest 'round-trip single byte' => sub {
    is rt('A'), 'A', 'single byte';
};

subtest 'round-trip no repetition' => sub {
    is rt('ABCDE'), 'ABCDE', 'ABCDE';
};

subtest 'round-trip all identical' => sub {
    is rt('AAAAAAA'), 'AAAAAAA', 'AAAAAAA';
};

subtest 'round-trip ABABAB' => sub {
    is rt('ABABAB'), 'ABABAB', 'ABABAB';
};

subtest 'round-trip AABCBBABC' => sub {
    is rt('AABCBBABC'), 'AABCBBABC', 'AABCBBABC';
};

subtest 'round-trip hello world' => sub {
    is rt('hello world'), 'hello world', 'hello world';
};

subtest 'round-trip ABC x100' => sub {
    my $data = 'ABC' x 100;
    is rt($data), $data, 'ABC x100';
};

subtest 'round-trip binary nulls' => sub {
    my $data = "\x00\x00\x00\xff\xff";
    is rt($data), $data, 'binary nulls';
};

subtest 'round-trip full byte range' => sub {
    my $data = pack 'C*', 0..255;
    is rt($data), $data, 'full byte range';
};

subtest 'round-trip repeated pattern' => sub {
    my $data = pack 'C*', map { $_ % 3 } 0..299;
    is rt($data), $data, 'repeated 0,1,2 pattern';
};

subtest 'round-trip long ABCDEF' => sub {
    my $data = 'ABCDEF' x 500;
    is rt($data), $data, 'ABCDEF x500';
};

# ---- Wire format ----

subtest 'compress stores original length' => sub {
    my $compressed = compress('hello');
    my ($orig_len) = unpack 'N', substr($compressed, 0, 4);
    is $orig_len, 5, 'original length = 5';
};

subtest 'compress empty → 8-byte header' => sub {
    my $c = compress('');
    is length($c), 8, '8 bytes for empty';
    my ($orig_len, $block_count) = unpack 'NN', $c;
    is $orig_len,    0, 'orig_len = 0';
    is $block_count, 0, 'block_count = 0';
};

subtest 'compress is deterministic' => sub {
    my $data = 'hello world test';
    is compress($data), compress($data), 'deterministic';
};

subtest 'crafted large block_count is safe' => sub {
    my $bad = pack('NN', 4, 0x40000000) . "\x00ABCD";
    my $result = decompress($bad);
    ok defined $result, 'decompress returns a defined value';
};

# ---- Compression effectiveness ----

subtest 'repetitive data compresses' => sub {
    my $data = 'ABC' x 1000;
    ok length(compress($data)) < length($data), 'compressed < original';
};

subtest 'all same byte compresses' => sub {
    my $data = "\x42" x 10000;
    my $compressed = compress($data);
    ok length($compressed) < length($data), 'compressed < original';
    is decompress($compressed), $data, 'round-trip';
};

done_testing;
