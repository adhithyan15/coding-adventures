use strict;
use warnings;
use Test::More;
use CodingAdventures::SkipList;

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'insert, update, search, and membership' => sub {
    my $list = CodingAdventures::SkipList->new;
    ok($list->insert(5, 'five'), 'new key inserted');
    ok($list->insert(2, 'two'), 'second key inserted');
    ok($list->insert(8, 'eight'), 'third key inserted');
    ok(!$list->insert(5, 'FIVE'), 'existing key updated');
    is($list->search(5), 'FIVE', 'updated value returned');
    is($list->get(2), 'two', 'get alias');
    ok(!defined $list->search(99), 'missing lookup');
    ok($list->contains(8), 'contains key');
    ok($list->has(2), 'has alias');
    ok(!$list->contains(7), 'absent key');
    is($list->size, 3, 'size');
    is($list->length, 3, 'length');
    ok($list->is_valid_skip_list, 'skip-list invariants valid');
};

subtest 'undefined values remain distinguishable from missing keys' => sub {
    my $list = CodingAdventures::SkipList->new;
    $list->insert(7, undef);
    ok($list->contains(7), 'key with undefined value is present');
    ok(!defined $list->search(7), 'stored undefined value returned');
    ok(!$list->contains(8), 'missing key absent');
    ok($list->is_valid_skip_list, 'invariants valid');
};

subtest 'delete and defensive ordered snapshots' => sub {
    my $list = CodingAdventures::SkipList->from_entries([[3, 'c'], [1, 'a'], [2, 'b']]);
    ok($list->delete(2), 'existing key deleted');
    ok(!$list->remove(2), 'absent key not deleted twice');
    ok(!$list->delete(undef), 'undefined key absent');
    is_deeply($list->to_list, [1, 3], 'sorted keys remain');
    my $snapshot = $list->to_list;
    $snapshot->[0] = 999;
    is_deeply($list->to_list, [1, 3], 'snapshot is defensive');
    is("$list", 'SkipList([1, 3])', 'string rendering');
    ok($list->is_valid_skip_list, 'invariants valid after delete');
};

subtest 'entries, iterator, and boundaries' => sub {
    my $list = CodingAdventures::SkipList->from_entries([[5, 'e'], [1, 'a'], [3, 'c'], [9, 'i']]);
    is_deeply($list->entries, [[1, 'a'], [3, 'c'], [5, 'e'], [9, 'i']], 'ordered entries');
    my $iterator = $list->iterator;
    my @iterated;
    while (my ($key, $value) = $iterator->()) {
        push @iterated, "$key$value";
    }
    is_deeply(\@iterated, ['1a', '3c', '5e', '9i'], 'iterator yields pairs');
    is($list->min, 1, 'minimum');
    is($list->max, 9, 'maximum');
};

subtest 'rank and selection use sorted order' => sub {
    my $list = CodingAdventures::SkipList->from_entries([[10, 1], [20, 2], [30, 3], [40, 4]]);
    is($list->rank(10), 0, 'first rank');
    is($list->rank(30), 2, 'middle rank');
    ok(!defined $list->rank(25), 'missing rank');
    is($list->by_rank(0), 10, 'first selection');
    is($list->by_rank(3), 40, 'last selection');
    ok(!defined $list->by_rank(-1), 'negative rank');
    ok(!defined $list->by_rank(4), 'rank after end');
    is($list->kth_smallest(3), 30, 'one-based selection');
    ok(!defined $list->kth_smallest(0), 'invalid one-based selection');
};

subtest 'inclusive and exclusive ranges' => sub {
    my $list = CodingAdventures::SkipList->from_entries(
        [[5, 50], [12, 120], [20, 200], [37, 370], [42, 420]],
    );
    is_deeply($list->range_query(12, 37), [[12, 120], [20, 200], [37, 370]], 'inclusive range');
    is_deeply($list->range(12, 37, 0), [[20, 200]], 'exclusive range');
    is_deeply($list->range(50, 10), [], 'reversed range');
    is_deeply($list->range(100, 200), [], 'empty range');
};

subtest 'empty state and custom comparator' => sub {
    my $empty = CodingAdventures::SkipList->new;
    ok($empty->is_empty, 'empty list');
    ok(!defined $empty->min, 'empty minimum');
    ok(!defined $empty->max, 'empty maximum');
    ok(!defined $empty->by_rank(0), 'empty selection');
    is_deeply($empty->entries, [], 'empty entries');
    is("$empty", 'SkipList([])', 'empty rendering');

    my $by_length = sub {
        return length($_[0]) <=> length($_[1]) || $_[0] cmp $_[1];
    };
    my $custom = CodingAdventures::SkipList->new(max_level => 8, compare => $by_length, seed => 9);
    $custom->insert('banana', 6);
    $custom->insert('fig', 3);
    $custom->insert('apple', 5);
    is_deeply($custom->to_list, ['fig', 'apple', 'banana'], 'custom ordering');
    is($custom->by_rank(1), 'apple', 'custom selection');
    ok($custom->is_valid_skip_list, 'custom invariants valid');
};

subtest 'deterministic topology preserves spans through mutations' => sub {
    my $list = CodingAdventures::SkipList->new(max_level => 8, probability => 0.75, seed => 42);
    is($list->max_level, 8, 'max level stored');
    is($list->probability, 0.75, 'probability stored');
    my %expected;
    my $valid = 1;
    for my $index (1 .. 200) {
        my $key = ($index * 73) % 211;
        $list->insert($key, $index);
        $expected{$key} = 1;
        $valid &&= $list->is_valid_skip_list;
    }
    for my $key (grep { $_ % 3 == 1 } 1 .. 209) {
        $list->delete($key);
        delete $expected{$key};
        $valid &&= $list->is_valid_skip_list;
    }
    ok($valid, 'all mutation invariants valid');
    ok($list->current_level >= 1 && $list->current_level <= 8, 'active level bounded');

    my @sorted = sort { $a <=> $b } keys %expected;
    is_deeply($list->to_list, \@sorted, 'sorted reference matches');
    my $rank_roundtrips = 1;
    for my $rank (0 .. $#sorted) {
        $rank_roundtrips &&= defined($list->rank($sorted[$rank]))
            && $list->rank($sorted[$rank]) == $rank
            && defined($list->by_rank($rank))
            && $list->by_rank($rank) == $sorted[$rank];
    }
    ok($rank_roundtrips, 'all rank and selection roundtrips match');
};

subtest 'public input validation' => sub {
    dies_like(sub { CodingAdventures::SkipList->new(max_level => 0) }, qr/max_level must be a positive integer/, 'invalid max level rejected');
    dies_like(sub { CodingAdventures::SkipList->new(probability => 1) }, qr/probability must be between 0 and 1/, 'invalid probability rejected');
    dies_like(sub { CodingAdventures::SkipList->new(compare => 'compare') }, qr/compare must be a code reference/, 'invalid comparator rejected');
    dies_like(sub { CodingAdventures::SkipList->new(seed => 1.5) }, qr/seed must be an integer/, 'invalid seed rejected');
    dies_like(sub { CodingAdventures::SkipList->new->insert(undef) }, qr/key must be defined/, 'undefined key rejected');
    dies_like(sub { CodingAdventures::SkipList->from_entries('entries') }, qr/entries must be an array reference/, 'invalid entries rejected');
    dies_like(sub { CodingAdventures::SkipList->new->range(undef, 1) }, qr/minimum must be defined/, 'undefined minimum rejected');
    dies_like(sub { CodingAdventures::SkipList->new->range(1, undef) }, qr/maximum must be defined/, 'undefined maximum rejected');
    dies_like(sub { CodingAdventures::SkipList->new->range(1, 2, 'yes') }, qr/inclusive must be 0 or 1/, 'invalid inclusive flag rejected');
};

done_testing;
