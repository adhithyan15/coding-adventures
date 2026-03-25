#!/usr/bin/env perl

# t/04-cache.t -- Tests for CodingAdventures::BuildTool::Cache
# =============================================================
#
# 8 test cases covering cache roundtrip, change detection, and edge cases.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir tempfile);

use CodingAdventures::BuildTool::Cache;

sub make_pkg {
    my ($name) = @_;
    return { name => $name, path => '/tmp', language => 'perl', build_commands => [] };
}

# ---------------------------------------------------------------------------
# Test 1: Save and load roundtrip
# ---------------------------------------------------------------------------
subtest 'save and load roundtrip' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";
    my %hashes = (
        'perl/logic-gates' => 'abc123',
        'perl/arithmetic'  => 'def456',
    );

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save(\%hashes);

    my $c2 = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c2->load();

    is($c2->get('perl/logic-gates'), 'abc123', 'logic-gates hash preserved');
    is($c2->get('perl/arithmetic'),  'def456', 'arithmetic hash preserved');
};

# ---------------------------------------------------------------------------
# Test 2: Changed package detected
# ---------------------------------------------------------------------------
subtest 'changed package detected' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save({ 'perl/foo' => 'old_hash' });
    $c->load();

    my @pkgs = (make_pkg('perl/foo'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/foo' => 'new_hash' });

    is(scalar @changed, 1, 'one changed package detected');
    is($changed[0], 'perl/foo', 'correct package identified');
};

# ---------------------------------------------------------------------------
# Test 3: New package detected
# ---------------------------------------------------------------------------
subtest 'new package not in cache is detected' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save({});
    $c->load();

    my @pkgs    = (make_pkg('perl/new-pkg'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/new-pkg' => 'hash123' });

    is(scalar @changed, 1, 'new package detected as changed');
};

# ---------------------------------------------------------------------------
# Test 4: Removed package in cache is ignored
# ---------------------------------------------------------------------------
subtest 'package removed from repo is ignored' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save({ 'perl/old-pkg' => 'hash999', 'perl/current-pkg' => 'hash1' });
    $c->load();

    # 'perl/old-pkg' no longer exists; we only have 'perl/current-pkg'.
    my @pkgs    = (make_pkg('perl/current-pkg'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/current-pkg' => 'hash1' });

    is(scalar @changed, 0, 'current-pkg unchanged, old-pkg ignored');
};

# ---------------------------------------------------------------------------
# Test 5: Missing cache file treated as empty
# ---------------------------------------------------------------------------
subtest 'missing cache file is treated as empty' => sub {
    my $path = '/tmp/nonexistent-cache-file-xyz.json';
    my $c    = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->load();

    my @pkgs    = (make_pkg('perl/foo'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/foo' => 'hash1' });

    is(scalar @changed, 1, 'all packages changed when cache is missing');
};

# ---------------------------------------------------------------------------
# Test 6: Corrupt cache file treated as empty
# ---------------------------------------------------------------------------
subtest 'corrupt cache file treated as empty' => sub {
    my ($fh, $path) = tempfile(SUFFIX => '.json', UNLINK => 1);
    print $fh "{ this is not valid json !!!\n";
    close $fh;

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);

    # Should not die, should warn and treat as empty.
    my $ok = eval { $c->load(); 1 };
    ok($ok, 'load does not die on corrupt file');

    my @pkgs    = (make_pkg('perl/foo'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/foo' => 'hash1' });
    is(scalar @changed, 1, 'all packages changed after corrupt cache');
};

# ---------------------------------------------------------------------------
# Test 7: Cache file is valid JSON
# ---------------------------------------------------------------------------
subtest 'saved cache is valid JSON' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";
    my $c    = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save({ 'perl/foo' => 'aaa', 'perl/bar' => 'bbb' });

    # Read raw and attempt parse.
    open(my $fh, '<', $path) or die;
    local $/;
    my $raw = <$fh>;
    close $fh;

    use JSON::PP;
    my $parsed = eval { JSON::PP::decode_json($raw) };
    ok(!$@, "no parse error: $@");
    is(ref $parsed, 'HASH', 'parsed as hash');
};

# ---------------------------------------------------------------------------
# Test 8: Unchanged package not in changed list
# ---------------------------------------------------------------------------
subtest 'unchanged package not returned as changed' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $path = "$dir/cache.json";

    my $c = CodingAdventures::BuildTool::Cache->new(path => $path);
    $c->save({ 'perl/stable' => 'same_hash' });
    $c->load();

    my @pkgs    = (make_pkg('perl/stable'));
    my @changed = $c->changed_packages(\@pkgs, { 'perl/stable' => 'same_hash' });

    is(scalar @changed, 0, 'unchanged package not listed');
};

done_testing();
