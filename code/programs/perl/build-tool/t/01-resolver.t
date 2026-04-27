#!/usr/bin/env perl

# t/01-resolver.t -- Tests for CodingAdventures::BuildTool::Resolver
# ====================================================================
#
# We test 20 cases as specified in the spec. The resolver builds a
# dependency graph from package metadata files.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use File::Spec ();

use CodingAdventures::BuildTool::Resolver;

# Shared temp root — package paths are $TMPROOT/$lang/$name so that
# File::Basename::basename($path) returns the correct package dir name.
my $TMPROOT = tempdir(CLEANUP => 1);

# make_pkg -- Create a package hashref with a path that has the right basename.
#
# The resolver's build_known_names() uses basename($pkg->{path}) to derive
# the ecosystem package name. So "perl/logic-gates" must live at a path
# ending in "logic-gates".
sub make_pkg {
    my (%args) = @_;
    my $name  = $args{name} // 'unknown/pkg';
    my $lang  = $args{language} // 'unknown';
    my $path  = "$TMPROOT/$name";
    make_path($path);
    return {
        name           => $name,
        path           => $path,
        language       => $lang,
        build_commands => [],
    };
}

# write_file -- Write $content to $path.
sub write_file {
    my ($path, $content) = @_;
    open(my $fh, '>', $path) or die "Cannot write $path: $!";
    print $fh $content;
    close $fh;
}

my $r = CodingAdventures::BuildTool::Resolver->new();

# ---------------------------------------------------------------------------
# Tests 1-8: Language-specific dependency parsing
# ---------------------------------------------------------------------------

subtest 'perl dep parsed from cpanfile' => sub {
    my $pkg     = make_pkg(name => 'perl/arithmetic',  language => 'perl');
    my $dep_pkg = make_pkg(name => 'perl/logic-gates', language => 'perl');
    write_file("$pkg->{path}/cpanfile",
        "requires 'coding-adventures-logic-gates', '0.01';\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                 'one dep resolved');
    is($deps[0],     'perl/logic-gates', 'dep is perl/logic-gates');
};

subtest 'python dep parsed from pyproject.toml' => sub {
    my $pkg     = make_pkg(name => 'python/arithmetic',  language => 'python');
    my $dep_pkg = make_pkg(name => 'python/logic-gates', language => 'python');
    write_file("$pkg->{path}/pyproject.toml",
        qq([project]\nname = "arithmetic"\ndependencies = [\n  "coding-adventures-logic-gates>=0.1",\n]\n));

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                   'one dep resolved');
    is($deps[0],     'python/logic-gates', 'dep is python/logic-gates');
};

subtest 'ruby dep parsed from Gemfile' => sub {
    my $pkg     = make_pkg(name => 'ruby/arithmetic',  language => 'ruby');
    my $dep_pkg = make_pkg(name => 'ruby/logic-gates', language => 'ruby');
    write_file("$pkg->{path}/Gemfile",
        qq(source "https://rubygems.org"\ngem "coding_adventures_logic_gates"\n));

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                 'one dep');
    is($deps[0],     'ruby/logic-gates', 'dep is ruby/logic-gates');
};

subtest 'go dep parsed from go.mod' => sub {
    my $pkg     = make_pkg(name => 'go/arithmetic',  language => 'go');
    my $dep_pkg = make_pkg(name => 'go/logic-gates', language => 'go');
    write_file("$pkg->{path}/go.mod",
        "module github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic\n\nrequire (\n  github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.1.0\n)\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,               'one dep');
    is($deps[0],     'go/logic-gates', 'dep is go/logic-gates');
};

subtest 'typescript dep parsed from package.json' => sub {
    my $pkg     = make_pkg(name => 'typescript/arithmetic',  language => 'typescript');
    my $dep_pkg = make_pkg(name => 'typescript/logic-gates', language => 'typescript');
    write_file("$pkg->{path}/package.json",
        qq({\n  "name": "\@coding-adventures/arithmetic",\n  "dependencies": {\n    "\@coding-adventures/logic-gates": "file:../logic-gates"\n  }\n}\n));

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                         'one dep');
    is($deps[0],     'typescript/logic-gates', 'dep is typescript/logic-gates');
};

subtest 'rust dep parsed from Cargo.toml' => sub {
    my $pkg     = make_pkg(name => 'rust/arithmetic',  language => 'rust');
    my $dep_pkg = make_pkg(name => 'rust/logic-gates', language => 'rust');
    write_file("$pkg->{path}/Cargo.toml",
        "[package]\nname = \"arithmetic\"\n\n[dependencies]\nlogic-gates = { path = \"../../packages/rust/logic-gates\" }\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                 'one dep');
    is($deps[0],     'rust/logic-gates', 'dep is rust/logic-gates');
};

subtest 'wasm dep parsed through rust scope' => sub {
    my $pkg     = make_pkg(name => 'wasm/arithmetic',  language => 'wasm');
    my $dep_pkg = make_pkg(name => 'rust/logic-gates', language => 'rust');
    write_file("$pkg->{path}/Cargo.toml",
        "[package]\nname = \"arithmetic-wasm\"\n\n[dependencies]\nlogic-gates = { path = \"../../packages/rust/logic-gates\" }\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg], 'rust');
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                 'one dep');
    is($deps[0],     'rust/logic-gates', 'dep is rust/logic-gates');
};

subtest 'elixir dep parsed from mix.exs' => sub {
    my $pkg     = make_pkg(name => 'elixir/arithmetic',  language => 'elixir');
    my $dep_pkg = make_pkg(name => 'elixir/logic-gates', language => 'elixir');
    write_file("$pkg->{path}/mix.exs",
        "defmodule Arithmetic.MixProject do\n  def deps do\n    [\n      {:coding_adventures_logic_gates, path: \"../logic-gates\"}\n    ]\n  end\nend\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,                   'one dep');
    is($deps[0],     'elixir/logic-gates', 'dep is elixir/logic-gates');
};

subtest 'lua dep parsed from rockspec' => sub {
    my $pkg     = make_pkg(name => 'lua/arithmetic',  language => 'lua');
    my $dep_pkg = make_pkg(name => 'lua/logic-gates', language => 'lua');
    write_file("$pkg->{path}/arithmetic-0.1-1.rockspec",
        "package = \"arithmetic\"\ndependencies = {\n  \"coding-adventures-logic-gates\"\n}\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,               'one dep');
    is($deps[0],     'lua/logic-gates', 'dep is lua/logic-gates');
};

subtest '.NET dep parsed from project reference' => sub {
    my $pkg     = make_pkg(name => 'fsharp/graph-tests', language => 'fsharp');
    my $dep_pkg = make_pkg(name => 'csharp/graph',      language => 'csharp');
    write_file("$dep_pkg->{path}/CodingAdventures.Graph.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>\n");
    write_file("$pkg->{path}/CodingAdventures.Graph.Tests.fsproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <ItemGroup>\n    <ProjectReference Include=\"../graph/CodingAdventures.Graph.csproj\" />\n  </ItemGroup>\n</Project>\n");

    my %known = $r->build_known_names([$pkg, $dep_pkg], 'dotnet');
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 1,           'one dep');
    is($deps[0],     'csharp/graph', 'dep is csharp/graph');
};

# ---------------------------------------------------------------------------
# Test 9: External dep skipped
# ---------------------------------------------------------------------------
subtest 'external dep is skipped' => sub {
    my $pkg = make_pkg(name => 'perl/app-external', language => 'perl');
    write_file("$pkg->{path}/cpanfile", "requires 'Moo';\nrequires 'Test2::V0';\n");

    my %known = $r->build_known_names([$pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 0, 'external deps not included');
};

# ---------------------------------------------------------------------------
# Test 10: Multiple deps in one cpanfile
# ---------------------------------------------------------------------------
subtest 'multiple deps in cpanfile' => sub {
    my $pkg  = make_pkg(name => 'perl/multi-dep-app', language => 'perl');
    my $dep1 = make_pkg(name => 'perl/logic-gates2',  language => 'perl');
    my $dep2 = make_pkg(name => 'perl/arithmetic2',   language => 'perl');
    my $dep3 = make_pkg(name => 'perl/bitset2',       language => 'perl');
    write_file("$pkg->{path}/cpanfile",
        "requires 'coding-adventures-logic-gates2';\nrequires 'coding-adventures-arithmetic2';\nrequires 'coding-adventures-bitset2';\n");

    my %known = $r->build_known_names([$pkg, $dep1, $dep2, $dep3]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 3, 'all 3 deps resolved');
};

# ---------------------------------------------------------------------------
# Test 11: Diamond dependency graph
# ---------------------------------------------------------------------------
subtest 'diamond dependency graph' => sub {
    # A depends on B and C. B and C both depend on D.
    my $pkg_a = make_pkg(name => 'perl/dia-a', language => 'perl');
    my $pkg_b = make_pkg(name => 'perl/dia-b', language => 'perl');
    my $pkg_c = make_pkg(name => 'perl/dia-c', language => 'perl');
    my $pkg_d = make_pkg(name => 'perl/dia-d', language => 'perl');
    write_file("$pkg_a->{path}/cpanfile",
        "requires 'coding-adventures-dia-b';\nrequires 'coding-adventures-dia-c';\n");
    write_file("$pkg_b->{path}/cpanfile", "requires 'coding-adventures-dia-d';\n");
    write_file("$pkg_c->{path}/cpanfile", "requires 'coding-adventures-dia-d';\n");
    write_file("$pkg_d->{path}/cpanfile", "");

    my $graph = $r->resolve([$pkg_a, $pkg_b, $pkg_c, $pkg_d]);

    my @d_succs = sort $graph->successors('perl/dia-d');
    is(scalar @d_succs, 2, 'D has 2 successors');
    ok(grep { $_ eq 'perl/dia-b' } @d_succs, 'D -> B');
    ok(grep { $_ eq 'perl/dia-c' } @d_succs, 'D -> C');
    ok(grep { $_ eq 'perl/dia-a' } ($graph->successors('perl/dia-b')), 'B -> A');
    ok(grep { $_ eq 'perl/dia-a' } ($graph->successors('perl/dia-c')), 'C -> A');
};

# ---------------------------------------------------------------------------
# Test 12: No deps → node only, no edges
# ---------------------------------------------------------------------------
subtest 'package with no deps' => sub {
    my $pkg = make_pkg(name => 'perl/base-nodeps', language => 'perl');
    write_file("$pkg->{path}/cpanfile", "requires 'perl', '5.026';\n");

    my $graph = $r->resolve([$pkg]);

    ok($graph->has_node('perl/base-nodeps'), 'node exists');
    is(scalar($graph->successors('perl/base-nodeps')),   0, 'no outgoing edges');
    is(scalar($graph->predecessors('perl/base-nodeps')), 0, 'no incoming edges');
};

# ---------------------------------------------------------------------------
# Test 13: Missing config file → no deps
# ---------------------------------------------------------------------------
subtest 'missing config file yields no deps' => sub {
    my $pkg = make_pkg(name => 'python/no-config', language => 'python');
    # No pyproject.toml created.

    my %known = $r->build_known_names([$pkg]);
    my @deps  = $r->resolve_dependencies($pkg, \%known);

    is(scalar @deps, 0, 'no deps from missing file');
};

# ---------------------------------------------------------------------------
# Tests 14-18: Known names mapping
# ---------------------------------------------------------------------------
subtest 'known name for Python' => sub {
    my $pkg   = make_pkg(name => 'python/logic-gates', language => 'python');
    my %known = $r->build_known_names([$pkg]);
    ok(exists $known{'coding-adventures-logic-gates'}, 'python known name registered');
    is($known{'coding-adventures-logic-gates'}, 'python/logic-gates', 'maps correctly');
};

subtest 'known name for Ruby' => sub {
    my $pkg   = make_pkg(name => 'ruby/logic-gates', language => 'ruby');
    my %known = $r->build_known_names([$pkg]);
    # Ruby convention: hyphens in dir name are converted to underscores.
    # "logic-gates" becomes "logic_gates" → "coding_adventures_logic_gates"
    ok(exists $known{'coding_adventures_logic_gates'}, 'ruby known name registered (underscores)');
};

subtest 'known name for Perl' => sub {
    my $pkg   = make_pkg(name => 'perl/logic-gates', language => 'perl');
    my %known = $r->build_known_names([$pkg]);
    ok(exists $known{'coding-adventures-logic-gates'}, 'perl known name registered');
    is($known{'coding-adventures-logic-gates'}, 'perl/logic-gates', 'maps correctly');
};

subtest 'known name for TypeScript' => sub {
    my $pkg   = make_pkg(name => 'typescript/logic-gates', language => 'typescript');
    my %known = $r->build_known_names([$pkg]);
    ok(exists $known{'@coding-adventures/logic-gates'}, 'typescript known name registered');
};

subtest 'known name for Rust' => sub {
    my $pkg   = make_pkg(name => 'rust/logic-gates', language => 'rust');
    my %known = $r->build_known_names([$pkg]);
    ok(exists $known{'logic-gates'}, 'rust known name registered');
};

subtest 'known name for .NET project file' => sub {
    my $pkg = make_pkg(name => 'csharp/graph', language => 'csharp');
    write_file("$pkg->{path}/CodingAdventures.Graph.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>\n");

    my %known = $r->build_known_names([$pkg], 'dotnet');
    my $expected = lc File::Spec->canonpath("$pkg->{path}/CodingAdventures.Graph.csproj");
    ok(exists $known{$expected}, 'dotnet project path registered');
    is($known{$expected}, 'csharp/graph', 'maps correctly');
};

# ---------------------------------------------------------------------------
# Test 19: Graph structure — topological sort
# ---------------------------------------------------------------------------
subtest 'graph topological sort for chain A->B->C' => sub {
    # C depends on B; B depends on A.
    my $pkg_a = make_pkg(name => 'perl/chain-a', language => 'perl');
    my $pkg_b = make_pkg(name => 'perl/chain-b', language => 'perl');
    my $pkg_c = make_pkg(name => 'perl/chain-c', language => 'perl');
    write_file("$pkg_b->{path}/cpanfile", "requires 'coding-adventures-chain-a';\n");
    write_file("$pkg_c->{path}/cpanfile", "requires 'coding-adventures-chain-b';\n");

    my $graph  = $r->resolve([$pkg_a, $pkg_b, $pkg_c]);
    my @groups = $graph->independent_groups();

    is(scalar @groups, 3, '3 levels for a chain of 3');

    my @l0 = @{ $groups[0] };
    my @l1 = @{ $groups[1] };
    my @l2 = @{ $groups[2] };
    is($l0[0], 'perl/chain-a', 'level 0 has chain-a');
    is($l1[0], 'perl/chain-b', 'level 1 has chain-b');
    is($l2[0], 'perl/chain-c', 'level 2 has chain-c');
};

# ---------------------------------------------------------------------------
# Test 20: Independent groups — disconnected graph
# ---------------------------------------------------------------------------
subtest 'independent groups for disconnected pairs' => sub {
    # A->B and C->D are two independent chains.
    my $pkg_a = make_pkg(name => 'perl/disc-a', language => 'perl');
    my $pkg_b = make_pkg(name => 'perl/disc-b', language => 'perl');
    my $pkg_c = make_pkg(name => 'perl/disc-c', language => 'perl');
    my $pkg_d = make_pkg(name => 'perl/disc-d', language => 'perl');
    write_file("$pkg_b->{path}/cpanfile", "requires 'coding-adventures-disc-a';\n");
    write_file("$pkg_d->{path}/cpanfile", "requires 'coding-adventures-disc-c';\n");

    my $graph  = $r->resolve([$pkg_a, $pkg_b, $pkg_c, $pkg_d]);
    my @groups = $graph->independent_groups();

    is(scalar @groups, 2, '2 levels');

    my @l0 = sort @{ $groups[0] };
    my @l1 = sort @{ $groups[1] };
    is($l0[0], 'perl/disc-a', 'level 0 has disc-a');
    is($l0[1], 'perl/disc-c', 'level 0 has disc-c');
    is($l1[0], 'perl/disc-b', 'level 1 has disc-b');
    is($l1[1], 'perl/disc-d', 'level 1 has disc-d');
};

done_testing();
