# CodingAdventures::Heap (Perl)

Comparator-based binary heap package for Perl.

## Usage

```perl
use CodingAdventures::Heap;

my $min_heap = CodingAdventures::Heap::MinHeap->new();
$min_heap->push(5);
$min_heap->push(1);
$min_heap->push(3);
print $min_heap->pop();  # 1

my $max_heap = CodingAdventures::Heap::MaxHeap->new();
$max_heap->push(5);
$max_heap->push(1);
$max_heap->push(3);
print $max_heap->pop();  # 5

my $tuple_heap = CodingAdventures::Heap::MinHeap->new(sub {
    my ($left, $right) = @_;
    return $left->[0] <=> $right->[0] || $left->[1] cmp $right->[1];
});
$tuple_heap->push([1, 'b']);
$tuple_heap->push([1, 'a']);
print $tuple_heap->pop()->[1];  # a
```

## API

- `CodingAdventures::Heap::MinHeap->new($compare)`
- `CodingAdventures::Heap::MaxHeap->new($compare)`
- `from_iterable(\@items, $compare)`
- `push($value)`
- `pop()` returns the root or `undef` when empty
- `peek()` returns the root or `undef` when empty
- `size()`, `is_empty()`, and `to_array()`

## Running Tests

```bash
cpanm --installdeps --quiet .
prove -l -v t/
```
