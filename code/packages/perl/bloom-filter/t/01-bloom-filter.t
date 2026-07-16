use strict;
use warnings;
use utf8;
use Test::More;
use CodingAdventures::BloomFilter qw(
    optimal_m
    optimal_k
    capacity_for_memory
);

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'default filter starts empty' => sub {
    my $filter = CodingAdventures::BloomFilter->new;
    is($filter->bit_count, 9586, 'default bit count');
    is($filter->hash_count, 7, 'default hash count');
    is($filter->size_bytes, 1199, 'allocated bytes');
    is($filter->bits_set, 0, 'no bits set');
    is($filter->fill_ratio, 0, 'zero fill');
    is($filter->estimated_false_positive_rate, 0, 'zero estimated FPR');
    ok(!$filter->is_over_capacity, 'not over capacity');
    ok(!$filter->contains('anything'), 'empty filter misses value');
};

subtest 'inserted values have no false negatives' => sub {
    my $filter = CodingAdventures::BloomFilter->new(
        expected_items => 1_000,
        false_positive_rate => 0.01,
    );
    for my $index (1 .. 250) {
        $filter->add("item-$index");
    }
    my $all_present = 1;
    for my $index (1 .. 250) {
        $all_present &&= $filter->contains("item-$index");
    }
    ok($all_present, 'contains every inserted value');
    cmp_ok($filter->bits_set, '>', 0, 'adding values sets bits');
};

subtest 'explicit parameters are preserved' => sub {
    my $filter = CodingAdventures::BloomFilter->from_params(10_000, 7);
    is($filter->bit_count, 10_000, 'bit count');
    is($filter->hash_count, 7, 'hash count');
    is($filter->size_bytes, 1_250, 'byte count');
    $filter->add('hello');
    ok($filter->contains('hello'), 'contains inserted value');
    ok(!$filter->is_over_capacity, 'explicit filter does not track capacity');
};

subtest 'duplicate adds do not recount bits' => sub {
    my $filter = CodingAdventures::BloomFilter->new(expected_items => 100);
    $filter->add('duplicate');
    my $after_first = $filter->bits_set;
    $filter->add('duplicate');
    is($filter->bits_set, $after_first, 'bit count remains stable');
};

subtest 'sizing helpers match the formulas' => sub {
    my $bit_count = optimal_m(1_000_000, 0.01);
    is($bit_count, 9_585_059, 'optimal bit count');
    is(optimal_k($bit_count, 1_000_000), 7, 'optimal hash count');
    is(capacity_for_memory(1_000_000, 0.01), 834_632, 'memory capacity');
    is(capacity_for_memory(0, 0.01), 0, 'zero memory capacity');
};

subtest 'capacity and rendering reflect use' => sub {
    my $filter = CodingAdventures::BloomFilter->new(
        expected_items => 3,
        false_positive_rate => 0.01,
    );
    $filter->add($_) for qw(a b c);
    ok(!$filter->is_over_capacity, 'at capacity');
    $filter->add('d');
    ok($filter->is_over_capacity, 'over capacity');
    cmp_ok($filter->estimated_false_positive_rate, '>', 0, 'positive FPR');
    like("$filter", qr/BloomFilter/, 'stringifies with class name');
    like($filter->to_string, qr/bits_set=/, 'renders statistics');
};

subtest 'scalar and composite values are deterministic' => sub {
    my $filter = CodingAdventures::BloomFilter->new(expected_items => 100);
    my @values = (
        "cafe\x{301}",
        42,
        3.14,
        1,
        undef,
        ['math', 'code'],
        { name => 'Ada', tags => ['math', 'code'] },
    );
    for my $value (@values) {
        $filter->add($value);
        ok($filter->contains($value), 'contains encoded value');
    }
    ok(
        $filter->contains({ tags => ['math', 'code'], name => 'Ada' }),
        'hash ordering is stable',
    );
};

subtest 'invalid configurations and values are rejected' => sub {
    dies_like(
        sub { CodingAdventures::BloomFilter->new(expected_items => 0) },
        qr/expected_items must be a positive integer/,
        'zero expected items',
    );
    dies_like(
        sub { CodingAdventures::BloomFilter->new(false_positive_rate => 0) },
        qr/false_positive_rate must be in the open interval/,
        'zero false-positive rate',
    );
    dies_like(
        sub { CodingAdventures::BloomFilter->new(false_positive_rate => 1) },
        qr/false_positive_rate must be in the open interval/,
        'unit false-positive rate',
    );
    dies_like(
        sub { CodingAdventures::BloomFilter->from_params(0, 1) },
        qr/bit_count must be a positive integer/,
        'zero bit count',
    );
    dies_like(
        sub { CodingAdventures::BloomFilter->from_params(1, 0) },
        qr/hash_count must be a positive integer/,
        'zero hash count',
    );
    my $cycle = [];
    push @{$cycle}, $cycle;
    my $filter = CodingAdventures::BloomFilter->new;
    dies_like(
        sub { $filter->add($cycle) },
        qr/element references must not contain cycles/,
        'cyclic value',
    );
    dies_like(
        sub { $filter->add(sub { 1 }) },
        qr/element must be undef, a scalar, an array reference, or a hash reference/,
        'unsupported reference',
    );
};

done_testing;
