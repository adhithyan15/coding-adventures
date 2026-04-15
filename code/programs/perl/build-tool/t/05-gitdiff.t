#!/usr/bin/env perl

# t/05-gitdiff.t -- Tests for CodingAdventures::BuildTool::GitDiff
# =================================================================
#
# 10 test cases. Most tests mock the git output by subclassing GitDiff
# to override changed_files().

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use File::Spec ();

use CodingAdventures::BuildTool::GitDiff;
use CodingAdventures::BuildTool::Resolver;
use CodingAdventures::BuildTool::CIWorkflow ();

# Mock subclass that returns a predetermined list of changed files.
package MockGitDiff;
use parent -norequire, 'CodingAdventures::BuildTool::GitDiff';
sub new {
    my ($class, %args) = @_;
    my $self = CodingAdventures::BuildTool::GitDiff::new($class, %args);
    $self->{_mock_files} = $args{mock_files} // [];
    return $self;
}
sub changed_files { return @{ $_[0]->{_mock_files} } }

package main;

sub make_pkg {
    my (%args) = @_;
    return {
        name     => $args{name},
        path     => $args{path},
        language => $args{language} // 'perl',
        build_commands => [],
    };
}

sub empty_graph {
    my (@pkgs) = @_;
    my $r = CodingAdventures::BuildTool::Resolver->new();
    return $r->resolve(\@pkgs);
}

# ---------------------------------------------------------------------------
# Test 1: Changed file maps to package
# ---------------------------------------------------------------------------
subtest 'changed file maps to package' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path("$pkg_path/lib");

    my $pkg = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/packages/perl/bitset/lib/Bitset.pm"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    ok(grep { $_ eq 'perl/bitset' } @affected, 'perl/bitset affected');
};

# ---------------------------------------------------------------------------
# Test 2: File outside any package does not map to a package
# ---------------------------------------------------------------------------
subtest 'root README does not map to any package' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["README.md"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    is(scalar @affected, 0, 'README.md does not affect any package');
};

# ---------------------------------------------------------------------------
# Test 3: Multiple files in same package → one package
# ---------------------------------------------------------------------------
subtest 'multiple files in same package deduplicated' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => [
            "code/packages/perl/bitset/lib/Bitset.pm",
            "code/packages/perl/bitset/t/01-bitset.t",
        ],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    my @bitset = grep { $_ eq 'perl/bitset' } @affected;
    is(scalar @bitset, 1, 'perl/bitset appears exactly once');
};

# ---------------------------------------------------------------------------
# Test 4: Change propagates to dependents
# ---------------------------------------------------------------------------
subtest 'change in A propagates to B (B depends on A)' => sub {
    my $root  = tempdir(CLEANUP => 1);
    my $path_a = "$root/code/packages/perl/logic-gates";
    my $path_b = "$root/code/packages/perl/arithmetic";
    make_path($path_a);
    make_path($path_b);

    my $pkg_a = make_pkg(name => 'perl/logic-gates', path => $path_a);
    my $pkg_b = make_pkg(name => 'perl/arithmetic',  path => $path_b);

    # Build graph with A -> B.
    my $r = CodingAdventures::BuildTool::Resolver->new();
    my $graph = $r->resolve([$pkg_a, $pkg_b]);
    $graph->add_edge('perl/logic-gates', 'perl/arithmetic');

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/packages/perl/logic-gates/lib/LogicGates.pm"],
    );

    my @affected = $gd->affected_packages([$pkg_a, $pkg_b], $graph);
    ok(grep { $_ eq 'perl/logic-gates' } @affected, 'logic-gates is affected');
    ok(grep { $_ eq 'perl/arithmetic'  } @affected, 'arithmetic is transitively affected');
};

# ---------------------------------------------------------------------------
# Test 5: Empty diff → no packages affected
# ---------------------------------------------------------------------------
subtest 'empty diff yields no affected packages' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(root => $root, mock_files => []);

    my @affected = $gd->affected_packages([$pkg], $graph);
    is(scalar @affected, 0, 'no affected packages when no files changed');
};

# ---------------------------------------------------------------------------
# Test 6: BUILD file change affects the package
# ---------------------------------------------------------------------------
subtest 'BUILD file change affects package' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/logic-gates";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/logic-gates', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/packages/perl/logic-gates/BUILD"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    ok(grep { $_ eq 'perl/logic-gates' } @affected, 'logic-gates affected by BUILD change');
};

# ---------------------------------------------------------------------------
# Test 7: New file in package affects package
# ---------------------------------------------------------------------------
subtest 'new file in package affects package' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/logic-gates";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/logic-gates', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/packages/perl/logic-gates/lib/NewModule.pm"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    ok(grep { $_ eq 'perl/logic-gates' } @affected, 'package affected by new file');
};

# ---------------------------------------------------------------------------
# Test 8: Changes in two unrelated packages affect both
# ---------------------------------------------------------------------------
subtest 'changes in two unrelated packages affect both' => sub {
    my $root  = tempdir(CLEANUP => 1);
    my $path1 = "$root/code/packages/perl/pkg-x";
    my $path2 = "$root/code/packages/perl/pkg-y";
    make_path($path1);
    make_path($path2);

    my $pkg1  = make_pkg(name => 'perl/pkg-x', path => $path1);
    my $pkg2  = make_pkg(name => 'perl/pkg-y', path => $path2);
    my $graph = empty_graph($pkg1, $pkg2);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => [
            "code/packages/perl/pkg-x/lib/X.pm",
            "code/packages/perl/pkg-y/lib/Y.pm",
        ],
    );

    my @affected = $gd->affected_packages([$pkg1, $pkg2], $graph);
    ok(grep { $_ eq 'perl/pkg-x' } @affected, 'pkg-x affected');
    ok(grep { $_ eq 'perl/pkg-y' } @affected, 'pkg-y affected');
};

# ---------------------------------------------------------------------------
# Test 9: Shared code directory changes (code/specs) affect all packages
# ---------------------------------------------------------------------------
subtest 'file outside any package path does not affect specific packages' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/specs/perl-build-tool.md"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    is(scalar @affected, 0, 'specs change does not affect packages (correct: use --force for that)');
};

# ---------------------------------------------------------------------------
# Test 10: Path with trailing slash handled correctly
# ---------------------------------------------------------------------------
subtest 'path matching handles subdirectory correctly' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/tree";
    make_path("$pkg_path/lib/CodingAdventures/Tree");

    my $pkg   = make_pkg(name => 'perl/tree', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => ["code/packages/perl/tree/lib/CodingAdventures/Tree/Node.pm"],
    );

    my @affected = $gd->affected_packages([$pkg], $graph);
    ok(grep { $_ eq 'perl/tree' } @affected, 'deeply nested file maps to package');
};

# ---------------------------------------------------------------------------
# Test 11: Safe ci.yml changes no longer force a full rebuild
# ---------------------------------------------------------------------------
subtest 'toolchain-scoped ci workflow change does not force a full rebuild' => sub {
    my $root = tempdir(CLEANUP => 1);
    my $pkg_path = "$root/code/packages/perl/bitset";
    make_path($pkg_path);

    my $pkg   = make_pkg(name => 'perl/bitset', path => $pkg_path);
    my $graph = empty_graph($pkg);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => [CodingAdventures::BuildTool::CIWorkflow::ci_workflow_path()],
    );

    no warnings 'redefine';
    local *CodingAdventures::BuildTool::CIWorkflow::analyze_changes = sub {
        return { toolchains => { dotnet => 1 }, requires_full_rebuild => 0 };
    };

    my @affected = $gd->affected_packages([$pkg], $graph);
    is(scalar @affected, 0, 'safe ci.yml change does not affect packages directly');
};

# ---------------------------------------------------------------------------
# Test 12: Unsafe ci.yml changes still force a full rebuild
# ---------------------------------------------------------------------------
subtest 'unsafe ci workflow change still forces a full rebuild' => sub {
    my $root  = tempdir(CLEANUP => 1);
    my $path1 = "$root/code/packages/perl/pkg-x";
    my $path2 = "$root/code/packages/perl/pkg-y";
    make_path($path1);
    make_path($path2);

    my $pkg1  = make_pkg(name => 'perl/pkg-x', path => $path1);
    my $pkg2  = make_pkg(name => 'perl/pkg-y', path => $path2);
    my $graph = empty_graph($pkg1, $pkg2);

    my $gd = MockGitDiff->new(
        root       => $root,
        mock_files => [CodingAdventures::BuildTool::CIWorkflow::ci_workflow_path()],
    );

    no warnings 'redefine';
    local *CodingAdventures::BuildTool::CIWorkflow::analyze_changes = sub {
        return { toolchains => {}, requires_full_rebuild => 1 };
    };

    my @affected = sort $gd->affected_packages([$pkg1, $pkg2], $graph);
    is(\@affected, ['perl/pkg-x', 'perl/pkg-y'], 'unsafe ci.yml change still rebuilds everything');
};

done_testing();
