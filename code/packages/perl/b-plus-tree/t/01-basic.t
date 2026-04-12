use strict;
use warnings;
use Test2::V0;
use CodingAdventures::BPlusTree;

# ============================================================================
# Helper: check is_valid after every mutation
# ============================================================================
sub check {
    my ($tree, $label) = @_;
    ok( $tree->is_valid, "is_valid: $label" );
}

# Helper: verify linked list is sorted and has the expected count
sub check_list {
    my ($tree, $label) = @_;
    my @all  = $tree->full_scan;
    my @keys = map { $_->[0] } @all;
    my $sorted = 1;
    for my $i (1 .. $#keys) {
        $sorted = 0, last if $keys[$i] <= $keys[$i - 1];
    }
    ok($sorted, "linked list sorted: $label");
    is( scalar @all, $tree->size, "list count == size: $label" );
}

# ============================================================================
# 1. Empty tree
# ============================================================================
subtest 'empty tree' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    is( $t->size,   0,     'size=0' );
    is( $t->height, 0,     'height=0' );
    is( $t->search(42), undef, 'search undef' );
    is( $t->min_key, undef, 'min_key undef' );
    is( $t->max_key, undef, 'max_key undef' );
    is( [ $t->full_scan ], [], 'full_scan empty' );
    is( [ $t->range_scan(1, 10) ], [], 'range_scan empty' );
    is( $t->delete(1), 0, 'delete returns 0' );
    check($t, 'empty');
};

# ============================================================================
# 2. Basic insert / search (t=2)
# ============================================================================
subtest 'basic insert and search t=2' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 2 );
    my @pairs = ([10,'ten'],[20,'twenty'],[5,'five'],[15,'fifteen'],[25,'twenty-five']);
    for my $p (@pairs) { $t->insert($p->[0], $p->[1]) }
    is( $t->size, 5, 'size=5' );
    for my $p (@pairs) {
        is( $t->search($p->[0]), $p->[1], "search $p->[0]" );
    }
    is( $t->search(99), undef, 'absent key' );
    is( $t->min_key, 5,  'min_key=5' );
    is( $t->max_key, 25, 'max_key=25' );
    check($t,      'after inserts');
    check_list($t, 'after inserts');
};

# ============================================================================
# 3. Upsert — inserting an existing key updates the value
# ============================================================================
subtest 'upsert' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert(1, 'one');
    $t->insert(1, 'ONE');
    is( $t->size,      1,     'size=1 after upsert' );
    is( $t->search(1), 'ONE', 'value updated' );
    check($t, 'upsert');
};

# ============================================================================
# 4. Inorder / full_scan is sorted
# ============================================================================
subtest 'full_scan sorted' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 3 );
    for my $k (50, 10, 80, 30, 60, 5, 25, 55, 75, 90) {
        $t->insert($k, $k * 2);
    }
    my @pairs = $t->full_scan;
    my @keys  = map { $_->[0] } @pairs;
    is( \@keys, [ sort { $a <=> $b } @keys ], 'full_scan is sorted' );
    for my $p (@pairs) { is( $p->[1], $p->[0] * 2, "value for $p->[0]" ) }
    check($t, 'full_scan');
    check_list($t, 'full_scan');
};

# ============================================================================
# 5. Range scan — uses the leaf linked list
# ============================================================================
subtest 'range_scan' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert($_, "v$_") for 1..20;

    my @r = $t->range_scan(5, 10);
    is( [ map { $_->[0] } @r ], [5..10], 'range 5..10 keys' );
    for my $pair (@r) {
        is( $pair->[1], "v$pair->[0]", "value for $pair->[0]" );
    }

    is( scalar $t->range_scan(100, 200), 0, 'empty range' );

    my @s = $t->range_scan(7, 7);
    is( scalar @s, 1,  'single-element range count' );
    is( $s[0][0],  7,  'single-element range key' );

    check($t,      'range_scan');
    check_list($t, 'range_scan');
};

# ============================================================================
# 6. Linked list integrity after inserts
# ============================================================================
subtest 'linked list integrity after inserts' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 2 );
    for my $k (30, 10, 50, 5, 20, 40, 60, 15, 25, 35, 45, 55) {
        $t->insert($k, $k);
        check($t,      "insert $k");
        check_list($t, "insert $k");
    }
};

# ============================================================================
# 7. Delete — basic leaf deletion
# ============================================================================
subtest 'delete leaf' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert($_, $_) for (10, 20, 5, 15, 25);
    is( $t->delete(5),  1, 'delete returns 1' );
    is( $t->size,       4, 'size decremented' );
    is( $t->search(5),  undef, 'deleted key absent' );
    is( $t->delete(99), 0, 'absent key returns 0' );
    check($t,      'delete leaf');
    check_list($t, 'delete leaf');
};

# ============================================================================
# 8. Delete until empty
# ============================================================================
subtest 'delete until empty' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert($_, $_) for 1..10;
    for my $k (1..10) {
        $t->delete($k);
        check($t,      "after delete $k");
        check_list($t, "after delete $k");
    }
    is( $t->size,    0,     'empty' );
    is( $t->min_key, undef, 'min_key undef' );
    is( $t->max_key, undef, 'max_key undef' );
};

# ============================================================================
# 9. Delete all cases (sequential 1..30 then specific order)
# ============================================================================
subtest 'delete all cases' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 2 );
    $t->insert($_, $_) for 1..30;
    check($t, 'after 30 inserts');

    for my $k (15, 1, 30, 10, 20, 5, 25, 8, 22, 3, 17, 28) {
        is( $t->delete($k), 1, "delete $k" );
        is( $t->search($k), undef, "key $k gone" );
        check($t,      "after delete $k");
        check_list($t, "after delete $k");
    }
    is( $t->size, 30 - 12, 'size correct after deletes' );
};

# ============================================================================
# 10. Large-scale — 500 keys, t=2
# ============================================================================
subtest 'large scale t=2' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 2 );
    my @keys = 1..500;
    for my $i (reverse 1..$#keys) {
        my $j = int rand($i + 1);
        @keys[$i, $j] = @keys[$j, $i];
    }
    $t->insert($_, $_ * 3) for @keys;
    is( $t->size, 500, 'size=500' );
    check($t,      '500 inserts');
    check_list($t, '500 inserts');
    is( $t->min_key, 1,   'min_key=1' );
    is( $t->max_key, 500, 'max_key=500' );

    for my $k (1..500) {
        is( $t->search($k), $k * 3, "search $k" );
    }

    $t->delete($_) for grep { $_ % 2 == 1 } 1..500;
    is( $t->size, 250, 'size=250 after deleting odds' );
    check($t,      'after deleting 250 keys');
    check_list($t, 'after deleting 250 keys');
};

# ============================================================================
# 11. Large-scale — 600 keys, t=3
# ============================================================================
subtest 'large scale t=3' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 3 );
    $t->insert($_, $_) for 1..600;
    is( $t->size, 600, 'size=600' );
    check($t, '600 inserts');

    my @all  = $t->full_scan;
    my @keys = map { $_->[0] } @all;
    is( \@keys, [1..600], 'full_scan=1..600' );

    my @r = $t->range_scan(200, 400);
    is( scalar @r,  201, 'range 200..400 has 201 items' );
    is( $r[0][0],   200, 'first item' );
    is( $r[-1][0],  400, 'last item' );

    $t->delete($_) for grep { $_ % 2 == 0 } 1..600;
    is( $t->size, 300, 'size=300' );
    check($t,      'after halving');
    check_list($t, 'after halving');
};

# ============================================================================
# 12. Large-scale — 750 keys, t=5
# ============================================================================
subtest 'large scale t=5' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 5 );
    $t->insert($_, "key$_") for 1..750;
    is( $t->size, 750, 'size=750' );
    check($t,      '750 inserts');
    check_list($t, '750 inserts');

    my @r = $t->range_scan(100, 200);
    is( scalar @r, 101, 'range 100..200 has 101 items' );
    is( $r[0][0],  100, 'first item' );
    is( $r[-1][0], 200, 'last item' );
};

# ============================================================================
# 13. Min / max after deletions
# ============================================================================
subtest 'min max' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert($_, $_) for (7, 3, 11, 1, 5, 9, 13);
    is( $t->min_key, 1,  'min=1'  );
    is( $t->max_key, 13, 'max=13' );
    $t->delete(1);
    is( $t->min_key, 3, 'min=3 after delete' );
    $t->delete(13);
    is( $t->max_key, 11, 'max=11 after delete' );
    check($t,      'min max');
    check_list($t, 'min max');
};

# ============================================================================
# 14. Reverse-order insert
# ============================================================================
subtest 'reverse order insert' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 3 );
    $t->insert($_, $_) for reverse 1..300;
    is( $t->size, 300, 'size=300' );
    check($t,      'reverse inserts');
    check_list($t, 'reverse inserts');
    my @keys = map { $_->[0] } $t->full_scan;
    is( \@keys, [1..300], 'full_scan sorted' );
};

# ============================================================================
# 15. Insert → delete all → re-insert
# ============================================================================
subtest 'insert delete reinsert' => sub {
    my $t = CodingAdventures::BPlusTree->new;
    $t->insert($_, $_) for 1..50;
    $t->delete($_) for 1..50;
    is( $t->size, 0, 'empty after deletions' );
    check($t,      'after all deletions');
    check_list($t, 'after all deletions');

    $t->insert($_, $_) for 51..100;
    is( $t->size, 50, 'size=50 after reinsert' );
    check($t,      'after reinsert');
    check_list($t, 'after reinsert');
    is( $t->min_key, 51,  'min=51' );
    is( $t->max_key, 100, 'max=100' );
};

# ============================================================================
# 16. Leaf linked list cross-leaf range
# ============================================================================
subtest 'cross-leaf range scan' => sub {
    my $t = CodingAdventures::BPlusTree->new( t => 2 );
    $t->insert($_ * 10, $_ * 10) for 1..20;
    my @r = $t->range_scan(30, 150);
    my @expected = map { $_ * 10 } grep { $_ >= 3 && $_ <= 15 } 1..20;
    is( [ map { $_->[0] } @r ], \@expected, 'cross-leaf range correct' );
    check($t, 'cross-leaf range');
};

done_testing;
