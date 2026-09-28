#!/usr/bin/env perl

# t/15-graph-diff.t -- Process-free graph and diff-selection conformance
# =======================================================================
#
# The neutral corpus is the contract.  This suite names every required case
# explicitly, then feeds the already-decoded values through the production
# module.  Fixture discovery and JSON decoding stay outside that pure boundary.

use strict;
use warnings;
use utf8;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Cwd qw(abs_path);
use File::Spec ();
use JSON::PP ();
use Test2::V0;

use CodingAdventures::BuildTool::GraphDiff ();

my $REPO_ROOT = abs_path(File::Spec->catdir($Bin, qw(.. .. .. .. ..)));
my $FIXTURE_ROOT = File::Spec->catdir(
    $REPO_ROOT, qw(code specs fixtures build-tool-v1),
);
my $CASE_ROOT = File::Spec->catdir($FIXTURE_ROOT, 'cases');

my @EXPECTED_GRAPH_FIXTURES = qw(
    graph-canonical-edge-order.json
    graph-chain.json
    graph-cycle.json
    graph-diamond.json
    graph-empty.json
    graph-isolated.json
    graph-multiple-components.json
    graph-partial-cycle-no-output.json
);
my @EXPECTED_GRAPH_IDS = (
    'graph/canonical-edge-order',
    'graph/chain',
    'graph/cycle',
    'graph/diamond',
    'graph/empty',
    'graph/isolated',
    'graph/multiple-components',
    'graph/partial-cycle-no-output',
);
my @EXPECTED_DIFF_FIXTURES = qw(
    diff-selection-exact-build-fronts.json
    diff-selection-forced-package.json
    diff-selection-known-unmatched-near-build.json
    diff-selection-match-work-at-limit.json
    diff-selection-match-work-over-limit.json
    diff-selection-package-prefix.json
    diff-selection-repository-boundary.json
    diff-selection-strict-glob-character-classes.json
    diff-selection-transitive.json
    diff-selection-unknown-all.json
    diff-selection-unknown-error.json
);
my @EXPECTED_DIFF_IDS = (
    'diff-selection/exact-build-fronts',
    'diff-selection/forced-package',
    'diff-selection/known-unmatched-near-build',
    'diff-selection/match-work-at-limit',
    'diff-selection/match-work-over-limit',
    'diff-selection/package-prefix',
    'diff-selection/repository-boundary-reverse-index',
    'diff-selection/strict-glob-character-classes',
    'diff-selection/transitive-package-change',
    'diff-selection/unknown-path-all',
    'diff-selection/unknown-path-error',
);

sub load_json {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "cannot read fixture $path: $!";
    local $/;
    my $bytes = <$fh>;
    close $fh;
    return JSON::PP->new->utf8->decode($bytes);
}

sub load_case {
    my ($name) = @_;
    return load_json(File::Spec->catfile($CASE_ROOT, $name));
}

sub boundary {
    return load_json(File::Spec->catfile(
        $FIXTURE_ROOT, 'repository-source-input-boundary.json',
    ));
}

sub capture_error {
    my ($callback) = @_;
    my $ok = eval { $callback->(); 1 };
    return '' if $ok;
    my $error = "$@";
    $error =~ s/\s+at\s+.*\z//s;
    return $error;
}

sub capture_raw_error {
    my ($callback) = @_;
    my $ok = eval { $callback->(); 1 };
    return '' if $ok;
    return "$@";
}

subtest 'consumes the exact eight neutral graph fixtures' => sub {
    my @found = map { s{.*[\\/]}{}r }
        sort glob(File::Spec->catfile($CASE_ROOT, 'graph-*.json'));
    is(\@found, \@EXPECTED_GRAPH_FIXTURES, 'graph fixture roster is exact');
    is(
        [map { load_case($_)->{id} } @found],
        \@EXPECTED_GRAPH_IDS,
        'graph case identifiers are exact',
    );

    for my $name (@found) {
        my $fixture = load_case($name);
        my $actual = CodingAdventures::BuildTool::GraphDiff::evaluate_graph(
            $fixture->{input}{options},
        );
        my $expected = $fixture->{expected};
        if ($expected->{outcome} eq 'error') {
            is($actual->{error_code}, $expected->{diagnostics}[0]{code}, $fixture->{id});
            is($actual->{edges}, [], "$fixture->{id} has no partial edges");
            is($actual->{levels}, [], "$fixture->{id} has no partial levels");
        } else {
            is($actual->{error_code}, '', "$fixture->{id} succeeds");
            is($actual->{edges}, $expected->{result}{edges}, "$fixture->{id} edges");
            is($actual->{levels}, $expected->{result}{levels}, "$fixture->{id} levels");
        }
    }
};

subtest 'consumes the exact eleven neutral diff-selection fixtures' => sub {
    my @found = map { s{.*[\\/]}{}r }
        sort glob(File::Spec->catfile($CASE_ROOT, 'diff-selection-*.json'));
    is(\@found, \@EXPECTED_DIFF_FIXTURES, 'diff fixture roster is exact');
    is(
        [map { load_case($_)->{id} } @found],
        \@EXPECTED_DIFF_IDS,
        'diff case identifiers are exact',
    );

    for my $name (@found) {
        my $fixture = load_case($name);
        my %extra = exists $fixture->{input}{options}{boundary_sha256}
            ? (boundary => boundary())
            : ();
        my $before = JSON::PP->new->canonical->encode($fixture->{input});
        my $actual = CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection(
            $fixture->{input}, %extra,
        );
        my $expected = $fixture->{expected};
        is(
            JSON::PP->new->canonical->encode($fixture->{input}),
            $before,
            "$fixture->{id} leaves caller input unchanged",
        );
        if ($expected->{outcome} eq 'error') {
            is($actual->{error_code}, $expected->{diagnostics}[0]{code}, $fixture->{id});
            is($actual->{changed_packages}, [], "$fixture->{id} has no partial change set");
            is($actual->{affected_packages}, [], "$fixture->{id} has no partial affected set");
            is($actual->{prerequisite_packages}, [], "$fixture->{id} has no partial prerequisite set");
        } else {
            is($actual->{error_code}, '', "$fixture->{id} succeeds");
            is($actual->{changed_packages}, $expected->{result}{changed_packages}, "$fixture->{id} changed");
            is($actual->{affected_packages}, $expected->{result}{affected_packages}, "$fixture->{id} affected");
            is(
                $actual->{prerequisite_packages},
                $expected->{result}{prerequisite_packages},
                "$fixture->{id} prerequisites",
            );
        }
    }
};

subtest 'structural graph failures are stable and bounded' => sub {
    is(
        capture_raw_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => ['fixture/a', 'fixture/a'], edges => [],
            });
        }),
        'GRAPH_PACKAGE_DUPLICATE',
        'raw structural error contains only the stable code',
    );
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => ['fixture/a', 'fixture/a'], edges => [],
            });
        }),
        'GRAPH_PACKAGE_DUPLICATE',
        'duplicate package rejected',
    );
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => ['fixture/a'], edges => [['fixture/a', 'fixture/a']],
            });
        }),
        'GRAPH_EDGE_SELF',
        'self edge rejected',
    );
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => ['fixture/a'], edges => [['fixture/a', 'fixture/b']],
            });
        }),
        'GRAPH_EDGE_UNKNOWN',
        'unknown endpoint rejected',
    );
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => ['fixture/a', 'fixture/b'],
                edges => [['fixture/a', 'fixture/b'], ['fixture/a', 'fixture/b']],
            });
        }),
        'GRAPH_EDGE_DUPLICATE',
        'duplicate edge rejected',
    );

    my @exact = map { "fixture/p-$_" } 0 .. 4095;
    my $result = CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
        packages => \@exact, edges => [],
    });
    is(scalar @{$result->{levels}[0]}, 4096, 'exact package ceiling proceeds');
    push @exact, 'fixture/overflow';
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => \@exact, edges => [],
            });
        }),
        'GRAPH_PACKAGE_LIMIT_EXCEEDED',
        'package ceiling rejects only above the bound',
    );

    my @edge_packages = map { "fixture/e-$_" } 0 .. 4095;
    my @exact_edges;
    for my $offset (1 .. 4) {
        push @exact_edges, map {
            ["fixture/e-$_", 'fixture/e-' . (($_ + $offset) % 4096)]
        } 0 .. 4095;
    }
    my $edge_result = CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
        packages => \@edge_packages, edges => \@exact_edges,
    });
    is($edge_result->{error_code}, 'GRAPH_CYCLE', 'exact edge ceiling validates and proceeds');
    push @exact_edges, ['fixture/e-0', 'fixture/e-5'];
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_graph({
                packages => \@edge_packages, edges => \@exact_edges,
            });
        }),
        'GRAPH_EDGE_LIMIT_EXCEEDED',
        'edge ceiling rejects before inspecting the extra edge',
    );
};

subtest 'diff validation precedence is stable' => sub {
    my $cycle = {
        input => {
            options => {
                packages => [
                    {name => 'fixture/a', rel_path => 'a', source_mode => 'package_prefix', source_globs => []},
                    {name => 'fixture/b', rel_path => 'b', source_mode => 'package_prefix', source_globs => []},
                ],
                edges => [['fixture/a', 'fixture/b'], ['fixture/b', 'fixture/a']],
                forced_packages => [], unknown_path_policy => 'error',
            },
            changed_paths => [],
        },
    };
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($cycle->{input});
        }),
        'DIFF_EDGE_CYCLE',
        'diff cycles fail before selection',
    );

    my $invalid_glob = {
        operation => 'diff_selection',
        options => {
            packages => [{
                name => 'fixture/a', rel_path => 'p', source_mode => 'strict_globs',
                source_globs => ['src/**', '[z-a]'],
            }],
            edges => [], forced_packages => [], unknown_path_policy => 'error',
        },
        changed_paths => ['p/src/value.pm'],
    };
    is(
        capture_raw_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($invalid_glob);
        }),
        'DIFF_GLOB_INVALID',
        'invalid later glob is rejected before matching with only the stable code',
    );

    my $over = load_case('diff-selection-match-work-over-limit.json')->{input};
    push @{$over->{changed_paths}}, 'outside/unknown.txt';
    my $over_result = CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($over);
    is(
        $over_result->{error_code},
        'DIFF_MATCH_LIMIT_EXCEEDED',
        'match ceiling precedes unknown-path handling',
    );

    $over->{options}{boundary_sha256} = '0' x 64;
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection(
                $over, boundary => boundary(),
            );
        }),
        'DIFF_BOUNDARY_DIGEST_MISMATCH',
        'boundary mismatch precedes match-work preflight',
    );
};

subtest 'diff preflight prevents premature matching' => sub {
    my $original_matcher = \&CodingAdventures::BuildTool::GraphDiff::_match_path;
    my $calls = 0;
    no warnings 'redefine';
    local *CodingAdventures::BuildTool::GraphDiff::_match_path = sub {
        $calls++;
        return $original_matcher->(@_);
    };

    my $exact = load_case('diff-selection-match-work-at-limit.json')->{input};
    my $exact_result = CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($exact);
    is($exact_result->{error_code}, '', 'exact ceiling proceeds');
    ok($calls > 0, 'exact ceiling reaches the matcher');

    $calls = 0;
    my $over = load_case('diff-selection-match-work-over-limit.json')->{input};
    my $over_result = CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($over);
    is($over_result->{error_code}, 'DIFF_MATCH_LIMIT_EXCEEDED', 'over ceiling is rejected');
    is($calls, 0, 'over ceiling never reaches the matcher');
};

subtest 'portable roots and glob grammar are validated before selection' => sub {
    my $nested_alias = {
        operation => 'diff_selection',
        options => {
            packages => [
                {name => 'fixture/a', rel_path => 'Code/Package', source_mode => 'package_prefix'},
                {name => 'fixture/b', rel_path => 'code/package/child', source_mode => 'package_prefix'},
            ],
            edges => [], forced_packages => [], unknown_path_policy => 'error',
        },
        changed_paths => [],
    };
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($nested_alias);
        }),
        'DIFF_PATH_INVALID',
        'case-folded nested roots are rejected',
    );

    my $question_glob = {
        operation => 'diff_selection',
        options => {
            packages => [{
                name => 'fixture/a', rel_path => 'package-a', source_mode => 'strict_globs',
                source_globs => ['src/?.pm'],
            }],
            edges => [], forced_packages => [], unknown_path_policy => 'error',
        },
        changed_paths => [],
    };
    is(
        capture_error(sub {
            CodingAdventures::BuildTool::GraphDiff::evaluate_diff_selection($question_glob);
        }),
        'DIFF_GLOB_INVALID',
        'forbidden question-mark glob is rejected',
    );
};

done_testing();
