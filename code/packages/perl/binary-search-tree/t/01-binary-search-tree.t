use strict;
use warnings;
use Test::More;
use CodingAdventures::BinarySearchTree;

sub populated {
    my $tree = CodingAdventures::BinarySearchTree->empty;
    $tree = $tree->insert($_) for (5, 1, 8, 3, 7);
    return $tree;
}

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'persistent insert, search, rank, and delete' => sub {
    my $tree = populated();
    is_deeply($tree->to_sorted_array, [1, 3, 5, 7, 8], 'sorted values');
    is($tree->size, 5, 'size');
    ok($tree->contains(7), 'contains value');
    is($tree->search(7)->value, 7, 'search value');
    is($tree->min_value, 1, 'minimum');
    is($tree->max_value, 8, 'maximum');
    is($tree->predecessor(5), 3, 'predecessor');
    is($tree->successor(5), 7, 'successor');
    is($tree->rank(4), 2, 'rank');
    is($tree->kth_smallest(4), 7, 'k-th smallest');

    my $deleted = $tree->delete(5);
    ok(!$deleted->contains(5), 'delete removes value');
    ok($deleted->is_valid, 'deleted tree is valid');
    ok($tree->contains(5), 'original remains unchanged');

    my $duplicate = $tree->insert(3);
    is_deeply($duplicate->to_sorted_array, $tree->to_sorted_array, 'duplicate ignored');
    is($duplicate->search(3), $tree->search(3), 'duplicate node is shared');
};

subtest 'balanced and empty construction' => sub {
    my $values = [1, 2, 3, 4, 5, 6, 7];
    my $tree = CodingAdventures::BinarySearchTree->from_sorted_array($values);
    is_deeply($tree->to_sorted_array, $values, 'sorted array round trip');
    is($tree->root->value, 4, 'middle root');
    is($tree->height, 2, 'balanced height');
    is($tree->size, 7, 'balanced size');
    ok($tree->is_valid, 'balanced tree valid');

    my $empty = CodingAdventures::BinarySearchTree->empty;
    ok(!defined $empty->search(1), 'empty search');
    ok(!defined $empty->min_value, 'empty minimum');
    ok(!defined $empty->max_value, 'empty maximum');
    ok(!defined $empty->predecessor(1), 'empty predecessor');
    ok(!defined $empty->successor(1), 'empty successor');
    ok(!defined $empty->kth_smallest(0), 'zero k');
    ok(!defined $empty->kth_smallest(1), 'empty k');
    is($empty->rank(1), 0, 'empty rank');
    is($empty->height, -1, 'empty height');
    is($empty->size, 0, 'empty size');
    ok($empty->is_valid, 'empty valid');
    is("$empty", 'BinarySearchTree(root=undef, size=0)', 'empty rendering');
};

subtest 'construction validation' => sub {
    dies_like(sub { CodingAdventures::BinarySearchTree::Node->new(undef) }, qr/node value must be defined/, 'undefined node value rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree::Node->new(1, 'left') }, qr/left must be a CodingAdventures/, 'invalid child rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree::Node->new(1, undef, undef, -1) }, qr/size must be a non-negative integer/, 'invalid size rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree->new('root') }, qr/root must be a CodingAdventures/, 'invalid root rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree->new(undef, 'compare') }, qr/compare must be a code reference/, 'invalid comparator rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree->from_sorted_array('values') }, qr/array reference/, 'invalid array rejected');
    dies_like(sub { CodingAdventures::BinarySearchTree->empty->insert(undef) }, qr/value must be defined/, 'undefined insert rejected');
};

subtest 'deletion cases' => sub {
    my $tree = CodingAdventures::BinarySearchTree->from_sorted_array([2, 4, 6, 8]);
    is($tree->root->value, 6, 'upper middle root');
    is_deeply($tree->delete(2)->to_sorted_array, [4, 6, 8], 'delete leaf');
    is_deeply($tree->delete(8)->to_sorted_array, [2, 4, 6], 'delete one-child branch');
    is_deeply($tree->delete(99)->to_sorted_array, [2, 4, 6, 8], 'delete absent value');
};

subtest 'validation catches corruption' => sub {
    my $bad_order = CodingAdventures::BinarySearchTree->new(
        CodingAdventures::BinarySearchTree::Node->new(5, CodingAdventures::BinarySearchTree::Node->new(6)),
    );
    my $bad_size = CodingAdventures::BinarySearchTree->new(
        CodingAdventures::BinarySearchTree::Node->new(5, CodingAdventures::BinarySearchTree::Node->new(3), undef, 99),
    );
    ok(!$bad_order->is_valid, 'ordering corruption');
    ok(!$bad_size->is_valid, 'size corruption');
};

subtest 'custom comparator' => sub {
    my $by_length = sub { return length($_[0]) <=> length($_[1]); };
    my $tree = CodingAdventures::BinarySearchTree->empty($by_length)
        ->insert('bbb')->insert('a')->insert('cc');
    is_deeply($tree->to_sorted_array, ['a', 'cc', 'bbb'], 'length order');
    ok($tree->contains('zz'), 'comparator equality');
    is($tree->predecessor('zz'), 'a', 'custom predecessor');
    is($tree->successor('zz'), 'bbb', 'custom successor');
};

done_testing;
