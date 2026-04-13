use strict;
use warnings;
use Test::More;
use CodingAdventures::HuffmanTree;
no warnings 'redefine';

sub is ($$;$) {
    my ($got, $expected, $name) = @_;
    if (ref($got) || ref($expected)) {
        return Test::More::is_deeply($got, $expected, $name);
    }
    return Test::More::is($got, $expected, $name);
}

sub dies (&) {
    my ($code) = @_;
    return !eval { $code->(); 1 };
}

sub lives (&) {
    my ($code) = @_;
    return eval { $code->(); 1 } ? 1 : 0;
}

# ── Build validation ──────────────────────────────────────────────────────────

subtest 'build validation' => sub {
    ok dies { CodingAdventures::HuffmanTree->build([]) },
        'dies on empty weights';

    ok dies { CodingAdventures::HuffmanTree->build(undef) },
        'dies on undef weights';

    ok dies { CodingAdventures::HuffmanTree->build([[65, 0]]) },
        'dies on zero frequency';

    ok dies { CodingAdventures::HuffmanTree->build([[65, -1]]) },
        'dies on negative frequency';

    ok lives { CodingAdventures::HuffmanTree->build([[65, 5]]) },
        'succeeds with one symbol';

    my @many = map { [$_, $_] } 1..20;
    ok lives { CodingAdventures::HuffmanTree->build(\@many) },
        'succeeds with 20 symbols';
};

# ── code_table ────────────────────────────────────────────────────────────────

subtest 'code_table' => sub {
    # Heap construction trace for A(65,3), B(66,2), C(67,1):
    #   Priority tuples: C=[1,0,67,inf], B=[2,0,66,inf], A=[3,0,65,inf]
    #   Pop C(weight=1), pop B(weight=2) → Internal(w=3, left=C, right=B, order=0)
    #   Heap: A=[3,0,65,inf], Internal=[3,1,inf,0]
    #   Pop A (leaf wins tie), pop Internal → root(w=6, left=A, right=Internal)
    # Codes: A→"0", C→"10", B→"11"
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    my $tbl  = $tree->code_table();
    is $tbl->{65}, '0',  'A gets code 0';
    is $tbl->{67}, '10', 'C gets code 10';
    is $tbl->{66}, '11', 'B gets code 11';

    # Single symbol → code '0'
    my $t1  = CodingAdventures::HuffmanTree->build([[42, 7]]);
    my $t1b = $t1->code_table();
    is $t1b->{42}, '0', 'single symbol gets code 0';

    # Two symbols: each gets a 1-bit code
    my $t2 = CodingAdventures::HuffmanTree->build([[65,10],[66,1]]);
    my $tb = $t2->code_table();
    is length($tb->{65}), 1, 'two symbols: code lengths are 1';
    is length($tb->{66}), 1, 'two symbols: code lengths are 1';

    # All codes are distinct
    my @inputs = ([1,5],[2,3],[3,2],[4,1]);
    my $t = CodingAdventures::HuffmanTree->build(\@inputs);
    my $codes = $t->code_table();
    my %seen;
    for my $code (values %$codes) {
        ok !$seen{$code}, "code '$code' is unique";
        $seen{$code} = 1;
    }

    # Prefix-free property
    my @code_list = values %$codes;
    for my $i (0..$#code_list) {
        for my $j (0..$#code_list) {
            next if $i == $j;
            my ($a, $b) = ($code_list[$i], $code_list[$j]);
            ok substr($b, 0, length($a)) ne $a,
               "'$a' is not a prefix of '$b'";
        }
    }
};

# ── code_for ──────────────────────────────────────────────────────────────────

subtest 'code_for' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    my $tbl  = $tree->code_table();

    is $tree->code_for(65), $tbl->{65}, 'code_for matches code_table for A';
    is $tree->code_for(66), $tbl->{66}, 'code_for matches code_table for B';
    is $tree->code_for(67), $tbl->{67}, 'code_for matches code_table for C';
    is $tree->code_for(99), undef,      'returns undef for unknown symbol';

    my $t1 = CodingAdventures::HuffmanTree->build([[1, 1]]);
    is $t1->code_for(1), '0', 'single symbol returns 0';
};

# ── canonical_code_table ──────────────────────────────────────────────────────

subtest 'canonical_code_table' => sub {
    my $tree  = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    my $canon = $tree->canonical_code_table();
    # Lengths: A=1, B=2, C=2. Sorted: A(1), B(2), C(2).
    # Canonical: A→"0", B→"10", C→"11"
    is $canon->{65}, '0',  'canonical A → 0';
    is $canon->{66}, '10', 'canonical B → 10';
    is $canon->{67}, '11', 'canonical C → 11';

    # Single symbol → '0'
    my $t1 = CodingAdventures::HuffmanTree->build([[5, 10]]);
    is $t1->canonical_code_table()->{5}, '0', 'single symbol canonical → 0';

    # Canonical preserves same lengths as tree codes
    my @weights = ([1,5],[2,3],[3,2],[4,1],[5,1]);
    my $tree2   = CodingAdventures::HuffmanTree->build(\@weights);
    my $regular = $tree2->code_table();
    my $can2    = $tree2->canonical_code_table();
    for my $sym (keys %$regular) {
        is length($can2->{$sym}), length($regular->{$sym}),
            "canonical length matches for symbol $sym";
    }

    # Canonical codes are prefix-free
    my @codes = values %$can2;
    for my $i (0..$#codes) {
        for my $j (0..$#codes) {
            next if $i == $j;
            my ($a, $b) = ($codes[$i], $codes[$j]);
            ok substr($b, 0, length($a)) ne $a,
               "canonical: '$a' not prefix of '$b'";
        }
    }

    # Canonical codes are deterministic
    my $can3 = $tree2->canonical_code_table();
    for my $sym (keys %$can2) {
        is $can3->{$sym}, $can2->{$sym},
            "canonical deterministic for symbol $sym";
    }
};

# ── decode_all ────────────────────────────────────────────────────────────────

subtest 'decode_all' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);

    # A='0', C='10', B='11'
    # Decode 'A' from '0'
    my @r1 = $tree->decode_all('0', 1);
    is \@r1, [65], 'decode single A';

    # AABC = '0' + '0' + '11' + '10' = "001110"
    my @r2 = $tree->decode_all('001110', 4);
    is \@r2, [65, 65, 66, 67], 'decode AABC from 001110';

    # Single-leaf tree
    my $t1   = CodingAdventures::HuffmanTree->build([[42, 5]]);
    my @r3   = $t1->decode_all('000', 3);
    is \@r3, [42, 42, 42], 'single-leaf: each 0 bit decodes to symbol';

    # Decode 0 symbols
    my @r4 = $tree->decode_all('', 0);
    is \@r4, [], 'decode 0 symbols from empty string';

    # Round-trip
    my $tbl     = $tree->code_table();
    my @message = (65, 65, 65, 66, 66, 67);
    my $bits    = join('', map { $tbl->{$_} } @message);
    my @decoded = $tree->decode_all($bits, scalar @message);
    is \@decoded, \@message, 'round-trip AAABBC';

    # Exhausted stream dies
    ok dies { $tree->decode_all('1011', 5) },
        'dies on exhausted bit stream';

    # Five-symbol round-trip
    my @w2  = ([1,10],[2,5],[3,3],[4,2],[5,1]);
    my $t2  = CodingAdventures::HuffmanTree->build(\@w2);
    my $tb2 = $t2->code_table();
    my @msg2 = (1,2,3,4,5,1,1,3,2);
    my $b2   = join('', map { $tb2->{$_} } @msg2);
    my @dec2 = $t2->decode_all($b2, scalar @msg2);
    is \@dec2, \@msg2, 'five-symbol round-trip';
};

# ── weight / depth / symbol_count ─────────────────────────────────────────────

subtest 'weight' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    is $tree->weight(), 6, 'weight = sum of frequencies';

    my $t1 = CodingAdventures::HuffmanTree->build([[0, 100]]);
    is $t1->weight(), 100, 'single symbol weight';
};

subtest 'depth' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    is $tree->depth(), 2, 'AAABBC depth = 2';

    my $t1 = CodingAdventures::HuffmanTree->build([[1, 5]]);
    is $t1->depth(), 0, 'single symbol depth = 0';

    my $t2 = CodingAdventures::HuffmanTree->build([[1,1],[2,1]]);
    is $t2->depth(), 1, 'two equal symbols depth = 1';
};

subtest 'symbol_count' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    is $tree->symbol_count(), 3, 'symbol_count = 3';

    my $t1 = CodingAdventures::HuffmanTree->build([[7, 99]]);
    is $t1->symbol_count(), 1, 'single symbol count = 1';

    my @ten = map { [$_, $_] } 1..10;
    my $t10 = CodingAdventures::HuffmanTree->build(\@ten);
    is $t10->symbol_count(), 10, 'ten symbols count = 10';
};

# ── leaves ────────────────────────────────────────────────────────────────────

subtest 'leaves' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    my @lvs  = $tree->leaves();
    is scalar @lvs, 3, 'three leaves';
    # In-order: A(left=0), then C(left child of internal=10), B(right=11)
    is $lvs[0][0], 65,   'first leaf symbol = A';
    is $lvs[0][1], '0',  'first leaf code = 0';
    is $lvs[1][0], 67,   'second leaf symbol = C';
    is $lvs[1][1], '10', 'second leaf code = 10';
    is $lvs[2][0], 66,   'third leaf symbol = B';
    is $lvs[2][1], '11', 'third leaf code = 11';

    # All symbols appear exactly once
    my %seen;
    for my $pair (@lvs) {
        ok !$seen{$pair->[0]}, "symbol $pair->[0] unique in leaves";
        $seen{$pair->[0]} = 1;
    }

    # Single leaf
    my $t1   = CodingAdventures::HuffmanTree->build([[99, 7]]);
    my @lvs1 = $t1->leaves();
    is scalar @lvs1, 1,   'single leaf count';
    is $lvs1[0][0],  99,  'single leaf symbol';
    is $lvs1[0][1],  '0', 'single leaf code';
};

# ── is_valid ──────────────────────────────────────────────────────────────────

subtest 'is_valid' => sub {
    my $tree = CodingAdventures::HuffmanTree->build([[65,3],[66,2],[67,1]]);
    is $tree->is_valid(), 1, 'valid tree returns 1';

    my $t1 = CodingAdventures::HuffmanTree->build([[1, 10]]);
    is $t1->is_valid(), 1, 'single symbol tree valid';

    my @big = map { [$_, $_ * 2] } 1..15;
    my $tb = CodingAdventures::HuffmanTree->build(\@big);
    is $tb->is_valid(), 1, 'large tree valid';
};

# ── All-equal weights ─────────────────────────────────────────────────────────

subtest 'all equal weights' => sub {
    my $t2 = CodingAdventures::HuffmanTree->build([[1,1],[2,1]]);
    is $t2->depth(), 1, 'two equal: depth 1';
    is $t2->is_valid(), 1, 'two equal: valid';

    my $t4 = CodingAdventures::HuffmanTree->build([[1,1],[2,1],[3,1],[4,1]]);
    is $t4->depth(), 2, 'four equal: depth 2';
    is $t4->is_valid(), 1, 'four equal: valid';

    my $t8 = CodingAdventures::HuffmanTree->build(
        [[1,1],[2,1],[3,1],[4,1],[5,1],[6,1],[7,1],[8,1]]);
    is $t8->depth(), 3, 'eight equal: depth 3';
    is $t8->is_valid(), 1, 'eight equal: valid';
};

# ── Determinism ───────────────────────────────────────────────────────────────

subtest 'determinism' => sub {
    my @w  = ([1,5],[2,3],[3,2],[4,1],[5,1]);
    my $t1 = CodingAdventures::HuffmanTree->build(\@w);
    my $t2 = CodingAdventures::HuffmanTree->build(\@w);
    my $c1 = $t1->code_table();
    my $c2 = $t2->code_table();
    for my $sym (keys %$c1) {
        is $c2->{$sym}, $c1->{$sym},
            "deterministic code for symbol $sym";
    }

    # Tie-breaking: equal weights
    my $tt = CodingAdventures::HuffmanTree->build([[1,1],[2,1],[3,1],[4,1]]);
    is $tt->depth(), 2, 'tie-break: four equal depth = 2';
    is $tt->is_valid(), 1, 'tie-break: four equal valid';
};

# ── Byte-range round-trip ─────────────────────────────────────────────────────

subtest 'byte-range round-trip' => sub {
    my @weights = map { [$_, $_ + 1] } 0..15;
    my $tree    = CodingAdventures::HuffmanTree->build(\@weights);
    my $tbl     = $tree->code_table();
    my @message = (0..15);
    my $bits    = join('', map { $tbl->{$_} } @message);
    my @decoded = $tree->decode_all($bits, scalar @message);
    is \@decoded, \@message, 'byte-range round-trip';
};

done_testing;
