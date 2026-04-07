#!/usr/bin/env perl

# =============================================================================
# Tests for CodingAdventures::CorrelationVector
# =============================================================================
#
# This test file covers the full API surface of the CorrelationVector module.
# We use Test2::V0, the modern Perl testing framework that supersedes
# Test::More.
#
# Test2::V0 provides:
#   - ok($bool, $name)      — basic boolean assertion
#   - is($got, $expected)   — deep equality comparison
#   - like($got, $pattern)  — regex match assertion
#   - dies { ... }          — check that code dies (throws an exception)
#   - subtest $name => sub {} — group related tests together
#
# Each subtest is independent — failures in one don't prevent others from
# running. This makes it easier to identify the scope of a failure.

use strict;
use warnings;
use Test2::V0;

# Add the library to the path. The BUILD file handles this in CI via
# PERL5LIB, but for local `prove -l`, the `-l` flag adds `lib/` automatically.
use CodingAdventures::CorrelationVector;

# =============================================================================
# Test group 1: Root lifecycle
# =============================================================================
#
# The most fundamental test: create a root CV, contribute to it, pass it
# through a stage, and delete it. This exercises the core happy path.
#
# Expected ID format: "base.N" where base is 8 hex chars.

subtest 'root lifecycle' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    # --- create with an origin string ---
    my $cv_id = $log->create(origin_string => 'app.ts:5:12');

    # The ID should match "xxxxxxxx.N" (8 hex chars, dot, integer)
    like($cv_id, qr/\A[0-9a-f]{8}\.\d+\z/, 'cv_id matches base.N format');

    # The entry should be stored and accessible
    my $entry = $log->get($cv_id);
    ok(defined $entry, 'entry is stored after create');
    is($entry->{cv_id},        $cv_id,        'entry cv_id matches');
    is($entry->{parent_cv_id}, undef,         'root has no parent');
    is($entry->{merged_from},  [],            'root has no merged_from');
    is($entry->{deleted},      undef,         'root is not deleted');
    is($entry->{contributions}, [],           'no contributions yet');

    # --- contribute ---
    $log->contribute($cv_id,
        source => 'scope_analysis',
        tag    => 'resolved',
        meta   => { binding => 'local:count:fn_main' },
    );

    my $history = $log->history($cv_id);
    is(scalar @$history, 1, 'one contribution recorded');
    is($history->[0]{source}, 'scope_analysis', 'contribution source correct');
    is($history->[0]{tag},    'resolved',       'contribution tag correct');
    is($history->[0]{meta}{binding}, 'local:count:fn_main', 'meta preserved');
    ok(defined $history->[0]{timestamp}, 'timestamp is set');

    # --- passthrough ---
    $log->passthrough($cv_id, source => 'type_checker');

    $history = $log->history($cv_id);
    is(scalar @$history, 2, 'two entries after passthrough');
    is($history->[1]{source}, 'type_checker', 'passthrough source correct');
    is($history->[1]{tag},    'passthrough',  'passthrough tag correct');

    # pass_order should contain both sources, deduplicated
    my $entry2 = $log->get($cv_id);
    is(scalar @{ $entry2->{pass_order} }, 2, 'two unique sources in pass_order');

    # Passthrough again with same source — should NOT add duplicate to pass_order
    $log->passthrough($cv_id, source => 'type_checker');
    $entry2 = $log->get($cv_id);
    is(scalar @{ $entry2->{pass_order} }, 2, 'pass_order deduplicates sources');
    is(scalar @{ $log->history($cv_id) }, 3, 'but contributions still records all');

    # --- delete ---
    $log->delete($cv_id, by => 'dead_code_eliminator',
        meta => { reason => 'unreachable' });

    my $e = $log->get($cv_id);
    ok(defined $e->{deleted},      'deleted record is set');
    is($e->{deleted}{by},   'dead_code_eliminator', 'deleted.by correct');
    ok(defined $e->{deleted}{at},  'deleted.at timestamp is set');

    # Contributing to a deleted entity should die
    ok(dies { $log->contribute($cv_id, source => 'foo', tag => 'bar') },
        'contribute to deleted cv_id dies');

    # --- synthetic root ---
    my $syn_id = $log->create(synthetic => 1);
    like($syn_id, qr/\A00000000\.\d+\z/, 'synthetic cv_id starts with 00000000');
};

# =============================================================================
# Test group 2: Derivation
# =============================================================================
#
# Tests the parent-child relationship: one entity splits into multiple outputs.
# The derived IDs should embed the parent ID as a prefix.

subtest 'derivation' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    my $parent = $log->create(origin_string => 'src.ts:10:0');

    # Derive two children from the same parent
    my $child_a = $log->derive($parent);
    my $child_b = $log->derive($parent);

    # Derived IDs should start with the parent ID
    like($child_a, qr/\A\Q$parent\E\.\d+\z/, 'child_a ID prefixed with parent');
    like($child_b, qr/\A\Q$parent\E\.\d+\z/, 'child_b ID prefixed with parent');

    # Children should have different IDs
    ok($child_a ne $child_b, 'two derived children have different IDs');

    # Child entries should reference the parent
    my $entry_a = $log->get($child_a);
    is($entry_a->{parent_cv_id}, $parent, 'child_a parent_cv_id is correct');
    is($entry_a->{merged_from},  [],      'derived entry has no merged_from');

    # ancestors(child) → [parent]
    my $ancs = $log->ancestors($child_a);
    is(scalar @$ancs, 1, 'child has one ancestor');
    is($ancs->[0],    $parent, 'ancestor is the parent');

    # descendants(parent) → [child_a, child_b] (order may vary)
    my $descs = $log->descendants($parent);
    is(scalar @$descs, 2, 'parent has two descendants');
    my %desc_set = map { $_ => 1 } @$descs;
    ok($desc_set{$child_a}, 'child_a in descendants');
    ok($desc_set{$child_b}, 'child_b in descendants');

    # Deriving from non-existent parent should die
    ok(dies { $log->derive('nonexistent.0') },
        'derive from non-existent parent dies');
};

# =============================================================================
# Test group 3: Merging
# =============================================================================
#
# Tests combining multiple CVs into one. The merged CV has multiple parents,
# accessible via the merged_from field and via ancestors().

subtest 'merging' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    my $a = $log->create(origin_string => 'a.ts:1:0');
    my $b = $log->create(origin_string => 'b.ts:2:0');
    my $c = $log->create(origin_string => 'c.ts:3:0');

    # Merge all three into one
    my $merged = $log->merge([$a, $b, $c]);

    # Merged ID should be base.N format (base derived from sorted parent IDs)
    like($merged, qr/\A[0-9a-f]{8}\.\d+\z/, 'merged ID has base.N format');

    # The merged entry should reference all parents
    my $entry = $log->get($merged);
    my %from_set = map { $_ => 1 } @{ $entry->{merged_from} };
    ok($from_set{$a}, 'merged_from includes a');
    ok($from_set{$b}, 'merged_from includes b');
    ok($from_set{$c}, 'merged_from includes c');
    is($entry->{parent_cv_id}, undef, 'merged entry has no single parent_cv_id');

    # ancestors(merged) → should return all three parents
    my $ancs = $log->ancestors($merged);
    is(scalar @$ancs, 3, 'merged entity has three ancestors');
    my %anc_set = map { $_ => 1 } @$ancs;
    ok($anc_set{$a}, 'ancestor a present');
    ok($anc_set{$b}, 'ancestor b present');
    ok($anc_set{$c}, 'ancestor c present');

    # Each original parent should list the merged entity as a descendant
    for my $parent ($a, $b, $c) {
        my $descs = $log->descendants($parent);
        my %ds = map { $_ => 1 } @$descs;
        ok($ds{$merged}, "merged is a descendant of $parent");
    }

    # Merging non-existent IDs should die
    ok(dies { $log->merge(['nonexistent.0', $a]) },
        'merge with non-existent parent dies');

    # Merge order should not matter for the base hash
    # (The base is derived from the sorted parent IDs)
    my $log2 = CodingAdventures::CorrelationVector->new(enabled => 1);
    my $x = $log2->create(origin_string => 'x');
    my $y = $log2->create(origin_string => 'y');
    my $m1 = $log2->merge([$x, $y]);
    my $m2 = $log2->merge([$y, $x]);

    # The base should be the same (same sorted parents), but N differs
    my ($base1) = $m1 =~ /\A([0-9a-f]{8})\./;
    my ($base2) = $m2 =~ /\A([0-9a-f]{8})\./;
    is($base1, $base2, 'merge order does not affect base hash');
};

# =============================================================================
# Test group 4: Deep ancestry chain
# =============================================================================
#
# Tests a chain of four entities: A → B → C → D
# ancestors(D) should return [C, B, A] (nearest-first)
# lineage(D) should return [A, B, C, D] (oldest-first)

subtest 'deep ancestry' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    my $a = $log->create(origin_string => 'root');
    my $b = $log->derive($a);
    my $c = $log->derive($b);
    my $d = $log->derive($c);

    # ancestors(D) = [C, B, A] nearest-first
    my $ancs = $log->ancestors($d);
    is(scalar @$ancs, 3,  'D has three ancestors');
    is($ancs->[0],    $c, 'nearest ancestor is C');
    is($ancs->[1],    $b, 'second ancestor is B');
    is($ancs->[2],    $a, 'third ancestor is A');

    # lineage(D) = [A, B, C, D] oldest-first
    my $lin = $log->lineage($d);
    is(scalar @$lin, 4,           'lineage has four entries');
    is($lin->[0]{cv_id}, $a,      'lineage[0] is A (oldest)');
    is($lin->[1]{cv_id}, $b,      'lineage[1] is B');
    is($lin->[2]{cv_id}, $c,      'lineage[2] is C');
    is($lin->[3]{cv_id}, $d,      'lineage[3] is D (the entity itself)');

    # descendants(A) = [B, C, D] (in any order)
    my $descs = $log->descendants($a);
    is(scalar @$descs, 3, 'A has three descendants');
    my %ds = map { $_ => 1 } @$descs;
    ok($ds{$b}, 'B is a descendant of A');
    ok($ds{$c}, 'C is a descendant of A');
    ok($ds{$d}, 'D is a descendant of A');
};

# =============================================================================
# Test group 5: Disabled log
# =============================================================================
#
# When enabled => 0:
# - All write operations complete without error
# - CV IDs are still generated and returned (entities still need their IDs)
# - get() returns undef (nothing was stored)
# - history() returns empty list
# - ancestors() and descendants() return empty lists

subtest 'disabled log' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 0);

    # IDs should still be generated
    my $cv_id = $log->create(origin_string => 'test.ts:1:0');
    ok(defined $cv_id, 'cv_id is generated even when disabled');
    like($cv_id, qr/\A[0-9a-f]{8}\.\d+\z/, 'disabled: cv_id still has correct format');

    # Nothing should be stored
    my $entry = $log->get($cv_id);
    ok(!defined $entry, 'get() returns undef when disabled');

    # Write operations should complete without error
    ok(!dies { $log->contribute($cv_id, source => 'foo', tag => 'bar') },
        'contribute is a no-op when disabled');
    ok(!dies { $log->passthrough($cv_id, source => 'stage') },
        'passthrough is a no-op when disabled');
    ok(!dies { $log->delete($cv_id, by => 'cleaner') },
        'delete is a no-op when disabled');

    # Derived/merged IDs should still be generated
    my $child = $log->derive($cv_id);
    ok(defined $child, 'derive generates an ID when disabled');

    my $cv2    = $log->create(origin_string => 'other.ts');
    my $merged = $log->merge([$cv_id, $cv2]);
    ok(defined $merged, 'merge generates an ID when disabled');

    # Query operations return empty results
    is($log->history($cv_id),     [],  'history empty when disabled');
    is($log->ancestors($cv_id),   [],  'ancestors empty when disabled');
    is($log->descendants($cv_id), [],  'descendants empty when disabled');
    is($log->lineage($cv_id),     [],  'lineage empty when disabled');
};

# =============================================================================
# Test group 6: Serialization roundtrip
# =============================================================================
#
# Build a complex CVLog, serialize it, deserialize it, and verify that every
# entry is identical. This is the cross-process / cross-language interchange test.

subtest 'serialization roundtrip' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    # Build a complex log
    my $root_a = $log->create(origin_string => 'file_a.ts:1:0');
    my $root_b = $log->create(origin_string => 'file_b.ts:2:0');
    my $syn    = $log->create(synthetic => 1);

    $log->contribute($root_a,
        source => 'parser',
        tag    => 'created',
        meta   => { token => 'IDENTIFIER' },
    );
    $log->contribute($root_a,
        source => 'scope_analysis',
        tag    => 'resolved',
        meta   => { binding => 'global:foo' },
    );
    $log->passthrough($root_b, source => 'type_checker');

    my $child = $log->derive($root_a);
    $log->contribute($child,
        source => 'renamer',
        tag    => 'renamed',
        meta   => { from => 'foo', to => 'a' },
    );

    my $merged = $log->merge([$root_a, $root_b]);

    $log->delete($syn, by => 'pruner', meta => { reason => 'not needed' });

    # Serialize
    my $json = $log->serialize();
    ok(defined $json, 'serialize returns a string');
    like($json, qr/"entries"/, 'JSON contains entries key');

    # Deserialize
    my $log2 = CodingAdventures::CorrelationVector->deserialize($json);
    ok(defined $log2, 'deserialized log is defined');

    # Compare entries
    for my $cv_id ($root_a, $root_b, $syn, $child, $merged) {
        my $orig  = $log->get($cv_id);
        my $restored = $log2->get($cv_id);
        ok(defined $restored, "entry $cv_id is present after deserialization");
        is($restored->{cv_id},       $orig->{cv_id},        "cv_id matches for $cv_id");
        is($restored->{parent_cv_id}, $orig->{parent_cv_id}, "parent_cv_id matches for $cv_id");
        is(scalar @{ $restored->{contributions} },
           scalar @{ $orig->{contributions} },
           "contribution count matches for $cv_id");
        is(scalar @{ $restored->{merged_from} },
           scalar @{ $orig->{merged_from} },
           "merged_from count matches for $cv_id");
    }

    # Deleted entry
    my $del_entry = $log2->get($syn);
    ok(defined $del_entry->{deleted}, 'deleted record preserved');
    is($del_entry->{deleted}{by}, 'pruner', 'deleted.by preserved');

    # Deserialized log should continue generating unique IDs
    my $new_id = $log2->create(origin_string => 'new_file.ts');
    ok(defined $new_id, 'can create new entries from deserialized log');

    # The new ID should not collide with existing ones
    my $existing = $log2->get($new_id);
    # Actually new_id should be in the log now - let's check it's a fresh entry
    ok(!defined $log->get($new_id), 'new ID from deserialized log not in original log');
};

# =============================================================================
# Test group 7: ID uniqueness
# =============================================================================
#
# Creates 1000 root CVs and verifies all IDs are unique. This stress-tests
# the counter mechanism and the hash-base differentiation.

subtest 'id uniqueness' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);
    my %seen;
    my $count = 1000;

    # Mix of origins to exercise both same-base and different-base scenarios
    for my $i (1 .. $count) {
        # Use a mix: same origin (same base, different N) and different origins
        my $origin = ($i % 3 == 0) ? 'shared.ts:1:0' : "file_${i}.ts:1:0";
        my $cv_id  = $log->create(origin_string => $origin);

        ok(!$seen{$cv_id}, "id $cv_id is unique (iteration $i)");
        $seen{$cv_id} = 1;
    }

    is(scalar keys %seen, $count, "all $count IDs are distinct");

    # Additional test: synthetic IDs all have 00000000 base but differ by N
    my $log2 = CodingAdventures::CorrelationVector->new(enabled => 1);
    my %syn_seen;
    for my $i (1 .. 100) {
        my $cv_id = $log2->create(synthetic => 1);
        ok(!$syn_seen{$cv_id}, "synthetic id $cv_id is unique");
        $syn_seen{$cv_id} = 1;
        like($cv_id, qr/\A00000000\.\d+\z/, "synthetic id has correct format");
    }
    is(scalar keys %syn_seen, 100, 'all 100 synthetic IDs are distinct');
};

# =============================================================================
# Test group 8: Error handling
# =============================================================================
#
# Explicit error-case tests for all operations that should die on bad input.

subtest 'error handling' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    # contribute to non-existent ID
    ok(dies { $log->contribute('ghost.0', source => 'foo', tag => 'bar') },
        'contribute to non-existent cv_id dies');

    # passthrough to non-existent ID
    ok(dies { $log->passthrough('ghost.0', source => 'foo') },
        'passthrough to non-existent cv_id dies');

    # delete non-existent ID
    ok(dies { $log->delete('ghost.0', by => 'someone') },
        'delete non-existent cv_id dies');

    # contribute without source
    my $cv = $log->create(origin_string => 'x');
    ok(dies { $log->contribute($cv, tag => 'bar') },
        'contribute without source dies');

    # contribute without tag
    ok(dies { $log->contribute($cv, source => 'foo') },
        'contribute without tag dies');

    # passthrough without source
    ok(dies { $log->passthrough($cv) },
        'passthrough without source dies');
};

# =============================================================================
# Test group 9: pass_order across multiple entities
# =============================================================================
#
# Verifies that pass_order on each entry tracks which sources have touched
# that specific entity (not a global list across all entities).

subtest 'pass_order per entity' => sub {
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    my $a = $log->create(origin_string => 'a.ts');
    my $b = $log->create(origin_string => 'b.ts');

    # Entity A is touched by parser and renamer
    $log->contribute($a, source => 'parser',  tag => 'parsed');
    $log->contribute($a, source => 'renamer', tag => 'renamed');

    # Entity B is only touched by type_checker
    $log->contribute($b, source => 'type_checker', tag => 'checked');

    my $entry_a = $log->get($a);
    my $entry_b = $log->get($b);

    is(scalar @{ $entry_a->{pass_order} }, 2, 'entity A has 2 sources in pass_order');
    is(scalar @{ $entry_b->{pass_order} }, 1, 'entity B has 1 source in pass_order');

    # Contributing from the same source again should not grow pass_order
    $log->contribute($a, source => 'parser', tag => 'reparsed');
    my $entry_a2 = $log->get($a);
    is(scalar @{ $entry_a2->{pass_order} }, 2, 'pass_order still 2 after repeat source');
    is(scalar @{ $entry_a2->{contributions} }, 3, 'but contributions list grew to 3');
};

done_testing;
