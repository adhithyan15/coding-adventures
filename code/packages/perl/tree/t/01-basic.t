use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Tree; 1 }, 'CodingAdventures::Tree loads');

# ============================================================================
# BST — Binary Search Tree
# ============================================================================

subtest 'BST — basic insert and search' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    ok($bst, 'BST created');
    is($bst->size(), 0, 'empty BST size=0');

    $bst->insert(5);
    $bst->insert(3);
    $bst->insert(7);
    $bst->insert(1);
    $bst->insert(4);

    is($bst->size(), 5, 'BST size=5 after 5 inserts');
    is($bst->search(5), 1, 'search finds root');
    is($bst->search(1), 1, 'search finds leftmost leaf');
    is($bst->search(7), 1, 'search finds right child');
    is($bst->search(9), 0, 'search returns 0 for absent value');
    is($bst->search(0), 0, 'search returns 0 for absent value 0');
};

subtest 'BST — inorder returns sorted values' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    for my $v (5, 3, 7, 1, 4, 6, 8) {
        $bst->insert($v);
    }
    my @sorted = $bst->inorder();
    is(\@sorted, [1, 3, 4, 5, 6, 7, 8], 'inorder is sorted');
};

subtest 'BST — preorder and postorder' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5);
    $bst->insert(3);
    $bst->insert(7);
    $bst->insert(1);
    $bst->insert(4);

    # Preorder: Root-Left-Right: 5, 3, 1, 4, 7
    my @pre = $bst->preorder();
    is(\@pre, [5, 3, 1, 4, 7], 'preorder correct');

    # Postorder: Left-Right-Root: 1, 4, 3, 7, 5
    my @post = $bst->postorder();
    is(\@post, [1, 4, 3, 7, 5], 'postorder correct');
};

subtest 'BST — delete leaf node' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5); $bst->insert(3); $bst->insert(7);
    $bst->delete(3);
    is($bst->search(3), 0, 'deleted leaf not found');
    is($bst->search(5), 1, 'root still present');
    is($bst->size(), 2, 'size decremented after delete');
};

subtest 'BST — delete node with one child' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5); $bst->insert(3); $bst->insert(1);  # 3 has only left child
    $bst->delete(3);
    is($bst->search(3), 0, 'node with one child deleted');
    is($bst->search(1), 1, 'grandchild promoted');
    is($bst->search(5), 1, 'root intact');
};

subtest 'BST — delete node with two children' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5); $bst->insert(3); $bst->insert(7);
    $bst->insert(1); $bst->insert(4);
    # 3 has two children (1 and 4); inorder successor is 4
    $bst->delete(3);
    is($bst->search(3), 0, 'two-child node deleted');
    is($bst->search(1), 1, 'left grandchild still present');
    is($bst->search(4), 1, 'inorder successor still present');
    my @sorted = $bst->inorder();
    is(\@sorted, [1, 4, 5, 7], 'inorder still sorted after delete');
};

subtest 'BST — to_sorted_array' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(10); $bst->insert(2); $bst->insert(8);
    my $arr = $bst->to_sorted_array();
    is($arr, [2, 8, 10], 'to_sorted_array returns arrayref');
};

subtest 'BST — duplicate insert' => sub {
    my $bst = CodingAdventures::Tree::BST->new();
    $bst->insert(5); $bst->insert(5); $bst->insert(5);
    # Duplicates should be silently ignored; size stays 1
    is($bst->size(), 1, 'duplicate inserts do not grow tree');
    my @vals = $bst->inorder();
    is(\@vals, [5], 'only one copy of duplicate value');
};

# ============================================================================
# MinHeap
# ============================================================================

subtest 'MinHeap — empty heap' => sub {
    my $h = CodingAdventures::Tree::MinHeap->new();
    ok($h, 'MinHeap created');
    is($h->size(), 0, 'empty heap size=0');
    is($h->peek(), undef, 'peek on empty heap = undef');
    is($h->pop(),  undef, 'pop on empty heap = undef');
};

subtest 'MinHeap — push and peek' => sub {
    my $h = CodingAdventures::Tree::MinHeap->new();
    $h->push(5);
    is($h->peek(), 5, 'peek after one push');
    $h->push(1);
    is($h->peek(), 1, 'peek after inserting smaller value');
    $h->push(3);
    is($h->peek(), 1, 'peek still returns min');
    is($h->size(), 3, 'size=3');
};

subtest 'MinHeap — pop extracts minimum' => sub {
    my $h = CodingAdventures::Tree::MinHeap->new();
    for my $v (5, 1, 9, 3, 7, 2) {
        $h->push($v);
    }
    # Popping should give values in sorted ascending order
    my @extracted;
    while ($h->size() > 0) {
        push @extracted, $h->pop();
    }
    is(\@extracted, [1, 2, 3, 5, 7, 9], 'pop returns values in sorted order');
};

subtest 'MinHeap — heap sort equivalence' => sub {
    my $h = CodingAdventures::Tree::MinHeap->new();
    my @data = (42, 17, 3, 99, 8, 55);
    $h->push($_) for @data;
    my @sorted;
    push @sorted, $h->pop() while $h->size() > 0;
    is(\@sorted, [sort { $a <=> $b } @data], 'heap sort produces sorted array');
};

subtest 'MinHeap — single element' => sub {
    my $h = CodingAdventures::Tree::MinHeap->new();
    $h->push(42);
    is($h->pop(), 42, 'pop single element');
    is($h->size(), 0, 'empty after pop');
};

# ============================================================================
# Trie
# ============================================================================

subtest 'Trie — basic insert and search' => sub {
    my $t = CodingAdventures::Tree::Trie->new();
    ok($t, 'Trie created');

    $t->insert('cat');
    $t->insert('car');
    $t->insert('card');
    $t->insert('dog');

    is($t->search('cat'),  1, 'search finds "cat"');
    is($t->search('car'),  1, 'search finds "car"');
    is($t->search('card'), 1, 'search finds "card"');
    is($t->search('dog'),  1, 'search finds "dog"');
    is($t->search('ca'),   0, 'search: "ca" not inserted as full word');
    is($t->search('cats'), 0, 'search: "cats" not found');
    is($t->search('do'),   0, 'search: partial "do" not found');
};

subtest 'Trie — starts_with prefix search' => sub {
    my $t = CodingAdventures::Tree::Trie->new();
    $t->insert('apple');
    $t->insert('app');
    $t->insert('apply');
    $t->insert('banana');

    is($t->starts_with('app'),    1, '"app" prefix exists');
    is($t->starts_with('appl'),   1, '"appl" prefix exists');
    is($t->starts_with('apple'),  1, '"apple" itself is a prefix');
    is($t->starts_with('ban'),    1, '"ban" prefix exists');
    is($t->starts_with('xyz'),    0, '"xyz" prefix absent');
    is($t->starts_with('applez'), 0, 'longer-than-word prefix absent');
};

subtest 'Trie — empty string' => sub {
    my $t = CodingAdventures::Tree::Trie->new();
    $t->insert('');
    is($t->search(''),      1, 'empty string found after insert');
    is($t->starts_with(''), 1, 'empty prefix always matches');
};

subtest 'Trie — no words inserted' => sub {
    my $t = CodingAdventures::Tree::Trie->new();
    is($t->search('anything'),      0, 'search on empty trie=0');
    is($t->starts_with('anything'), 0, 'starts_with on empty trie=0');
    is($t->starts_with(''),         1, 'empty prefix on empty trie=1');
};

done_testing;
