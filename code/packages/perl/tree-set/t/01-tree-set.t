use strict;
use warnings;
use Test::More;
use CodingAdventures::TreeSet;

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'deduplication, ordering, mutation, and backend' => sub {
    my $set = CodingAdventures::TreeSet->new([5, 1, 3, 3, 9]);
    is($set->add(7), $set, 'add chains');

    is_deeply($set->to_sorted_array, [1, 3, 5, 7, 9], 'sorted unique values');
    is_deeply($set->to_list, [1, 3, 5, 7, 9], 'list alias');
    is($set->size, 5, 'size');
    is($set->length, 5, 'length');
    ok($set->contains(7), 'contains value');
    ok($set->has(3), 'has alias');
    ok(!$set->has(2), 'absent value');
    is("$set", 'TreeSet([1, 3, 5, 7, 9])', 'string rendering');
    ok($set->backend->is_valid_avl, 'AVL backend valid');

    ok($set->delete(3), 'delete existing value');
    ok($set->remove(5), 'remove alias');
    ok(!$set->discard(99), 'discard absent value');
    ok(!$set->delete(undef), 'undefined delete absent');
    is_deeply($set->to_array, [1, 7, 9], 'delete results');
    ok($set->backend->is_valid_avl, 'backend valid after deletes');
};

subtest 'boundaries, rank, and selection' => sub {
    my $set = CodingAdventures::TreeSet->from_values([10, 20, 30, 40]);

    is($set->min, 10, 'minimum');
    is($set->max, 40, 'maximum');
    is($set->first, 10, 'first');
    is($set->last, 40, 'last');
    is($set->predecessor(30), 20, 'predecessor');
    is($set->successor(30), 40, 'successor');
    is($set->rank(5), 0, 'rank before minimum');
    is($set->rank(25), 2, 'rank between values');
    is($set->by_rank(0), 10, 'zero-based selection');
    is($set->by_rank(3), 40, 'last rank');
    ok(!defined $set->by_rank(-1), 'negative rank');
    is($set->kth_smallest(3), 30, 'one-based selection');
    ok(!defined $set->kth_smallest(0), 'zero k');
};

subtest 'empty and defensive arrays' => sub {
    my $set = CodingAdventures::TreeSet->empty;
    ok($set->is_empty, 'empty set');
    ok(!defined $set->min, 'empty minimum');
    ok(!defined $set->max, 'empty maximum');
    ok(!defined $set->first, 'empty first');
    ok(!defined $set->last, 'empty last');
    ok(!defined $set->predecessor(1), 'empty predecessor');
    ok(!defined $set->successor(1), 'empty successor');
    ok(!defined $set->by_rank(0), 'empty rank selection');
    is($set->rank(1), 0, 'empty rank');
    is("$set", 'TreeSet([])', 'empty rendering');

    my $populated = CodingAdventures::TreeSet->from_values([1, 2]);
    my $snapshot = $populated->to_list;
    $snapshot->[0] = 999;
    is_deeply($populated->to_list, [1, 2], 'returned arrays are defensive');
};

subtest 'inclusive and exclusive ranges' => sub {
    my $set = CodingAdventures::TreeSet->from_values([1, 3, 5, 7, 9]);
    is_deeply($set->range(3, 7), [3, 5, 7], 'inclusive range');
    is_deeply($set->range(3, 7, 0), [5], 'exclusive range');
    is_deeply($set->range(10, 20), [], 'range above maximum');
    is_deeply($set->range(7, 3), [], 'reversed range');
};

subtest 'set algebra does not mutate inputs' => sub {
    my $left = CodingAdventures::TreeSet->from_values([1, 2, 3, 5]);
    my $right = CodingAdventures::TreeSet->from_values([3, 4, 5, 6]);

    is_deeply($left->union($right)->to_list, [1, 2, 3, 4, 5, 6], 'union');
    is_deeply($left->intersection($right)->to_list, [3, 5], 'intersection');
    is_deeply($left->difference($right)->to_list, [1, 2], 'difference');
    is_deeply($left->symmetric_difference($right)->to_list, [1, 2, 4, 6], 'symmetric difference');
    is_deeply($left->to_list, [1, 2, 3, 5], 'left input unchanged');
    is_deeply($right->to_list, [3, 4, 5, 6], 'right input unchanged');
};

subtest 'set predicates and equality' => sub {
    my $small = CodingAdventures::TreeSet->from_values([2, 3]);
    my $large = CodingAdventures::TreeSet->from_values([1, 2, 3, 4]);
    my $disjoint = CodingAdventures::TreeSet->from_values([8, 9]);

    ok($small->is_subset($large), 'subset');
    ok($large->is_superset($small), 'superset');
    ok($small->is_disjoint($disjoint), 'disjoint');
    ok(!$small->is_disjoint($large), 'overlap detected');
    ok($small->equals(CodingAdventures::TreeSet->from_values([3, 2])), 'content equality');
    ok(!$small->equals($large), 'different sizes unequal');
    ok(!$small->equals([2, 3]), 'different type unequal');
};

subtest 'custom comparator' => sub {
    my $by_length = sub {
        return length($_[0]) <=> length($_[1]) || $_[0] cmp $_[1];
    };
    my $set = CodingAdventures::TreeSet->new([], $by_length)
        ->add('banana')->add('fig')->add('apple');

    is_deeply($set->to_list, ['fig', 'apple', 'banana'], 'custom order');
    ok($set->backend->is_valid_avl, 'custom backend valid');
    is($set->by_rank(1), 'apple', 'custom rank selection');
};

subtest 'larger mutation sequence preserves AVL invariants' => sub {
    my $set = CodingAdventures::TreeSet->empty;
    my $valid = 1;
    for my $index (1 .. 100) {
        $set->add(($index * 37) % 101);
        $valid &&= $set->backend->is_valid_avl;
    }
    ok($valid, 'all inserts preserve invariants');
    is($set->size, 100, 'all values inserted');

    for my $value (grep { $_ % 2 == 1 } 1 .. 99) {
        $valid &&= $set->delete($value);
        $valid &&= $set->backend->is_valid_avl;
    }
    ok($valid, 'all deletes preserve invariants');
    is_deeply($set->to_list, [grep { $_ % 2 == 0 } 1 .. 100], 'even values remain');
};

subtest 'public input validation' => sub {
    dies_like(sub { CodingAdventures::TreeSet->new('values') }, qr/values must be an array reference/, 'invalid values rejected');
    dies_like(sub { CodingAdventures::TreeSet->new([], 'compare') }, qr/compare must be a code reference/, 'invalid comparator rejected');
    dies_like(sub { CodingAdventures::TreeSet->empty->add(undef) }, qr/value must be defined/, 'undefined add rejected');
    dies_like(sub { CodingAdventures::TreeSet->empty->range(undef, 2) }, qr/minimum must be defined/, 'undefined minimum rejected');
    dies_like(sub { CodingAdventures::TreeSet->empty->range(1, undef) }, qr/maximum must be defined/, 'undefined maximum rejected');
    dies_like(sub { CodingAdventures::TreeSet->empty->range(1, 2, 'yes') }, qr/inclusive must be 0 or 1/, 'invalid inclusive flag rejected');
    dies_like(sub { CodingAdventures::TreeSet->empty->union([]) }, qr/other must be a CodingAdventures::TreeSet/, 'invalid other set rejected');
};

done_testing;
