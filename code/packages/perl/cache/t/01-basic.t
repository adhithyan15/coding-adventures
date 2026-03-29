use strict;
use warnings;
use Test2::V0;

# Load the module
ok( eval { require CodingAdventures::Cache; 1 }, 'Cache module loads' )
    or diag($@);

# ============================================================================
# LRUCache tests
# ============================================================================

my $LRU = 'CodingAdventures::Cache::LRUCache';

# Test 1: LRUCache constructs
{
    my $lru = $LRU->new(capacity => 3);
    ok( defined $lru,             'LRUCache->new constructs' );
    is( $lru->capacity, 3,        'LRUCache capacity is set correctly' );
    is( $lru->size,     0,        'LRUCache starts empty' );
}

# Test 2: put and get basic
{
    my $lru = $LRU->new(capacity => 3);
    $lru->put('a', 1);
    $lru->put('b', 2);
    is( $lru->get('a'), 1, 'get returns correct value for a' );
    is( $lru->get('b'), 2, 'get returns correct value for b' );
}

# Test 3: get non-existent key returns undef
{
    my $lru = $LRU->new(capacity => 3);
    is( $lru->get('z'), undef, 'get for non-existent key returns undef' );
}

# Test 4: hit and miss counting
{
    my $lru = $LRU->new(capacity => 3);
    $lru->put('a', 1);
    $lru->get('a');   # hit
    $lru->get('b');   # miss
    is( $lru->hits,   1, 'hit counter increments on cache hit' );
    is( $lru->misses, 1, 'miss counter increments on cache miss' );
}

# Test 5: LRU eviction
{
    my $lru = $LRU->new(capacity => 2);
    $lru->put('a', 1);
    $lru->put('b', 2);
    $lru->put('c', 3);   # should evict 'a' (least recently used)
    is( $lru->get('a'), undef, 'LRU entry a was evicted' );
    is( $lru->get('b'), 2,     'b is still present' );
    is( $lru->get('c'), 3,     'c is present' );
}

# Test 6: get promotes to MRU, preventing eviction
{
    my $lru = $LRU->new(capacity => 2);
    $lru->put('a', 1);
    $lru->put('b', 2);
    $lru->get('a');       # a becomes MRU; b is now LRU
    $lru->put('c', 3);    # should evict b (LRU), not a
    is( $lru->get('a'), 1,     'a was not evicted (was promoted to MRU)' );
    is( $lru->get('b'), undef, 'b was evicted (was LRU after a was accessed)' );
    is( $lru->get('c'), 3,     'c is present' );
}

# Test 7: put updates existing key
{
    my $lru = $LRU->new(capacity => 3);
    $lru->put('a', 1);
    $lru->put('a', 99);    # update value
    is( $lru->get('a'), 99, 'put on existing key updates value' );
    is( $lru->size,     1,  'size does not grow on key update' );
}

# Test 8: size tracking
{
    my $lru = $LRU->new(capacity => 5);
    $lru->put('x', 1);
    $lru->put('y', 2);
    is( $lru->size, 2, 'size tracks number of entries' );
}

# Test 9: capacity=1 evicts immediately
{
    my $lru = $LRU->new(capacity => 1);
    $lru->put('a', 1);
    $lru->put('b', 2);   # evicts a
    is( $lru->get('a'), undef, 'capacity=1: a evicted after b inserted' );
    is( $lru->get('b'), 2,     'capacity=1: b is present' );
}

# Test 10: keys_list order (MRU first)
{
    my $lru = $LRU->new(capacity => 3);
    $lru->put('a', 1);
    $lru->put('b', 2);
    $lru->put('c', 3);
    my @keys = $lru->keys_list();
    is( $keys[0], 'c', 'most recently put is first in keys_list' );
    is( $keys[2], 'a', 'oldest entry is last in keys_list' );
}

# ============================================================================
# DirectMappedCache tests
# ============================================================================

my $DM = 'CodingAdventures::Cache::DirectMappedCache';

# Test 11: DirectMappedCache constructs
{
    my $dm = $DM->new(sets => 4);
    ok( defined $dm,      'DirectMappedCache->new constructs' );
    is( $dm->sets, 4,     'DirectMappedCache sets is correct' );
}

# Test 12: first access is a miss
{
    my $dm = $DM->new(sets => 4);
    is( $dm->access(0), 0, 'first access is a miss' );
    is( $dm->misses,    1, 'miss counter increments' );
}

# Test 13: repeated access is a hit
{
    my $dm = $DM->new(sets => 4);
    $dm->access(0);         # miss
    is( $dm->access(0), 1,  'repeated access is a hit' );
    is( $dm->hits,      1,  'hit counter increments' );
}

# Test 14: conflicting addresses cause misses
{
    # addresses 0 and 4 both map to slot 0 in a 4-set cache (0%4=0, 4%4=0)
    my $dm = $DM->new(sets => 4);
    $dm->access(0);   # miss: fill slot 0 with addr 0
    $dm->access(4);   # miss: conflict! slot 0 evicted, filled with addr 4
    is( $dm->access(0), 0, 'address 0 missed after addr 4 took its slot' );
    is( $dm->misses,    3, 'three misses for conflicting addresses' );
}

# Test 15: non-conflicting addresses don't evict each other
{
    my $dm = $DM->new(sets => 4);
    $dm->access(0);   # slot 0
    $dm->access(1);   # slot 1
    is( $dm->access(0), 1, 'addr 0 still in cache after addr 1 added' );
    is( $dm->access(1), 1, 'addr 1 still in cache after checking addr 0' );
}

# ============================================================================
# SetAssociativeCache tests
# ============================================================================

my $SA = 'CodingAdventures::Cache::SetAssociativeCache';

# Test 16: SetAssociativeCache constructs
{
    my $sa = $SA->new(sets => 4, ways => 2);
    ok( defined $sa,      'SetAssociativeCache->new constructs' );
    is( $sa->sets, 4,     'sets is correct' );
    is( $sa->ways, 2,     'ways is correct' );
}

# Test 17: first access is a miss
{
    my $sa = $SA->new(sets => 4, ways => 2);
    is( $sa->access(0), 0, 'first access is a miss' );
    is( $sa->misses,    1, 'miss counter increments' );
}

# Test 18: repeated access is a hit
{
    my $sa = $SA->new(sets => 4, ways => 2);
    $sa->access(0);
    is( $sa->access(0), 1, 'repeated access is a hit' );
    is( $sa->hits,      1, 'hit counter increments' );
}

# Test 19: 2-way set handles two conflicting addresses without eviction
{
    # With ways=2, both addr 0 and addr 4 map to set 0.
    # Unlike direct-mapped, 2-way associativity can hold both.
    my $sa = $SA->new(sets => 4, ways => 2);
    $sa->access(0);   # miss: fill way 0 in set 0
    $sa->access(4);   # miss: fill way 1 in set 0
    is( $sa->access(0), 1, 'addr 0 is a hit (2-way handles both)' );
    is( $sa->access(4), 1, 'addr 4 is a hit (2-way handles both)' );
}

# Test 20: LRU eviction in set-associative cache
{
    # ways=2 means the set can hold 2 addresses.
    # Inserting a third forces LRU eviction.
    my $sa = $SA->new(sets => 4, ways => 2);
    $sa->access(0);    # miss: set 0, way 0
    $sa->access(4);    # miss: set 0, way 1
    # Now both ways occupied. Access 0 to make 0 MRU, 4 LRU.
    $sa->access(0);    # hit
    # Insert addr 8 (also maps to set 0): evicts 4 (LRU)
    $sa->access(8);    # miss, evicts 4
    is( $sa->access(0), 1, 'addr 0 still present (was MRU)' );
    is( $sa->access(8), 1, 'addr 8 present (just added)' );
    is( $sa->access(4), 0, 'addr 4 was evicted (was LRU)' );
}

# Test 21: hits and misses counts across multiple accesses
{
    my $sa = $SA->new(sets => 8, ways => 2);
    $sa->access(0);   # miss
    $sa->access(1);   # miss
    $sa->access(0);   # hit
    $sa->access(1);   # hit
    $sa->access(2);   # miss
    is( $sa->hits,   2, 'set-associative: 2 hits' );
    is( $sa->misses, 3, 'set-associative: 3 misses' );
}

# Test 22: LRUCache with integer and string values
{
    my $lru = $LRU->new(capacity => 5);
    $lru->put(1, 'one');
    $lru->put(2, 'two');
    is( $lru->get(1), 'one', 'LRUCache stores string values' );
    is( $lru->get(2), 'two', 'LRUCache retrieves by integer key' );
}

# Test 23: LRUCache invalid capacity dies
{
    my $err;
    eval { $LRU->new(capacity => 0) };
    ok( $@, 'LRUCache dies on capacity=0' );
    eval { $LRU->new(capacity => -1) };
    ok( $@, 'LRUCache dies on negative capacity' );
}

# Test 24: DirectMappedCache hit and miss totals
{
    my $dm = $DM->new(sets => 8);
    $dm->access(0);   # miss
    $dm->access(0);   # hit
    $dm->access(1);   # miss
    $dm->access(1);   # hit
    $dm->access(0);   # hit
    is( $dm->hits,   3, 'DirectMappedCache hits=3' );
    is( $dm->misses, 2, 'DirectMappedCache misses=2' );
}

# Test 25: SetAssociativeCache ways=1 behaves like direct-mapped
{
    # With ways=1, the set-associative cache is equivalent to direct-mapped.
    my $sa = $SA->new(sets => 4, ways => 1);
    $sa->access(0);   # miss
    $sa->access(4);   # miss (conflict, evicts 0)
    is( $sa->access(0), 0, 'ways=1 acts like direct-mapped: addr 0 evicted by addr 4' );
}

done_testing;
