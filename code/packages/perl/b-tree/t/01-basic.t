use strict;
use warnings;
use Test2::V0;
use CodingAdventures::BTree;

# ============================================================================
# Helper: check is_valid after every mutation
# ============================================================================
sub check {
    my ($tree, $label) = @_;
    ok( $tree->is_valid, "is_valid: $label" );
}

# ============================================================================
# 1. Empty tree
# ============================================================================
subtest 'empty tree' => sub {
    my $t = CodingAdventures::BTree->new;
    is( $t->size,   0,     'size=0' );
    is( $t->height, 0,     'height=0' );
    is( $t->search('x'), undef, 'search returns undef' );
    is( $t->min_key, undef, 'min_key undef' );
    is( $t->max_key, undef, 'max_key undef' );
    is( [ $t->inorder ], [], 'inorder empty' );
    is( $t->delete('x'), 0, 'delete returns 0' );
    check($t, 'empty');
};

# ============================================================================
# 2. Basic insert / search (t=2)
# ============================================================================
subtest 'basic insert and search t=2' => sub {
    my $t = CodingAdventures::BTree->new( t => 2 );
    my @pairs = ([10,'ten'],[20,'twenty'],[5,'five'],[15,'fifteen'],[25,'twenty-five']);
    for my $p (@pairs) { $t->insert($p->[0], $p->[1]) }
    is( $t->size, 5, 'size=5' );
    for my $p (@pairs) {
        is( $t->search($p->[0]), $p->[1], "search $p->[0]" );
    }
    is( $t->search(99), undef, 'absent key' );
    is( $t->min_key, 5,  'min_key' );
    is( $t->max_key, 25, 'max_key' );
    check($t, 'after inserts');
};

# ============================================================================
# 3. Upsert
# ============================================================================
subtest 'upsert' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert(1, 'one');
    $t->insert(1, 'ONE');
    is( $t->size,   1,     'size=1 after upsert' );
    is( $t->search(1), 'ONE', 'value updated' );
    check($t, 'upsert');
};

# ============================================================================
# 4. In-order traversal is sorted
# ============================================================================
subtest 'inorder sorted' => sub {
    my $t = CodingAdventures::BTree->new( t => 3 );
    for my $k (50, 10, 80, 30, 60, 5, 25, 55, 75, 90) {
        $t->insert($k, $k * 2);
    }
    my @pairs = $t->inorder;
    my @keys  = map { $_->[0] } @pairs;
    is( \@keys, [ sort { $a <=> $b } @keys ], 'inorder is sorted' );
    for my $p (@pairs) { is( $p->[1], $p->[0] * 2, "value for $p->[0]" ) }
    check($t, 'inorder');
};

# ============================================================================
# 5. Range query
# ============================================================================
subtest 'range_query' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert($_, "v$_") for 1..20;
    my @r = $t->range_query(5, 10);
    is( [ map { $_->[0] } @r ], [5..10], 'range 5..10' );
    is( scalar $t->range_query(100, 200), 0, 'empty range' );
    my @s = $t->range_query(7, 7);
    is( $s[0][0], 7, 'single-element range' );
    check($t, 'range_query');
};

# ============================================================================
# 6. Delete — leaf (Case A)
# ============================================================================
subtest 'delete leaf' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert($_, $_) for (10, 20, 5, 15, 25);
    is( $t->delete(5), 1, 'delete returns 1' );
    is( $t->size, 4, 'size decremented' );
    is( $t->search(5), undef, 'deleted key absent' );
    is( $t->delete(99), 0, 'absent key returns 0' );
    check($t, 'delete leaf');
};

# ============================================================================
# 7. Delete — internal node
# ============================================================================
subtest 'delete internal node' => sub {
    my $t = CodingAdventures::BTree->new( t => 2 );
    $t->insert($_, $_) for 1..15;
    check($t, 'before internal delete');
    is( $t->delete(8), 1, 'delete 8' );
    is( $t->search(8), undef, 'key 8 gone' );
    is( $t->size, 14, 'size=14' );
    check($t, 'after internal delete');
    $t->delete(4); $t->delete(12);
    check($t, 'after more deletes');
};

# ============================================================================
# 8. Delete — all cases (sequential 1..30 then specific order)
# ============================================================================
subtest 'delete all cases' => sub {
    my $t = CodingAdventures::BTree->new( t => 2 );
    $t->insert($_, $_) for 1..30;
    check($t, 'after 30 inserts');

    for my $k (15, 1, 30, 10, 20, 5, 25, 8, 22, 3, 17, 28) {
        is( $t->delete($k), 1, "delete $k" );
        is( $t->search($k), undef, "key $k gone" );
        check($t, "after delete $k");
    }
    is( $t->size, 30 - 12, 'size correct after deletes' );
};

# ============================================================================
# 9. Delete until empty
# ============================================================================
subtest 'delete until empty' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert($_, $_) for 1..10;
    for my $k (1..10) {
        $t->delete($k);
        check($t, "after delete $k");
    }
    is( $t->size, 0, 'empty' );
    is( $t->min_key, undef, 'min_key undef' );
    is( $t->max_key, undef, 'max_key undef' );
};

# ============================================================================
# 10. Large-scale — 500 keys, t=2
# ============================================================================
subtest 'large scale t=2' => sub {
    my $t = CodingAdventures::BTree->new( t => 2 );
    # Insert in shuffled order using Fisher-Yates.
    my @keys = 1..500;
    for my $i (reverse 1..$#keys) {
        my $j = int rand($i + 1);
        @keys[$i, $j] = @keys[$j, $i];
    }
    $t->insert($_, $_ * 3) for @keys;
    is( $t->size, 500, 'size=500' );
    check($t, '500 inserts');
    is( $t->min_key, 1,   'min_key=1' );
    is( $t->max_key, 500, 'max_key=500' );

    # Verify every key findable.
    for my $k (1..500) {
        is( $t->search($k), $k * 3, "search $k" );
    }

    # Delete odd keys.
    $t->delete($_) for grep { $_ % 2 == 1 } 1..500;
    is( $t->size, 250, 'size=250 after deleting odds' );
    check($t, 'after deleting 250 keys');
};

# ============================================================================
# 11. Large-scale — 600 keys, t=3
# ============================================================================
subtest 'large scale t=3' => sub {
    my $t = CodingAdventures::BTree->new( t => 3 );
    $t->insert($_, $_) for 1..600;
    is( $t->size, 600, 'size=600' );
    check($t, '600 inserts');

    my @inord = map { $_->[0] } $t->inorder;
    is( \@inord, [1..600], 'inorder=1..600' );

    $t->delete($_) for grep { $_ % 2 == 0 } 1..600;
    is( $t->size, 300, 'size=300' );
    check($t, 'after halving');
};

# ============================================================================
# 12. Large-scale — 750 keys, t=5
# ============================================================================
subtest 'large scale t=5' => sub {
    my $t = CodingAdventures::BTree->new( t => 5 );
    $t->insert($_, "key$_") for 1..750;
    is( $t->size, 750, 'size=750' );
    check($t, '750 inserts');

    my @r = $t->range_query(100, 200);
    is( scalar @r, 101, 'range 100..200 has 101 items' );
    is( $r[0][0],  100, 'first item' );
    is( $r[-1][0], 200, 'last item' );
};

# ============================================================================
# 13. Min / max after deletions
# ============================================================================
subtest 'min max' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert($_, $_) for (7, 3, 11, 1, 5, 9, 13);
    is( $t->min_key, 1,  'min=1'  );
    is( $t->max_key, 13, 'max=13' );
    $t->delete(1);
    is( $t->min_key, 3, 'min=3 after delete' );
    $t->delete(13);
    is( $t->max_key, 11, 'max=11 after delete' );
    check($t, 'min max');
};

# ============================================================================
# 14. Reverse-order insert
# ============================================================================
subtest 'reverse order insert' => sub {
    my $t = CodingAdventures::BTree->new( t => 3 );
    $t->insert($_, $_) for reverse 1..300;
    is( $t->size, 300, 'size=300' );
    check($t, 'reverse inserts');
    my @inord = map { $_->[0] } $t->inorder;
    is( \@inord, [1..300], 'inorder sorted' );
};

# ============================================================================
# 15. Insert → delete all → re-insert
# ============================================================================
subtest 'insert delete reinsert' => sub {
    my $t = CodingAdventures::BTree->new;
    $t->insert($_, $_) for 1..50;
    $t->delete($_) for 1..50;
    is( $t->size, 0, 'empty after deletions' );
    check($t, 'after all deletions');
    $t->insert($_, $_) for 51..100;
    is( $t->size, 50, 'size=50 after reinsert' );
    check($t, 'after reinsert');
    is( $t->min_key, 51,  'min=51' );
    is( $t->max_key, 100, 'max=100' );
};

done_testing;
