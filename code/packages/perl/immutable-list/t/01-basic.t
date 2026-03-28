use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ImmutableList;

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::ImmutableList->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. empty()
# ---------------------------------------------------------------------------
my $e = CodingAdventures::ImmutableList->empty();
ok($e, 'empty() returns a defined object');
ok($e->is_empty, 'empty list reports is_empty = true');
is($e->length, 0, 'empty list has length 0');
is([$e->to_array], [], 'empty list to_array returns ()');

# Singleton: two calls return the same reference
is(CodingAdventures::ImmutableList->empty(),
   CodingAdventures::ImmutableList->empty(),
   'empty() is a singleton');

# ---------------------------------------------------------------------------
# 3. cons() and basic accessors
# ---------------------------------------------------------------------------
my $l1 = $e->cons(1);
ok(!$l1->is_empty, 'cons(1) list is not empty');
is($l1->head, 1, 'head of [1] is 1');
ok($l1->tail->is_empty, 'tail of [1] is empty');
is($l1->length, 1, 'length of [1] is 1');

my $l2 = $l1->cons(2);
is($l2->head, 2, 'head of [2,1] is 2');
is($l2->tail->head, 1, 'second element of [2,1] is 1');
is($l2->length, 2, 'length of [2,1] is 2');

my $l3 = $l2->cons(3);
is($l3->head, 3, 'head of [3,2,1] is 3');
is($l3->length, 3, 'length of [3,2,1] is 3');

# ---------------------------------------------------------------------------
# 4. Immutability / structural sharing
# ---------------------------------------------------------------------------
# $l2 must not be affected by $l3
is($l2->head, 2, 'l2 head unchanged after cons onto it');
is($l2->length, 2, 'l2 length unchanged after cons onto it');

# Both $l3 and a fresh $l3b share the same $l2 tail node
my $l3b = $l2->cons(99);
is($l3b->tail, $l2, 'structural sharing: tail of cons result == original list');

# ---------------------------------------------------------------------------
# 5. to_array()
# ---------------------------------------------------------------------------
is([$l3->to_array], [3, 2, 1], 'to_array() returns elements head-first');
is([$l1->to_array], [1],       'to_array() on single-element list');
is([$e->to_array],  [],        'to_array() on empty list');

# ---------------------------------------------------------------------------
# 6. from_array()
# ---------------------------------------------------------------------------
my $fa = CodingAdventures::ImmutableList->from_array(1, 2, 3);
is($fa->head, 1, 'from_array(1,2,3) head is 1');
is($fa->length, 3, 'from_array(1,2,3) length is 3');
is([$fa->to_array], [1, 2, 3], 'from_array round-trips through to_array');

my $fe = CodingAdventures::ImmutableList->from_array();
ok($fe->is_empty, 'from_array() with no args is empty');

# ---------------------------------------------------------------------------
# 7. append()
# ---------------------------------------------------------------------------
my $a = CodingAdventures::ImmutableList->from_array(1, 2, 3);
my $b = CodingAdventures::ImmutableList->from_array(4, 5);
my $c = $a->append($b);
is([$c->to_array], [1, 2, 3, 4, 5], 'append concatenates two lists');
is([$a->to_array], [1, 2, 3], 'append does not mutate left list');
is([$b->to_array], [4, 5],    'append does not mutate right list');

# append with empty
my $d = $a->append($e);
is([$d->to_array], [1, 2, 3], 'append with empty list is identity');
my $f = $e->append($a);
is([$f->to_array], [1, 2, 3], 'empty->append(list) equals list');

# ---------------------------------------------------------------------------
# 8. reverse()
# ---------------------------------------------------------------------------
my $rev = $l3->reverse;
is([$rev->to_array], [1, 2, 3], 'reverse of [3,2,1] is [1,2,3]');
is([$l3->to_array], [3, 2, 1],  'reverse does not mutate original');
ok($e->reverse->is_empty, 'reverse of empty is empty');

# ---------------------------------------------------------------------------
# 9. map()
# ---------------------------------------------------------------------------
my $doubled = $l3->map(sub { $_[0] * 2 });
is([$doubled->to_array], [6, 4, 2], 'map doubles each element');
is([$l3->to_array], [3, 2, 1],      'map does not mutate original');

my $stringed = CodingAdventures::ImmutableList->from_array(1, 2, 3)
    ->map(sub { "x$_[0]" });
is([$stringed->to_array], ['x1', 'x2', 'x3'], 'map can produce strings');

ok($e->map(sub { $_[0] + 1 })->is_empty, 'map over empty is empty');

# ---------------------------------------------------------------------------
# 10. filter()
# ---------------------------------------------------------------------------
my $big = $l3->filter(sub { $_[0] > 1 });
is([$big->to_array], [3, 2], 'filter keeps elements > 1');
is([$l3->to_array], [3, 2, 1], 'filter does not mutate original');

my $none = $l3->filter(sub { 0 });
ok($none->is_empty, 'filter(always false) returns empty list');

my $all = $l3->filter(sub { 1 });
is([$all->to_array], [3, 2, 1], 'filter(always true) returns same elements');

ok($e->filter(sub { 1 })->is_empty, 'filter over empty is empty');

# ---------------------------------------------------------------------------
# 11. foldl()
# ---------------------------------------------------------------------------
my $sum = $l3->foldl(0, sub { $_[0] + $_[1] });
is($sum, 6, 'foldl sum of [3,2,1] is 6');

my $product = CodingAdventures::ImmutableList->from_array(1, 2, 3, 4)
    ->foldl(1, sub { $_[0] * $_[1] });
is($product, 24, 'foldl product of [1,2,3,4] is 24');

# foldl with string concat preserves left-to-right order
my $str = CodingAdventures::ImmutableList->from_array('a', 'b', 'c')
    ->foldl('', sub { $_[0] . $_[1] });
is($str, 'abc', 'foldl string concat left-to-right');

my $zero_sum = $e->foldl(42, sub { $_[0] + $_[1] });
is($zero_sum, 42, 'foldl over empty returns initial value');

# ---------------------------------------------------------------------------
# 12. nth()
# ---------------------------------------------------------------------------
my $list = CodingAdventures::ImmutableList->from_array(10, 20, 30, 40);
is($list->nth(1), 10, 'nth(1) returns first element');
is($list->nth(2), 20, 'nth(2) returns second element');
is($list->nth(3), 30, 'nth(3) returns third element');
is($list->nth(4), 40, 'nth(4) returns fourth element');

# Out-of-range dies
ok(dies { $list->nth(5) }, 'nth(5) on 4-element list dies');
ok(dies { $e->nth(1) },    'nth(1) on empty list dies');
ok(dies { $list->nth(0) }, 'nth(0) dies (1-based)');

# ---------------------------------------------------------------------------
# 13. head/tail error cases
# ---------------------------------------------------------------------------
ok(dies { $e->head }, 'head() on empty list dies');
ok(dies { $e->tail }, 'tail() on empty list dies');

# ---------------------------------------------------------------------------
# 14. Chaining
# ---------------------------------------------------------------------------
my $chained = CodingAdventures::ImmutableList->from_array(1, 2, 3, 4, 5)
    ->filter(sub { $_[0] % 2 == 1 })   # [1, 3, 5]
    ->map(sub { $_[0] ** 2 })           # [1, 9, 25]
    ->foldl(0, sub { $_[0] + $_[1] });  # 35
is($chained, 35, 'filter->map->foldl chain works');

done_testing;
