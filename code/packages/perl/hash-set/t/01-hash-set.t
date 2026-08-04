use strict;
use warnings;
use Test::More;
use CodingAdventures::HashSet qw(
    new_set with_options from_list from_list_with_options
    add remove discard contains has size is_empty to_list
    union intersection difference symmetric_difference
    is_subset is_superset is_disjoint equals
);

sub sorted_numbers {
    my ($set) = @_;
    return [sort { $a <=> $b } @{$set->to_list}];
}

sub sorted_strings {
    my ($set) = @_;
    return [sort @{$set->to_list}];
}

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'deduplicates and checks membership with both strategies' => sub {
    for my $strategy (qw(chaining open_addressing)) {
        my $set = CodingAdventures::HashSet->from_list(
            [1, 1, 2, 2, 3],
            capacity => 4,
            strategy => $strategy,
        );
        is($set->size, 3, "$strategy size");
        is($set->len, 3, "$strategy len");
        ok($set->contains(1), "$strategy contains");
        ok($set->has(2), "$strategy has alias");
        ok(!$set->contains(4), "$strategy missing");
        ok(!$set->is_empty, "$strategy nonempty");
        is_deeply(sorted_numbers($set), [1, 2, 3], "$strategy values");
    }
};

subtest 'add remove and discard are persistent' => sub {
    my $base = from_list([qw(alpha beta)]);
    my $added = $base->add('gamma');
    my $removed = $added->remove('alpha');
    my $unchanged = $removed->discard('missing');
    is_deeply(sorted_strings($base), [qw(alpha beta)], 'base unchanged');
    is_deeply(sorted_strings($added), [qw(alpha beta gamma)], 'add result');
    is_deeply(sorted_strings($removed), [qw(beta gamma)], 'remove result');
    is_deeply(sorted_strings($unchanged), [qw(beta gamma)], 'discard missing');
};

subtest 'complete set algebra works' => sub {
    my $left = from_list([1, 2, 3, 4, 5]);
    my $right = from_list([3, 4, 5, 6, 7]);
    is_deeply(sorted_numbers($left->union($right)), [1 .. 7], 'union');
    is_deeply(sorted_numbers($left->intersection($right)), [3, 4, 5], 'intersection');
    is_deeply(sorted_numbers($left->difference($right)), [1, 2], 'difference');
    is_deeply(sorted_numbers($left->symmetric_difference($right)), [1, 2, 6, 7], 'symmetric difference');
    is_deeply(sorted_numbers($left), [1, 2, 3, 4, 5], 'left unchanged');
};

subtest 'relation predicates cover empty and overlapping sets' => sub {
    my $subset = from_list([1, 2, 3]);
    my $superset = from_list([1, 2, 3, 4, 5]);
    my $disjoint = from_list([10, 20]);
    ok($subset->is_subset($superset), 'subset');
    ok($superset->is_superset($subset), 'superset');
    ok(!$superset->is_subset($subset), 'not subset');
    ok($subset->is_disjoint($disjoint), 'disjoint');
    ok(!$subset->is_disjoint($superset), 'overlap');
    ok($subset->equals(from_list([3, 2, 1])), 'equals');
    ok(!$subset->equals($superset), 'not equals');
    ok(new_set()->is_subset($subset), 'empty subset');
};

subtest 'hash map options are preserved' => sub {
    my $set = with_options(4, 'open', 'murmur3_32')->add('Ada')->add('Grace');
    is($set->strategy, 'open_addressing', 'strategy normalized');
    is($set->hash_fn, 'murmur3', 'hash normalized');
    cmp_ok($set->capacity, '>=', 4, 'capacity preserved');
    my $seeded = from_list_with_options([qw(a b b c)], 2, 'chaining', 'djb2');
    is($seeded->size, 3, 'seeded deduplicates');
    is($seeded->strategy, 'chaining', 'seeded strategy');
    is($seeded->hash_fn, 'djb2', 'seeded hash');
    is($set->intersection($seeded)->strategy, 'open_addressing', 'left options preserved');
    is($set->union($seeded)->strategy, 'open_addressing', 'union preserves left options');
};

subtest 'exported functional wrappers work' => sub {
    my $set = from_list([10, 20]);
    $set = add($set, 30);
    ok(contains($set, 30), 'contains wrapper');
    $set = remove($set, 20);
    ok(!has($set, 20), 'remove and has wrappers');
    $set = discard($set, 99);
    my $other = from_list([30, 40]);
    my $unioned = union($set, $other);
    is_deeply(sorted_numbers($unioned), [10, 30, 40], 'union wrapper');
    is_deeply(sorted_numbers(intersection($set, $other)), [30], 'intersection wrapper');
    is_deeply(sorted_numbers(difference($set, $other)), [10], 'difference wrapper');
    is_deeply(sorted_numbers(symmetric_difference($set, $other)), [10, 40], 'symmetric wrapper');
    ok(is_subset($set, $unioned), 'subset wrapper');
    ok(is_superset($unioned, $set), 'superset wrapper');
    ok(is_disjoint($set, from_list([999])), 'disjoint wrapper');
    ok(equals($set, $set->clone), 'equals wrapper');
    is(size($set), 2, 'size wrapper');
    ok(!is_empty($set), 'empty wrapper');
    is_deeply([sort { $a <=> $b } @{to_list($set)}], [10, 30], 'list wrapper');
};

subtest 'reference elements use identity semantics' => sub {
    my $reference = [];
    my $other_reference = [];
    my $set = from_list([$reference, $reference, $other_reference]);
    is($set->size, 2, 'duplicate reference ignored');
    ok($set->contains($reference), 'first reference found');
    ok($set->contains($other_reference), 'second reference found');
    ok(!$set->contains([]), 'different reference absent');
    ok($set->remove($reference)->contains($other_reference), 'other reference survives');
};

subtest 'underlying map resizes retain every element' => sub {
    for my $strategy (qw(chaining open_addressing)) {
        my $set = new_set(capacity => 2, strategy => $strategy);
        $set = $set->add("key-$_") for 1 .. 100;
        is($set->size, 100, "$strategy size");
        cmp_ok($set->capacity, '>=', 100, "$strategy capacity");
        ok($set->contains("key-$_"), "$strategy key $_") for 1 .. 100;
    }
};

subtest 'invalid inputs are rejected' => sub {
    dies_like(sub { from_list('not-an-array') }, qr/elements must be an array reference/, 'non-array');
    dies_like(sub { new_set(capacity => 0) }, qr/capacity must be a positive integer/, 'zero capacity');
    dies_like(sub { new_set(strategy => 'quadratic') }, qr/strategy must be 'chaining' or 'open_addressing'/, 'strategy');
    dies_like(sub { new_set(hash_fn => 'sha256') }, qr/hash_fn must be 'fnv1a', 'murmur3', or 'djb2'/, 'hash');
    dies_like(sub { new_set()->add(undef) }, qr/element must be defined/, 'undef element');
    dies_like(sub { new_set()->union('not-a-set') }, qr/other must be a CodingAdventures::HashSet/, 'other type');
};

done_testing;
