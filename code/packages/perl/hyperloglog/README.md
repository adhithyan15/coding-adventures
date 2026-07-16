# hyperloglog

Pure Perl HyperLogLog sketch for approximate distinct counting with fixed
memory. The implementation uses deterministic internal FNV-1a hashes and only
core Perl modules.

## Usage

~~~perl
use CodingAdventures::HyperLogLog;

my $sketch = CodingAdventures::HyperLogLog->new(precision => 10);
$sketch->add("user-$_") for 1 .. 10000;
print $sketch->count;
~~~

Precision may range from 4 to 16 and defaults to 10. The sketch supports add,
count, non-mutating merge, merge_in_place, clear, is_empty, and accessors for
precision, register count, theoretical error rate, packed memory size, and a
defensive register snapshot.

## Tests

~~~sh
cd code/packages/perl/hyperloglog
prove -l -v t/
~~~
