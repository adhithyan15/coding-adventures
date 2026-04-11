use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LZ78 qw(
    encode decode compress decompress
    serialise_tokens deserialise_tokens
    new_cursor cursor_step cursor_insert cursor_reset cursor_dict_id cursor_at_root
);

# ─── TrieCursor ────────────────────────────────────────────────────────────────

subtest 'TrieCursor' => sub {
    my $c = new_cursor();
    ok cursor_at_root($c), 'new cursor is at root';
    is cursor_dict_id($c), 0, 'new cursor dict_id is 0';

    is cursor_step($c, 65), 0, 'step returns false on empty trie';

    cursor_insert($c, 65, 1);
    ok cursor_at_root($c), 'insert does not advance cursor';

    is cursor_step($c, 65), 1, 'step returns true after insert';
    is cursor_dict_id($c), 1, 'dict_id is 1 after step';
    ok !cursor_at_root($c), 'not at root after step';

    cursor_reset($c);
    ok cursor_at_root($c), 'reset returns cursor to root';

    is cursor_step($c, 66), 0, 'step misses on different byte';

    # Simulate AABCBBABC encoding
    my $cursor  = new_cursor();
    my $next_id = 1;
    my @got;
    for my $byte (unpack 'C*', 'AABCBBABC') {
        unless (cursor_step($cursor, $byte)) {
            push @got, [cursor_dict_id($cursor), $byte];
            cursor_insert($cursor, $byte, $next_id++);
            cursor_reset($cursor);
        }
    }
    is \@got, [[0,65],[1,66],[0,67],[0,66],[4,65],[4,67]],
        'TrieCursor LZ78 simulation: AABCBBABC';
};

# ─── encode ────────────────────────────────────────────────────────────────────

subtest 'encode' => sub {
    my @empty = encode('');
    is scalar @empty, 0, 'empty input: no tokens';

    my @single = encode('A');
    is scalar @single, 1, 'single byte: one token';
    is $single[0]{dict_index}, 0, 'single byte: dict_index=0';
    is $single[0]{next_char}, 65, 'single byte: next_char=65';

    my @abcde = encode('ABCDE');
    is scalar @abcde, 5, 'no repetition: 5 tokens';
    for my $tok (@abcde) {
        is $tok->{dict_index}, 0, 'no repetition: all literals';
    }

    my @want_aabcbbabc = (
        {dict_index => 0, next_char => 65},
        {dict_index => 1, next_char => 66},
        {dict_index => 0, next_char => 67},
        {dict_index => 0, next_char => 66},
        {dict_index => 4, next_char => 65},
        {dict_index => 4, next_char => 67},
    );
    is [encode('AABCBBABC')], \@want_aabcbbabc, 'AABCBBABC tokens';

    my @want_ababab = (
        {dict_index => 0, next_char => 65},
        {dict_index => 0, next_char => 66},
        {dict_index => 1, next_char => 66},
        {dict_index => 3, next_char => 0},
    );
    is [encode('ABABAB')], \@want_ababab, 'ABABAB ends with flush token';

    my @aaaaaaa = encode('AAAAAAA');
    is scalar @aaaaaaa, 4, 'all identical: 4 tokens';
};

# ─── decode ────────────────────────────────────────────────────────────────────

subtest 'decode' => sub {
    is decode([], undef), '', 'empty tokens: empty string';

    my $s = decode([{dict_index => 0, next_char => 65}], 1);
    is $s, 'A', 'single literal token';

    my @tok_aabcbbabc = encode('AABCBBABC');
    is decode(\@tok_aabcbbabc, 9), 'AABCBBABC', 'AABCBBABC round-trips';

    my @tok_ababab = encode('ABABAB');
    is decode(\@tok_ababab, 6), 'ABABAB', 'ABABAB round-trips with original_length';
};

# ─── compress / decompress ─────────────────────────────────────────────────────

subtest 'round-trip' => sub {
    my @cases = (
        ['empty',    ''],
        ['single',   'A'],
        ['no rep',   'ABCDE'],
        ['identical','AAAAAAA'],
        ['AABCBBABC','AABCBBABC'],
        ['ABABAB',   'ABABAB'],
        ['hello',    'hello world'],
        ['repeat',   'ABC' x 100],
    );
    for my $case (@cases) {
        my ($label, $data) = @$case;
        is decompress(compress($data)), $data, "round-trip: $label";
    }

    # Null bytes
    my $data = "\x00\x00\x00\xff\xff";
    is decompress(compress($data)), $data, 'round-trip: binary with null bytes';

    # Full byte range
    my $range = join('', map { chr($_) } 0..255);
    is decompress(compress($range)), $range, 'round-trip: full byte range 0-255';

    # Repeated pattern
    my $rep = "\x00\x01\x02" x 100;
    is decompress(compress($rep)), $rep, 'round-trip: repeated pattern';
};

# ─── Parameters ────────────────────────────────────────────────────────────────

subtest 'max_dict_size' => sub {
    my @tokens = encode('ABCABCABCABCABC', 10);
    for my $tok (@tokens) {
        ok $tok->{dict_index} < 10, 'dict_index < max_dict_size';
    }

    my @tokens1 = encode('AAAA', 1);
    for my $tok (@tokens1) {
        is $tok->{dict_index}, 0, 'max_dict_size=1: all literals';
    }
};

# ─── Wire format ───────────────────────────────────────────────────────────────

subtest 'wire format' => sub {
    my $data = 'AB';
    my $compressed = compress($data);
    my @tokens = encode($data);
    is length($compressed), 8 + @tokens * 4, 'compressed size matches format';

    my $data2 = 'hello world test';
    is compress($data2), compress($data2), 'compress is deterministic';

    my ($tok_ref, $orig) = deserialise_tokens(compress($data));
    is $orig, length($data), 'deserialise: original_length correct';
    is scalar @$tok_ref, scalar @tokens, 'deserialise: token count correct';
};

# ─── Compression effectiveness ────────────────────────────────────────────────

subtest 'compression effectiveness' => sub {
    my $data = 'ABC' x 1000;
    ok length(compress($data)) < length($data), 'repetitive data compresses';

    my $all_a = 'A' x 10000;
    my $c = compress($all_a);
    ok length($c) < length($all_a), 'all same byte compresses';
    is decompress($c), $all_a, 'all same byte round-trips';
};

done_testing;
