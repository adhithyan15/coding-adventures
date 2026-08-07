use strict;
use warnings;
use utf8;
use Test::More;
use CodingAdventures::Trie;

sub make_trie {
    my $trie = CodingAdventures::Trie->new;
    $trie->insert($_) for @_;
    return $trie;
}

sub dies_like {
    my ($code, $pattern, $name) = @_;
    my $ok = !eval { $code->(); 1 };
    ok($ok, $name);
    like($@, $pattern, "$name message") if $ok;
}

subtest 'empty trie' => sub {
    my $trie = CodingAdventures::Trie->new;
    is($trie->size, 0, 'size');
    is($trie->length, 0, 'length');
    ok($trie->is_empty, 'empty');
    is($trie->search('anything'), undef, 'missing search');
    ok(!$trie->starts_with('a'), 'missing prefix');
    ok($trie->is_valid, 'valid');
};

subtest 'insert, search, and update' => sub {
    my $trie = CodingAdventures::Trie->new;
    $trie->insert('hello', 42);
    is($trie->search('hello'), 42, 'exact key');
    is($trie->search('hell'), undef, 'prefix is not a key');
    is($trie->search('hellos'), undef, 'extension is not a key');
    $trie->insert('hello', 99);
    is($trie->search('hello'), 99, 'value updated');
    is($trie->size, 1, 'size unchanged');
    ok($trie->contains_key('hello'), 'contains key');
    ok($trie->key_exists('hello'), 'key_exists alias');
    $trie->insert('undefined', undef);
    ok($trie->contains('undefined'), 'terminal key can contain undef');
};

subtest 'prefix words and sorted keys' => sub {
    my $trie = make_trie(qw(banana app apple apply apt));
    is_deeply(
        $trie->words_with_prefix('app'),
        [['app', 1], ['apple', 1], ['apply', 1]],
        'prefix results are sorted',
    );
    is_deeply($trie->words_with_prefix('xyz'), [], 'missing prefix');
    is_deeply(
        $trie->keys,
        [qw(app apple apply apt banana)],
        'all keys sorted',
    );
    is_deeply($trie->entries, $trie->all_words, 'entries alias');
};

subtest 'delete leaves and shared prefixes' => sub {
    my $trie = make_trie(qw(app apple apt));
    ok($trie->delete('app'), 'delete shared prefix');
    ok(!$trie->contains_key('app'), 'deleted key absent');
    ok($trie->contains_key('apple'), 'longer key preserved');
    ok($trie->contains_key('apt'), 'sibling key preserved');
    is($trie->size, 2, 'size decremented');
    ok(!$trie->delete('missing'), 'missing key not deleted');
    ok(!$trie->delete('ap'), 'non-terminal prefix not deleted');
    ok($trie->delete('apple'), 'delete leaf');
    ok($trie->delete('apt'), 'delete final leaf');
    ok($trie->is_empty, 'empty after deletes');
    ok($trie->is_valid, 'valid after pruning');
};

subtest 'constructor and longest-prefix match' => sub {
    my $trie = CodingAdventures::Trie->new([
        ['a', 1], ['ab', 2], ['abc', 3], ['abcd', 4],
    ]);
    is_deeply($trie->longest_prefix_match('abcde'), ['abcd', 4], 'longest match');
    is($trie->longest_prefix_match('xyz'), undef, 'no match');
    is_deeply($trie->longest_prefix_match('a'), ['a', 1], 'exact match');
};

subtest 'Unicode and empty-string keys' => sub {
    my $trie = CodingAdventures::Trie->new;
    $trie->insert('', 'root');
    $trie->insert('cafe', 'plain');
    $trie->insert("cafe\x{0301}", 'accent-combining');
    $trie->insert("caf\x{00e9}", 'accent-single');
    is($trie->search(''), 'root', 'empty key');
    ok($trie->starts_with(''), 'empty prefix');
    ok($trie->starts_with('caf'), 'shared prefix');
    is($trie->search("caf\x{00e9}"), 'accent-single', 'Unicode key');
    is_deeply(
        $trie->longest_prefix_match("cafe\x{0301}-au-lait"),
        ["cafe\x{0301}", 'accent-combining'],
        'Unicode longest prefix',
    );
    ok($trie->delete(''), 'delete empty key');
    is($trie->search(''), undef, 'empty key absent');
    like("$trie", qr/3 keys/, 'string rendering');
};

subtest 'iteration and validation' => sub {
    my $trie = make_trie(qw(b a));
    my @seen;
    $trie->each(sub { push @seen, [@_] });
    is_deeply(\@seen, [['a', 1], ['b', 1]], 'sorted callback iteration');

    dies_like(
        sub { CodingAdventures::Trie->new('not-an-array') },
        qr/entries must be an array reference/,
        'constructor validation',
    );
    dies_like(
        sub { CodingAdventures::Trie->new([[]]) },
        qr/entry at index 0 must contain a key/,
        'entry validation',
    );
    dies_like(
        sub { $trie->insert([]) },
        qr/key must be a string/,
        'key validation',
    );
    dies_like(
        sub { $trie->starts_with({}) },
        qr/prefix must be a string/,
        'prefix validation',
    );
    dies_like(
        sub { $trie->longest_prefix_match([]) },
        qr/input must be a string/,
        'input validation',
    );
    dies_like(
        sub { $trie->each('not-code') },
        qr/callback must be a code reference/,
        'callback validation',
    );
};

done_testing;
