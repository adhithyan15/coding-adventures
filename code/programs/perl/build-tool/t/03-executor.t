#!/usr/bin/env perl

# t/03-executor.t -- Tests for CodingAdventures::BuildTool::Executor
# ===================================================================
#
# 10 test cases for build execution.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);

use CodingAdventures::BuildTool::Executor;
use CodingAdventures::BuildTool::Resolver;

sub make_pkg {
    my (%args) = @_;
    return {
        name     => $args{name}     // 'test/pkg',
        path     => $args{path}     // '/tmp',
        language => $args{language} // 'perl',
        build_commands => $args{commands} // [],
    };
}

# Build a simple graph with the given packages and dep edges.
# $edges is an arrayref of [$from, $to] pairs.
sub make_graph {
    my ($pkgs_ref, $edges_ref) = @_;
    my $r = CodingAdventures::BuildTool::Resolver->new();
    my $graph = CodingAdventures::BuildTool::Graph->new();
    for my $pkg (@{$pkgs_ref}) {
        $graph->add_node($pkg->{name});
    }
    for my $edge (@{$edges_ref // []}) {
        $graph->add_edge($edge->[0], $edge->[1]);
    }
    return $graph;
}

# ---------------------------------------------------------------------------
# Test 1: Single package success
# ---------------------------------------------------------------------------
subtest 'single package succeeds' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $pkg = make_pkg(name => 'test/pkg', path => $dir, commands => ['echo ok']);
    my $graph = make_graph([$pkg]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    my $ok = $e->execute([$pkg], $graph);

    ok($ok, 'execute returns true on success');
    is($e->results()->[0]{status}, 'pass', 'result status is pass');
};

# ---------------------------------------------------------------------------
# Test 2: Single package failure
# ---------------------------------------------------------------------------
subtest 'single package fails' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $pkg = make_pkg(name => 'test/fail', path => $dir, commands => ['exit 1']);
    my $graph = make_graph([$pkg]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    my $ok = $e->execute([$pkg], $graph);

    ok(!$ok, 'execute returns false on failure');
    is($e->results()->[0]{status}, 'fail', 'result status is fail');
};

# ---------------------------------------------------------------------------
# Test 3: Parallel execution — two independent packages both run
# ---------------------------------------------------------------------------
subtest 'two independent packages both run' => sub {
    my $dir1 = tempdir(CLEANUP => 1);
    my $dir2 = tempdir(CLEANUP => 1);
    my $pkg1 = make_pkg(name => 'test/pkg1', path => $dir1, commands => ['echo one']);
    my $pkg2 = make_pkg(name => 'test/pkg2', path => $dir2, commands => ['echo two']);
    my $graph = make_graph([$pkg1, $pkg2]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 2);
    my $ok = $e->execute([$pkg1, $pkg2], $graph);

    ok($ok, 'both packages pass');
    is(scalar @{ $e->results() }, 2, '2 results');
};

# ---------------------------------------------------------------------------
# Test 4: Dep-skip propagation on failure
# ---------------------------------------------------------------------------
subtest 'dependent is skipped when dependency fails' => sub {
    my $dir_a = tempdir(CLEANUP => 1);
    my $dir_b = tempdir(CLEANUP => 1);
    my $pkg_a = make_pkg(name => 'test/a', path => $dir_a, commands => ['exit 1']);
    my $pkg_b = make_pkg(name => 'test/b', path => $dir_b, commands => ['echo b']);
    # b depends on a: a -> b
    my $graph = make_graph([$pkg_a, $pkg_b], [['test/a', 'test/b']]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    $e->execute([$pkg_a, $pkg_b], $graph);

    my %statuses = map { $_->{package} => $_->{status} } @{ $e->results() };
    is($statuses{'test/a'}, 'fail', 'pkg-a failed');
    is($statuses{'test/b'}, 'skip', 'pkg-b was skipped');
};

# ---------------------------------------------------------------------------
# Test 5: Dry-run mode — no commands executed
# ---------------------------------------------------------------------------
subtest 'dry-run does not execute commands' => sub {
    my $dir  = tempdir(CLEANUP => 1);
    my $flag = "$dir/was_run";
    my $pkg  = make_pkg(name => 'test/dry', path => $dir,
                        commands => ["touch $flag"]);
    my $graph = make_graph([$pkg]);

    my $e = CodingAdventures::BuildTool::Executor->new(dry_run => 1);
    my $ok = $e->execute([$pkg], $graph);

    ok($ok, 'dry-run returns success');
    ok(!-f $flag, 'touch command was not executed');
    like($e->results()->[0]{output}, qr/dry-run/i, 'output mentions dry-run');
};

# ---------------------------------------------------------------------------
# Test 6: Sequential fallback (jobs=1)
# ---------------------------------------------------------------------------
subtest 'jobs=1 runs sequentially' => sub {
    my @dirs = map { tempdir(CLEANUP => 1) } 1..3;
    my @order_file = "$dirs[0]/order";
    my @pkgs = map {
        my $i = $_;
        make_pkg(name => "test/pkg$i", path => $dirs[$i-1],
                 commands => ["echo $i >> $dirs[0]/order"])
    } 1..3;
    my $graph = make_graph(\@pkgs);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    $e->execute(\@pkgs, $graph);

    is(scalar @{ $e->results() }, 3, 'all 3 ran');
    is((grep { $_->{status} eq 'pass' } @{ $e->results() }), 3, 'all 3 passed');
};

# ---------------------------------------------------------------------------
# Test 7: Build order respected (A before B when B depends on A)
# ---------------------------------------------------------------------------
subtest 'build order respects dependencies' => sub {
    my $dir_a = tempdir(CLEANUP => 1);
    my $dir_b = tempdir(CLEANUP => 1);
    my $sentinel = "$dir_a/a_was_built";
    my $pkg_a = make_pkg(name => 'test/ordered-a', path => $dir_a,
                         commands => ["touch $sentinel"]);
    my $pkg_b = make_pkg(name => 'test/ordered-b', path => $dir_b,
                         commands => ["test -f $sentinel"]);
    my $graph = make_graph([$pkg_a, $pkg_b], [['test/ordered-a', 'test/ordered-b']]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    my $ok = $e->execute([$pkg_a, $pkg_b], $graph);

    ok($ok, 'b passes (sentinel created by a)');
};

# ---------------------------------------------------------------------------
# Test 8: Multiple independent groups
# ---------------------------------------------------------------------------
subtest 'multiple independent groups processed in order' => sub {
    my @dirs = map { tempdir(CLEANUP => 1) } 1..4;
    my @pkgs = (
        make_pkg(name => 'test/g-a', path => $dirs[0], commands => ['echo a']),
        make_pkg(name => 'test/g-b', path => $dirs[1], commands => ['echo b']),
        make_pkg(name => 'test/g-c', path => $dirs[2], commands => ['echo c']),
        make_pkg(name => 'test/g-d', path => $dirs[3], commands => ['echo d']),
    );
    # a->b and c->d (two independent chains)
    my $graph = make_graph(\@pkgs, [['test/g-a', 'test/g-b'], ['test/g-c', 'test/g-d']]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 4);
    my $ok = $e->execute(\@pkgs, $graph);

    ok($ok, 'all 4 pass');
    is(scalar @{ $e->results() }, 4, '4 results');
};

# ---------------------------------------------------------------------------
# Test 9: Command output is captured
# ---------------------------------------------------------------------------
subtest 'command output is captured in result' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $pkg = make_pkg(name => 'test/output', path => $dir,
                       commands => ['echo hello_unique_string']);
    my $graph = make_graph([$pkg]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    $e->execute([$pkg], $graph);

    like($e->results()->[0]{output}, qr/hello_unique_string/, 'output captured');
};

# ---------------------------------------------------------------------------
# Test 10: Package with no build commands passes immediately
# ---------------------------------------------------------------------------
subtest 'package with no commands passes' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $pkg = make_pkg(name => 'test/empty-cmds', path => $dir, commands => []);
    my $graph = make_graph([$pkg]);

    my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    my $ok = $e->execute([$pkg], $graph);

    ok($ok, 'passes with no commands');
    is($e->results()->[0]{status}, 'pass', 'status is pass');
};

done_testing();
