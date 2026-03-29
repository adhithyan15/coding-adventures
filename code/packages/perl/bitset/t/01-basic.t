use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Bitset;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::Bitset->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# Test 1: Constructor
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100);
    ok($bs, 'new(100) returns an object');
    is($bs->size(), 100, 'size() returns 100');
}

# ---------------------------------------------------------------------------
# Test 2: All bits start at 0
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(64);
    for my $i (0 .. 63) {
        is($bs->test($i), 0, "bit $i starts at 0");
    }
}

# ---------------------------------------------------------------------------
# Test 3: set() and test()
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100);
    my $bs2 = $bs->set(42);

    is($bs->test(42),  0, 'original unchanged after set()');
    is($bs2->test(42), 1, 'new bitset has bit 42 set');
    is($bs2->test(0),  0, 'bit 0 still 0 in new bitset');
    is($bs2->test(99), 0, 'bit 99 still 0 in new bitset');
}

# ---------------------------------------------------------------------------
# Test 4: set() at boundary indices
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100);
    my $a  = $bs->set(0);
    my $b  = $bs->set(99);
    my $c  = $bs->set(31);   # last bit of word 0
    my $d  = $bs->set(32);   # first bit of word 1

    is($a->test(0),  1, 'bit 0 set');
    is($b->test(99), 1, 'bit 99 set');
    is($c->test(31), 1, 'bit 31 set (word boundary)');
    is($d->test(32), 1, 'bit 32 set (word boundary)');
}

# ---------------------------------------------------------------------------
# Test 5: clear()
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100)->set(42)->set(0)->set(99);
    my $cleared = $bs->clear(42);

    is($cleared->test(42), 0, 'bit 42 cleared');
    is($cleared->test(0),  1, 'bit 0 still set after clearing 42');
    is($cleared->test(99), 1, 'bit 99 still set after clearing 42');
    is($bs->test(42),      1, 'original unchanged after clear()');
}

# ---------------------------------------------------------------------------
# Test 6: popcount()
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100);
    is($bs->popcount(), 0, 'popcount of empty bitset is 0');

    $bs = $bs->set(0)->set(42)->set(99);
    is($bs->popcount(), 3, 'popcount after setting 3 bits');

    $bs = $bs->clear(42);
    is($bs->popcount(), 2, 'popcount after clearing one bit');
}

# ---------------------------------------------------------------------------
# Test 7: set_bits()
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(100)->set(0)->set(42)->set(99);
    my @bits = $bs->set_bits();
    is(\@bits, [0, 42, 99], 'set_bits() returns sorted indices');
}

{
    my $bs = CodingAdventures::Bitset->new(100);
    my @bits = $bs->set_bits();
    is(\@bits, [], 'set_bits() of empty bitset is empty list');
}

# ---------------------------------------------------------------------------
# Test 8: Functional/immutable style — chain of operations
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(64)
        ->set(0)->set(1)->set(2)->set(63);

    is($bs->popcount(), 4, 'chained sets: popcount = 4');
    my @bits = $bs->set_bits();
    is(\@bits, [0, 1, 2, 63], 'chained sets: correct indices');
}

# ---------------------------------------------------------------------------
# Test 9: bitwise_and
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(8)->set(0)->set(1)->set(2);
    my $b = CodingAdventures::Bitset->new(8)->set(1)->set(2)->set(3);

    my $and = $a->bitwise_and($b);
    is($and->test(0), 0, 'AND: bit 0 is 0 (only in a)');
    is($and->test(1), 1, 'AND: bit 1 is 1 (in both)');
    is($and->test(2), 1, 'AND: bit 2 is 1 (in both)');
    is($and->test(3), 0, 'AND: bit 3 is 0 (only in b)');
    is($and->popcount(), 2, 'AND: popcount = 2');
}

# ---------------------------------------------------------------------------
# Test 10: bitwise_or
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(8)->set(0)->set(1);
    my $b = CodingAdventures::Bitset->new(8)->set(1)->set(2);

    my $or = $a->bitwise_or($b);
    is($or->test(0), 1, 'OR: bit 0');
    is($or->test(1), 1, 'OR: bit 1');
    is($or->test(2), 1, 'OR: bit 2');
    is($or->test(3), 0, 'OR: bit 3 not set');
    is($or->popcount(), 3, 'OR: popcount = 3');
}

# ---------------------------------------------------------------------------
# Test 11: bitwise_xor
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(8)->set(0)->set(1);
    my $b = CodingAdventures::Bitset->new(8)->set(1)->set(2);

    my $xor = $a->bitwise_xor($b);
    is($xor->test(0), 1, 'XOR: bit 0 (only in a)');
    is($xor->test(1), 0, 'XOR: bit 1 (in both -> 0)');
    is($xor->test(2), 1, 'XOR: bit 2 (only in b)');
    is($xor->popcount(), 2, 'XOR: popcount = 2');
}

# ---------------------------------------------------------------------------
# Test 12: Bitset with exactly 32 bits (one full word)
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(32);
    for my $i (0 .. 31) {
        $bs = $bs->set($i);
    }
    is($bs->popcount(), 32, 'all 32 bits set in a 32-bit bitset');
    my @bits = $bs->set_bits();
    is(scalar @bits, 32, '32 indices returned by set_bits');
}

# ---------------------------------------------------------------------------
# Test 13: Bitset with 33 bits (cross word boundary)
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(33)->set(32);
    is($bs->test(32),  1, 'bit 32 set in 33-bit bitset');
    is($bs->test(31),  0, 'bit 31 not set');
    is($bs->popcount(), 1, 'popcount = 1 for 33-bit bitset with only bit 32');
}

# ---------------------------------------------------------------------------
# Test 14: Out-of-range access dies
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(10);
    ok(dies { $bs->test(10) },  'test() dies on index == size');
    ok(dies { $bs->test(-1) },  'test() dies on negative index');
    ok(dies { $bs->set(10) },   'set() dies on out-of-range index');
    ok(dies { $bs->clear(10) }, 'clear() dies on out-of-range index');
}

# ---------------------------------------------------------------------------
# Test 15: Size-mismatch dies for bitwise operations
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(8);
    my $b = CodingAdventures::Bitset->new(16);
    ok(dies { $a->bitwise_and($b) }, 'bitwise_and dies on size mismatch');
    ok(dies { $a->bitwise_or($b) },  'bitwise_or dies on size mismatch');
    ok(dies { $a->bitwise_xor($b) }, 'bitwise_xor dies on size mismatch');
}

# ---------------------------------------------------------------------------
# Test 16: popcount cross-word boundary
# ---------------------------------------------------------------------------
{
    # Set bits spanning two words: 30, 31, 32, 33
    my $bs = CodingAdventures::Bitset->new(64)
        ->set(30)->set(31)->set(32)->set(33);
    is($bs->popcount(), 4, 'popcount spanning word boundary');
    is([$bs->set_bits()], [30, 31, 32, 33], 'set_bits spanning word boundary');
}

# ---------------------------------------------------------------------------
# Test 17: Constructing with invalid args dies
# ---------------------------------------------------------------------------
{
    ok(dies { CodingAdventures::Bitset->new(0)  }, 'new(0) dies');
    ok(dies { CodingAdventures::Bitset->new(-1) }, 'new(-1) dies');
}

# ---------------------------------------------------------------------------
# Test 18: XOR of identical bitsets is all-zero
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(32)->set(5)->set(10)->set(20);
    my $b = CodingAdventures::Bitset->new(32)->set(5)->set(10)->set(20);
    my $xor = $a->bitwise_xor($b);
    is($xor->popcount(), 0, 'XOR of identical bitsets has popcount 0');
}

# ---------------------------------------------------------------------------
# Test 19: AND of bitset with itself equals itself
# ---------------------------------------------------------------------------
{
    my $a = CodingAdventures::Bitset->new(16)->set(3)->set(7)->set(15);
    my $and = $a->bitwise_and($a);
    is($and->popcount(), 3, 'AND with itself preserves bits');
    is([$and->set_bits()], [3, 7, 15], 'AND with itself: correct indices');
}

# ---------------------------------------------------------------------------
# Test 20: Large bitset (1000 bits)
# ---------------------------------------------------------------------------
{
    my $bs = CodingAdventures::Bitset->new(1000);
    is($bs->size(), 1000, 'size() = 1000');
    $bs = $bs->set(0)->set(500)->set(999);
    is($bs->popcount(), 3, 'large bitset: 3 bits set');
    is([$bs->set_bits()], [0, 500, 999], 'large bitset: correct indices');
}

done_testing;
