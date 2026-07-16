use strict;
use warnings;
use Test::More;
use CodingAdventures::HyperLogLog;

sub within {
    my ($actual, $expected, $tolerance, $name) = @_;
    ok(
        abs($actual - $expected) <= $tolerance,
        "$name: got $actual, expected $expected +/- $tolerance",
    );
}

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'empty and duplicate values' => sub {
    my $sketch = CodingAdventures::HyperLogLog->new;
    is($sketch->count, 0, 'empty count');
    ok($sketch->is_empty, 'empty state');
    $sketch->add('same-value') for 1 .. 1000;
    is($sketch->count, 1, 'duplicates do not increase cardinality');
    ok(!$sketch->is_empty, 'non-empty state');
};

subtest 'bounded cardinality estimate' => sub {
    my $sketch = CodingAdventures::HyperLogLog->new(precision => 10);
    $sketch->add("user-$_") for 1 .. 10000;
    within($sketch->count, 10000, 1000, 'ten thousand distinct values');
};

subtest 'disjoint and overlapping merges' => sub {
    my $left = CodingAdventures::HyperLogLog->new(precision => 10);
    my $right = CodingAdventures::HyperLogLog->new(precision => 10);
    for my $value (1 .. 1000) {
        $left->add("left-$value");
        $right->add("right-$value");
    }

    my $left_count = $left->count;
    my $merged = $left->merge($right);
    is($left->count, $left_count, 'merge does not mutate the receiver');
    within($merged->count, 2000, 300, 'disjoint union');

    my $overlap = CodingAdventures::HyperLogLog->new(precision => 10);
    $overlap->add("left-$_") for 1 .. 1000;
    within($left->merge($overlap)->count, 1000, 150, 'overlapping union');
};

subtest 'metadata and defensive register snapshots' => sub {
    my $sketch = CodingAdventures::HyperLogLog->new(precision => 10);
    is($sketch->precision, 10, 'precision');
    is($sketch->num_registers, 1024, 'register count');
    is($sketch->memory_bytes, 768, 'packed memory size');
    ok(abs($sketch->error_rate - 0.0325) < 0.0001, 'theoretical error rate');

    my $registers = $sketch->registers;
    $registers->[0] = 99;
    is($sketch->registers->[0], 0, 'register snapshot is defensive');
    like("$sketch", qr/precision=10/, 'string rendering');
};

subtest 'clear, deterministic hashing, and validation' => sub {
    my $left = CodingAdventures::HyperLogLog->new(precision => 8);
    my $right = CodingAdventures::HyperLogLog->new(precision => 8);
    for my $value (1 .. 100) {
        $left->add("item-$value");
        $right->add("item-$value");
    }
    is_deeply($left->registers, $right->registers, 'hashing is deterministic');
    $left->clear;
    ok($left->is_empty, 'clear resets registers');

    dies_like(
        sub { CodingAdventures::HyperLogLog->new(precision => 3) },
        qr/precision must be an integer between 4 and 16/,
        'low precision rejected',
    );
    dies_like(
        sub { CodingAdventures::HyperLogLog->new(precision => 17) },
        qr/precision must be an integer between 4 and 16/,
        'high precision rejected',
    );
    dies_like(
        sub { $left->merge(CodingAdventures::HyperLogLog->new(precision => 9)) },
        qr/different precisions/,
        'precision mismatch rejected',
    );
    dies_like(
        sub { $left->merge({}) },
        qr/other must be a CodingAdventures::HyperLogLog/,
        'invalid merge operand rejected',
    );
};

done_testing;
