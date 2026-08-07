use strict;
use warnings;
use Test::More;
use CodingAdventures::AVLTree;

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'left and right rotations' => sub {
    my $right_heavy = CodingAdventures::AVLTree->from_values([10, 20, 30]);
    my $left_heavy = CodingAdventures::AVLTree->from_values([30, 20, 10]);

    is($right_heavy->root->value, 20, 'left rotation root');
    is($left_heavy->root->value, 20, 'right rotation root');
    is_deeply($right_heavy->to_sorted_array, [10, 20, 30], 'sorted values');
    ok($right_heavy->is_valid_bst, 'valid BST');
    ok($right_heavy->is_valid_avl, 'valid AVL tree');
    is($right_heavy->height, 1, 'height');
    is($right_heavy->size, 3, 'size');
    is($right_heavy->balance_factor($right_heavy->root), 0, 'root balance factor');
};

subtest 'double rotations' => sub {
    my $left_right = CodingAdventures::AVLTree->from_values([30, 10, 20]);
    my $right_left = CodingAdventures::AVLTree->from_values([10, 30, 20]);

    is($left_right->root->value, 20, 'left-right rotation root');
    is($right_left->root->value, 20, 'right-left rotation root');
    ok($left_right->is_valid_avl, 'left-right result valid');
    ok($right_left->is_valid_avl, 'right-left result valid');
};

subtest 'persistent queries and deletion' => sub {
    my $tree = CodingAdventures::AVLTree->from_values([40, 20, 60, 10, 30, 50, 70]);

    is($tree->search(20)->value, 20, 'search');
    ok($tree->contains(50), 'contains');
    is($tree->min_value, 10, 'minimum');
    is($tree->max_value, 70, 'maximum');
    is($tree->predecessor(40), 30, 'predecessor');
    is($tree->successor(40), 50, 'successor');
    is($tree->kth_smallest(4), 40, 'k-th smallest');
    is($tree->rank(35), 3, 'rank');

    my $deleted = $tree->delete(20);
    ok(!$deleted->contains(20), 'delete removes value');
    ok($deleted->is_valid_avl, 'deleted tree valid');
    ok($tree->contains(20), 'original remains unchanged');
};

subtest 'empty, duplicate, and absent cases' => sub {
    my $empty = CodingAdventures::AVLTree->empty;
    ok(!defined $empty->search(1), 'empty search');
    ok(!defined $empty->min_value, 'empty minimum');
    ok(!defined $empty->max_value, 'empty maximum');
    ok(!defined $empty->predecessor(1), 'empty predecessor');
    ok(!defined $empty->successor(1), 'empty successor');
    ok(!defined $empty->kth_smallest(0), 'zero k');
    is($empty->rank(1), 0, 'empty rank');
    is($empty->balance_factor(undef), 0, 'empty balance factor');
    is($empty->height, -1, 'empty height');
    is($empty->size, 0, 'empty size');
    ok($empty->is_valid_avl, 'empty valid');
    is("$empty", 'AVLTree(root=undef, height=-1, size=0)', 'empty rendering');

    my $tree = CodingAdventures::AVLTree->from_values([30, 20, 40, 10, 25, 35, 50]);
    is_deeply($tree->insert(25)->to_sorted_array, $tree->to_sorted_array, 'duplicate ignored');
    is_deeply($tree->delete(999)->to_sorted_array, $tree->to_sorted_array, 'absent delete unchanged');
};

subtest 'nested successor deletion' => sub {
    my $tree = CodingAdventures::AVLTree->from_values([5, 3, 8, 7, 9, 6]);
    my $deleted = $tree->delete(5);

    is_deeply($deleted->to_sorted_array, [3, 6, 7, 8, 9], 'nested successor chosen');
    ok($deleted->is_valid_avl, 'nested successor result valid');
    is($tree->kth_smallest(1), 3, 'first order statistic');
    is($tree->kth_smallest(6), 9, 'last order statistic');
    is($tree->rank(5), 1, 'absent rank');
    is_deeply(CodingAdventures::AVLTree->from_values([1, 2])->delete(1)->to_sorted_array, [2], 'delete root with right child');
    is_deeply(CodingAdventures::AVLTree->from_values([2, 1])->delete(2)->to_sorted_array, [1], 'delete root with left child');
};

subtest 'larger update sequence' => sub {
    my $tree = CodingAdventures::AVLTree->empty;
    for my $index (1 .. 100) {
        $tree = $tree->insert(($index * 37) % 101);
        ok($tree->is_valid_avl, "insert $index preserves invariants");
    }
    is($tree->size, 100, 'all values inserted');

    my $updated = $tree;
    for my $value (grep { $_ % 2 == 1 } 1 .. 99) {
        $updated = $updated->delete($value);
        ok($updated->is_valid_avl, "delete $value preserves invariants");
    }
    is_deeply($updated->to_sorted_array, [grep { $_ % 2 == 0 } 1 .. 100], 'even values remain');
    is($tree->size, 100, 'original remains unchanged');
};

subtest 'validation catches corruption' => sub {
    my $bad_order = CodingAdventures::AVLTree->new(
        CodingAdventures::AVLTree::Node->new(5, CodingAdventures::AVLTree::Node->new(6), undef, 1, 2),
    );
    my $bad_right_order = CodingAdventures::AVLTree->new(
        CodingAdventures::AVLTree::Node->new(5, undef, CodingAdventures::AVLTree::Node->new(4), 1, 2),
    );
    my $bad_height = CodingAdventures::AVLTree->new(
        CodingAdventures::AVLTree::Node->new(5, CodingAdventures::AVLTree::Node->new(3), undef, 99, 2),
    );
    my $bad_size = CodingAdventures::AVLTree->new(
        CodingAdventures::AVLTree::Node->new(5, CodingAdventures::AVLTree::Node->new(3), undef, 1, 99),
    );
    my $unbalanced_left = CodingAdventures::AVLTree::Node->new(
        2,
        CodingAdventures::AVLTree::Node->new(1),
    );
    my $unbalanced = CodingAdventures::AVLTree->new(
        CodingAdventures::AVLTree::Node->new(3, $unbalanced_left),
    );

    ok(!$bad_order->is_valid_bst, 'left ordering corruption');
    ok(!$bad_order->is_valid_avl, 'left ordering invalid AVL');
    ok(!$bad_right_order->is_valid_bst, 'right ordering corruption');
    ok(!$bad_right_order->is_valid_avl, 'right ordering invalid AVL');
    ok(!$bad_height->is_valid_avl, 'height corruption');
    ok(!$bad_size->is_valid_avl, 'size corruption');
    ok(!$unbalanced->is_valid_avl, 'balance corruption');
};

subtest 'custom comparator' => sub {
    my $by_length = sub { return length($_[0]) <=> length($_[1]); };
    my $tree = CodingAdventures::AVLTree->from_values(['bbb', 'a', 'cc'], $by_length);

    is_deeply($tree->to_sorted_array, ['a', 'cc', 'bbb'], 'length order');
    ok($tree->contains('zz'), 'comparator equality');
    is($tree->predecessor('zz'), 'a', 'custom predecessor');
    is($tree->successor('zz'), 'bbb', 'custom successor');
    ok($tree->is_valid_avl, 'custom tree valid');
};

subtest 'construction validation' => sub {
    dies_like(sub { CodingAdventures::AVLTree::Node->new(undef) }, qr/node value must be defined/, 'undefined node value rejected');
    dies_like(sub { CodingAdventures::AVLTree::Node->new(1, 'left') }, qr/left must be a CodingAdventures/, 'invalid child rejected');
    dies_like(sub { CodingAdventures::AVLTree::Node->new(1, undef, undef, -1) }, qr/height must be a non-negative integer/, 'invalid height rejected');
    dies_like(sub { CodingAdventures::AVLTree::Node->new(1, undef, undef, undef, -1) }, qr/size must be a non-negative integer/, 'invalid size rejected');
    dies_like(sub { CodingAdventures::AVLTree->new('root') }, qr/root must be a CodingAdventures/, 'invalid root rejected');
    dies_like(sub { CodingAdventures::AVLTree->new(undef, 'compare') }, qr/compare must be a code reference/, 'invalid comparator rejected');
    dies_like(sub { CodingAdventures::AVLTree->from_values('values') }, qr/array reference/, 'invalid values rejected');
    dies_like(sub { CodingAdventures::AVLTree->empty->insert(undef) }, qr/value must be defined/, 'undefined insert rejected');
    dies_like(sub { CodingAdventures::AVLTree->empty->delete(undef) }, qr/value must be defined/, 'undefined delete rejected');
    dies_like(sub { CodingAdventures::AVLTree->empty->balance_factor('node') }, qr/node must be a CodingAdventures/, 'invalid balance node rejected');
};

done_testing;
