#!/usr/bin/env perl

# t/10-integration.t -- End-to-end integration tests
# ====================================================
#
# 2 test cases exercising the full pipeline with fixture packages.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);

use CodingAdventures::BuildTool;
use CodingAdventures::BuildTool::Discovery;
use CodingAdventures::BuildTool::Resolver;
use CodingAdventures::BuildTool::Executor;

sub write_file {
    my ($path, $content) = @_;
    open(my $fh, '>', $path) or die "Cannot write $path: $!";
    print $fh $content;
    close $fh;
}

# ---------------------------------------------------------------------------
# Test 1: Full pipeline (force) — discover → resolve → execute all
# ---------------------------------------------------------------------------
subtest 'full force pipeline discovers and runs packages' => sub {
    my $root = tempdir(CLEANUP => 1);

    # Create a small monorepo with two packages (pkg-a and pkg-b, b depends on a).
    my $dir_a = "$root/packages/perl/pkg-a";
    my $dir_b = "$root/packages/perl/pkg-b";
    make_path($dir_a);
    make_path($dir_b);

    write_file("$dir_a/BUILD",    "echo building pkg-a\n");
    write_file("$dir_a/cpanfile", "requires 'perl', '5.026';\n");
    write_file("$dir_b/BUILD",    "echo building pkg-b\n");
    write_file("$dir_b/cpanfile", "requires 'coding-adventures-pkg-a', '0.01';\n");

    # Discovery.
    my $d = CodingAdventures::BuildTool::Discovery->new(root => $root);
    $d->discover();
    my @pkgs = @{ $d->packages() };

    is(scalar @pkgs, 2, 'discovered 2 packages');

    # Resolve.
    my $r     = CodingAdventures::BuildTool::Resolver->new();
    my $graph = $r->resolve(\@pkgs);

    ok($graph->has_node('perl/pkg-a'), 'pkg-a in graph');
    ok($graph->has_node('perl/pkg-b'), 'pkg-b in graph');

    # Execute.
    my $e  = CodingAdventures::BuildTool::Executor->new(max_jobs => 1);
    my $ok = $e->execute(\@pkgs, $graph);

    ok($ok, 'all builds pass');
    is(scalar @{ $e->results() }, 2, '2 results');

    my %statuses = map { $_->{package} => $_->{status} } @{ $e->results() };
    is($statuses{'perl/pkg-a'}, 'pass', 'pkg-a passed');
    is($statuses{'perl/pkg-b'}, 'pass', 'pkg-b passed');
};

# ---------------------------------------------------------------------------
# Test 2: Full pipeline (dry-run) — shows plan without executing
# ---------------------------------------------------------------------------
subtest 'dry-run pipeline shows plan without executing' => sub {
    my $root   = tempdir(CLEANUP => 1);
    my $dir_a  = "$root/packages/perl/pkg-a";
    make_path($dir_a);

    my $sentinel = "$root/was_built";
    write_file("$dir_a/BUILD", "touch $sentinel\n");

    # Mock: use BuildTool with dry_run and force (bypass git diff).
    my $bt = CodingAdventures::BuildTool->new(
        root    => $root,
        force   => 1,
        dry_run => 1,
    );

    # Capture output using select() to redirect the default output handle.
    my $output = '';
    open(my $capture, '>', \$output) or die "Cannot open capture: $!";
    my $old_fh = select $capture;
    my $exit   = eval { $bt->run() };
    my $err    = $@;
    select $old_fh;
    close $capture;
    die $err if $err;

    ok(!-f $sentinel, 'touch was not executed in dry-run');
    is($exit, 0, 'dry-run returns 0');
};

done_testing();
