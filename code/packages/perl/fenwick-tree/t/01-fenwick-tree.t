use strict;
use warnings;
use Test::More;
use CodingAdventures::FenwickTree;

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'construction and state' => sub {
    my $empty = CodingAdventures::FenwickTree->new(0);
    is($empty->size, 0, 'empty size');
    is($empty->len, 0, 'len alias');
    ok($empty->is_empty, 'empty state');
    is_deeply($empty->bit_array, [], 'empty BIT array');

    dies_like(sub { CodingAdventures::FenwickTree->new(-1) }, qr/size must be non-negative/, 'negative size rejected');
    dies_like(sub { CodingAdventures::FenwickTree->new(1.5) }, qr/size must be an integer/, 'fractional size rejected');
    dies_like(sub { CodingAdventures::FenwickTree->from_list('nope') }, qr/array reference/, 'non-array input rejected');
    dies_like(sub { CodingAdventures::FenwickTree->from_list([1, 'x']) }, qr/value at index 2 must be a number/, 'non-number value rejected');
};

subtest 'prefix, range, point, and update operations' => sub {
    my $tree = CodingAdventures::FenwickTree->from_list([3, 2, 1, 7, 4]);
    is($tree->size, 5, 'tree size');
    ok(!$tree->is_empty, 'tree is populated');
    is_deeply(
        [map { $tree->prefix_sum($_) } 1 .. 5],
        [3, 5, 6, 13, 17],
        'reference prefix vector',
    );
    is($tree->prefix_sum(0), 0, 'zero prefix');
    is($tree->range_sum(2, 4), 10, 'middle range');
    is($tree->range_sum(1, 5), 17, 'full range');
    is($tree->point_query(4), 7, 'point query');
    is($tree->update(3, 5), $tree, 'update chains');
    is($tree->point_query(3), 6, 'updated point');
    is($tree->prefix_sum(3), 11, 'updated prefix');
};

subtest 'numeric values, defensive copy, and rendering' => sub {
    my $tree = CodingAdventures::FenwickTree->from_list([5, -2, 7, 1.5, 4.5]);
    is($tree->prefix_sum(5), 16, 'negative and floating values');
    is($tree->range_sum(2, 4), 6.5, 'floating range');
    $tree->update(2, 4.5);
    is($tree->point_query(2), 2.5, 'floating update');

    my $copy = $tree->bit_array;
    $copy->[0] = 99;
    is($tree->point_query(1), 5, 'BIT array is defensive copy');
    like("$tree", qr/FenwickTree/, 'string identifies tree');
};

subtest 'order statistics' => sub {
    my $tree = CodingAdventures::FenwickTree->from_list([1, 2, 3, 4, 5]);
    is_deeply(
        [map { $tree->find_kth($_) } (1, 2, 3, 4, 10, 15)],
        [1, 2, 2, 3, 4, 5],
        'find_kth reference vector',
    );

    dies_like(sub { CodingAdventures::FenwickTree->new(0)->find_kth(1) }, qr/empty tree/, 'empty lookup rejected');
    dies_like(sub { $tree->find_kth(0) }, qr/target must be positive/, 'zero target rejected');
    dies_like(sub { $tree->find_kth(16) }, qr/target exceeds total sum/, 'oversized target rejected');
};

subtest 'validation' => sub {
    my $tree = CodingAdventures::FenwickTree->from_list([1, 2, 3]);
    dies_like(sub { $tree->prefix_sum(-1) }, qr/prefix index -1 out of range/, 'negative prefix rejected');
    dies_like(sub { $tree->prefix_sum(4) }, qr/prefix index 4 out of range/, 'large prefix rejected');
    dies_like(sub { $tree->update(0, 1) }, qr/index 0 out of range/, 'zero update index rejected');
    dies_like(sub { $tree->update(1, 'x') }, qr/delta must be a number/, 'non-number delta rejected');
    dies_like(sub { $tree->range_sum(3, 1) }, qr/left must be <= right/, 'reversed range rejected');
    dies_like(sub { $tree->range_sum(0, 2) }, qr/index 0 out of range/, 'invalid left rejected');
    dies_like(sub { $tree->point_query(4) }, qr/index 4 out of range/, 'invalid point rejected');
};

done_testing;
