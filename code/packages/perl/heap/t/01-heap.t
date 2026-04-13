use strict;
use warnings;
use Test::More;
use CodingAdventures::Heap;
no warnings 'redefine';

sub is ($$;$) {
    my ($got, $expected, $name) = @_;
    if (ref($got) || ref($expected)) {
        return Test::More::is_deeply($got, $expected, $name);
    }
    return Test::More::is($got, $expected, $name);
}

subtest 'min heap orders ascending' => sub {
    my $heap = CodingAdventures::Heap::MinHeap->new();
    $heap->push($_) for (5, 3, 8, 1, 4);

    is $heap->peek(), 1, 'peek returns smallest value';
    is [map { $heap->pop() } 1..5], [1, 3, 4, 5, 8], 'pop returns ascending order';
    is $heap->pop(), undef, 'empty heap returns undef';
};

subtest 'max heap orders descending' => sub {
    my $heap = CodingAdventures::Heap::MaxHeap->new();
    $heap->push($_) for (5, 3, 8, 1, 4);

    is $heap->peek(), 8, 'peek returns largest value';
    is [map { $heap->pop() } 1..5], [8, 5, 4, 3, 1], 'pop returns descending order';
};

subtest 'from_iterable heapifies input' => sub {
    my $heap = CodingAdventures::Heap::MinHeap->from_iterable([9, 2, 7, 1, 5]);
    is $heap->size(), 5, 'heap contains all items';
    is [map { $heap->pop() } 1..5], [1, 2, 5, 7, 9], 'heapified items pop in sorted order';
};

subtest 'custom comparator supports tuples' => sub {
    my $heap = CodingAdventures::Heap::MinHeap->new(sub {
        my ($left, $right) = @_;
        return $left->[0] <=> $right->[0] || $left->[1] cmp $right->[1];
    });

    $heap->push([1, 'b']);
    $heap->push([1, 'a']);
    $heap->push([0, 'z']);

    is $heap->pop(), [0, 'z'], 'lowest priority tuple pops first';
    is $heap->pop(), [1, 'a'], 'tie breaks by secondary comparator';
    is $heap->pop(), [1, 'b'], 'remaining tuple pops last';
};

subtest 'empty helpers report state' => sub {
    my $heap = CodingAdventures::Heap::MinHeap->new();
    is $heap->size(), 0, 'empty heap size is zero';
    ok $heap->is_empty(), 'empty heap reports true';
    is $heap->peek(), undef, 'peek on empty heap returns undef';
    $heap->push(42);
    ok !$heap->is_empty(), 'heap is no longer empty after push';
    is $heap->to_array(), [42], 'to_array returns shallow copy';
};

done_testing;
