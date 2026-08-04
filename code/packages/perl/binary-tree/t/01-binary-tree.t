use strict;
use warnings;
use Test::More;
use CodingAdventures::BinaryTree;

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'empty and explicit construction' => sub {
    my $empty = CodingAdventures::BinaryTree->new;
    ok(!defined $empty->root, 'empty root');
    is($empty->height, -1, 'empty height');
    is($empty->size, 0, 'empty size');
    ok($empty->is_full, 'empty tree is full');
    ok($empty->is_complete, 'empty tree is complete');
    ok($empty->is_perfect, 'empty tree is perfect');
    is_deeply($empty->inorder, [], 'empty inorder');
    is_deeply($empty->level_order, [], 'empty level order');
    is_deeply($empty->to_array, [], 'empty array');
    is($empty->to_ascii, '', 'empty ASCII');
    is("$empty", 'BinaryTree(root=undef, size=0)', 'empty rendering');

    my $root = CodingAdventures::BinaryTree::Node->new(
        'root',
        CodingAdventures::BinaryTree::Node->new('left'),
        CodingAdventures::BinaryTree::Node->new('right'),
    );
    my $tree = CodingAdventures::BinaryTree->with_root($root);
    is($tree->root, $root, 'explicit root retained');
    is($tree->left_child('root')->value, 'left', 'left child lookup');
    is($tree->right_child('root')->value, 'right', 'right child lookup');

    my $singleton = CodingAdventures::BinaryTree->singleton('only');
    is($singleton->root->value, 'only', 'singleton value');
};

subtest 'construction validation' => sub {
    dies_like(
        sub { CodingAdventures::BinaryTree->new('root') },
        qr/root must be a CodingAdventures::BinaryTree::Node/,
        'invalid root rejected',
    );
    dies_like(
        sub { CodingAdventures::BinaryTree::Node->new(undef) },
        qr/node value must be defined/,
        'undefined node value rejected',
    );
    dies_like(
        sub { CodingAdventures::BinaryTree::Node->new(1, 'left') },
        qr/left must be a CodingAdventures::BinaryTree::Node/,
        'invalid child rejected',
    );
    dies_like(
        sub { CodingAdventures::BinaryTree->from_level_order('nope') },
        qr/values must be an array reference/,
        'invalid level-order input rejected',
    );
};

subtest 'traversals and lookup' => sub {
    my $tree = CodingAdventures::BinaryTree->from_level_order([1, 2, 3, 4, undef, 5, undef]);
    is_deeply($tree->level_order, [1, 2, 3, 4, 5], 'level order');
    is_deeply($tree->preorder, [1, 2, 4, 3, 5], 'preorder');
    is_deeply($tree->inorder, [4, 2, 1, 5, 3], 'inorder');
    is_deeply($tree->postorder, [4, 2, 5, 3, 1], 'postorder');
    is_deeply($tree->to_array, [1, 2, 3, 4, undef, 5, undef], 'sparse array');
    is($tree->find(5)->value, 5, 'find existing value');
    ok(!defined $tree->find(999), 'missing value');
};

subtest 'level-order round trip' => sub {
    my $values = [1, 2, 3, 4, 5, 6, 7];
    my $tree = CodingAdventures::BinaryTree->from_level_order($values);
    is_deeply($tree->to_array, $values, 'array round trip');
    is_deeply($tree->level_order, $values, 'level-order round trip');
};

subtest 'shape predicates' => sub {
    my $complete = CodingAdventures::BinaryTree->from_level_order([1, 2, undef]);
    ok(!$complete->is_full, 'single-child tree is not full');
    ok($complete->is_complete, 'left child preserves completeness');
    ok(!$complete->is_perfect, 'single-child tree is not perfect');
    is($complete->height, 1, 'height');
    is($complete->size, 2, 'size');
    is($complete->left_child(1)->value, 2, 'left child');
    ok(!defined $complete->right_child(1), 'missing right child');

    my $incomplete = CodingAdventures::BinaryTree->from_level_order([1, undef, 3]);
    ok(!$incomplete->is_complete, 'right-only child is incomplete');

    my $perfect = CodingAdventures::BinaryTree->from_level_order(['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    ok($perfect->is_full, 'perfect tree is full');
    ok($perfect->is_complete, 'perfect tree is complete');
    ok($perfect->is_perfect, 'perfect tree is perfect');
};

subtest 'ASCII and string rendering' => sub {
    my $tree = CodingAdventures::BinaryTree->from_level_order(['root', 'left', 'right']);
    like($tree->to_ascii, qr/`-- root/, 'root connector');
    like($tree->to_ascii, qr/left/, 'left value');
    like($tree->to_ascii, qr/right/, 'right value');
    is("$tree", 'BinaryTree(root=root, size=3)', 'tree string');
};

done_testing;
