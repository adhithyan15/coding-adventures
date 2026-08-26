#!/usr/bin/env perl

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
use Cwd qw(abs_path);
use JSON::PP ();
use File::Temp qw(tempdir);
use File::Path qw(make_path);

use CodingAdventures::BuildTool::Validator;

sub write_ci {
    my ($root, $content) = @_;
    make_path("$root/.github/workflows");
    open(my $fh, '>', "$root/.github/workflows/ci.yml") or die "Cannot create ci.yml: $!";
    print {$fh} $content;
    close($fh);
}

sub write_file {
    my ($path, $content) = @_;
    my $dir = $path;
    $dir =~ s{/[^/]+$}{};
    make_path($dir);
    open(my $fh, '>', $path) or die "Cannot create $path: $!";
    print {$fh} $content;
    close($fh);
}

sub read_json {
    my ($path) = @_;
    open(my $fh, '<:raw', $path) or die "Cannot read $path: $!";
    local $/;
    my $payload = <$fh>;
    close($fh);
    return JSON::PP->new->utf8->decode($payload);
}

subtest 'fails without normalized outputs' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_ci($root, <<'YAML');
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
YAML

    my $error = CodingAdventures::BuildTool::Validator::validate_ci_full_build_toolchains(
        $root,
        [
            { language => 'elixir' },
            { language => 'python' },
        ],
    );

    ok(defined $error, 'validation fails');
    like($error, qr/\.github\/workflows\/ci\.yml/, 'mentions ci.yml');
    like($error, qr/elixir/, 'mentions elixir');
    like($error, qr/python/, 'mentions python');
};

subtest 'allows normalized outputs' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_ci($root, <<'YAML');
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
YAML

    is(
        CodingAdventures::BuildTool::Validator::validate_ci_full_build_toolchains(
            $root,
            [
                { language => 'elixir' },
                { language => 'python' },
            ],
        ),
        undef,
        'validation passes'
    );
};

subtest 'validate_build_contracts flags lua isolated-build violations' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_file("$root/code/packages/lua/problem_pkg/BUILD", <<'BUILD');
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
BUILD

    my $error = CodingAdventures::BuildTool::Validator::validate_build_contracts(
        $root,
        [
            { language => 'lua', path => "$root/code/packages/lua/problem_pkg" },
        ],
    );

    ok(defined $error, 'validation fails');
    like($error, qr/coding-adventures-branch-predictor/, 'mentions unrelated remove');
    like($error, qr/state_machine before directed_graph/, 'mentions build order');
};

subtest 'validate_build_contracts flags guarded lua install without deps mode' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_file("$root/code/packages/lua/guarded_pkg/BUILD", <<'BUILD');
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
BUILD

    my $error = CodingAdventures::BuildTool::Validator::validate_build_contracts(
        $root,
        [
            { language => 'lua', path => "$root/code/packages/lua/guarded_pkg" },
        ],
    );

    ok(defined $error, 'validation fails');
    like($error, qr/--deps-mode=none or --no-manifest/, 'mentions deps-mode guidance');
};

subtest 'validate_build_contracts flags windows lua sibling drift' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_file("$root/code/packages/lua/arm1_gatelevel/BUILD", <<'BUILD');
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
BUILD
    write_file("$root/code/packages/lua/arm1_gatelevel/BUILD_windows", <<'BUILD');
(cd ..\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
BUILD

    my $error = CodingAdventures::BuildTool::Validator::validate_build_contracts(
        $root,
        [
            { language => 'lua', path => "$root/code/packages/lua/arm1_gatelevel" },
        ],
    );

    ok(defined $error, 'validation fails');
    like($error, qr/BUILD_windows is missing sibling installs present in BUILD/, 'mentions missing windows prereqs');
    like($error, qr/\.\.\/logic_gates/, 'mentions missing logic_gates prereq');
    like($error, qr/\.\.\/arithmetic/, 'mentions missing arithmetic prereq');
    like($error, qr/--deps-mode=none or --no-manifest/, 'mentions deps-mode guidance');
};

subtest 'validate_build_contracts flags perl Test2 bootstrap without --notest' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_file("$root/code/packages/perl/draw-instructions-svg/BUILD", <<'BUILD');
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
BUILD

    my $error = CodingAdventures::BuildTool::Validator::validate_build_contracts(
        $root,
        [
            { language => 'perl', path => "$root/code/packages/perl/draw-instructions-svg" },
        ],
    );

    ok(defined $error, 'validation fails');
    like($error, qr/Test2::V0 without --notest/, 'mentions notest requirement');
};

subtest 'validate_build_contracts allows safe lua isolated-build patterns' => sub {
    my $root = tempdir(CLEANUP => 1);
    write_file("$root/code/packages/lua/safe_pkg/BUILD", <<'BUILD');
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
BUILD
    write_file("$root/code/packages/lua/safe_pkg/BUILD_windows", <<'BUILD');
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
BUILD

    is(
        CodingAdventures::BuildTool::Validator::validate_build_contracts(
            $root,
            [
                { language => 'lua', path => "$root/code/packages/lua/safe_pkg" },
            ],
        ),
        undef,
        'validation passes'
    );
};

subtest 'orphan-crate validator consumes every language-neutral fixture' => sub {
    my $repo_root = abs_path("$Bin/../../../../..");
    my $cases = "$repo_root/code/specs/fixtures/build-tool-v1/cases";

    for my $name (qw(crates-clean crates-unlisted exemptions-invalid exemptions-stale)) {
        my $fixture = read_json("$cases/validation-orphan-$name.json");
        my $snapshot = $fixture->{input}{options}{orphan_snapshot};
        my $actual = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot(
            $snapshot,
        );
        my %actual_result = %{$actual};
        delete $actual_result{diagnostics};

        is($actual->{diagnostics}, $fixture->{expected}{diagnostics}, "$name diagnostics match");
        is(\%actual_result, $fixture->{expected}{result}, "$name result matches");
    }
};

subtest 'orphan-crate validator redacts unsafe paths including invalid UTF-8' => sub {
    my $invalid_utf8 = "code/packages/rust/\xFF";
    my @unsafe_paths = (
        '',
        "😀" x 513,
        '/absolute/secret-project',
        'C:/host/secret-project',
        'code/packages/rust/bad<name>',
        'code/packages/rust/trailing.',
        'code/packages/rust/CON',
        $invalid_utf8,
    );

    for my $unsafe_path (@unsafe_paths) {
        my $result = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot({
            directories => ['code/packages/rust/demo'],
            manifests => [{ path => 'code/packages/rust/demo', kind => 'package' }],
            build_files => [],
            exemptions => [{
                line => 7,
                kind => 'PENDING',
                path => $unsafe_path,
                reason => 'not allowed',
            }],
        });
        my ($invalid) = grep { $_->{code} eq 'ORPHAN_EXEMPTION_INVALID' }
            @{$result->{diagnostics}};

        is(
            $invalid,
            {
                code => 'ORPHAN_EXEMPTION_INVALID',
                severity => 'error',
                path => 'code/BUILD-EXEMPTIONS',
                details => { line => 7, problem => 'PATH_UNSAFE' },
            },
            'unsafe path is replaced by the fixed ledger diagnostic',
        );
        ok(!exists $invalid->{details}{path}, 'raw unsafe path is not retained');
    }
};

subtest 'orphan-crate validator uses the exact Python blank-reason set' => sub {
    my $result = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot({
        directories => ['code/packages/rust/blank', 'code/packages/rust/bom'],
        manifests => [
            { path => 'code/packages/rust/blank', kind => 'package' },
            { path => 'code/packages/rust/bom', kind => 'package' },
        ],
        build_files => [],
        exemptions => [
            { line => 7, kind => 'PENDING', path => 'code/packages/rust/blank', reason => "\x{001C}" },
            { line => 8, kind => 'PENDING', path => 'code/packages/rust/bom', reason => "\x{FEFF}" },
        ],
    });

    is($result->{pending_exemption_count}, 1, 'BOM reason remains active and non-blank');
    is(
        $result->{diagnostic_codes},
        ['ORPHAN_CRATE_UNLISTED', 'ORPHAN_EXEMPTION_INVALID'],
        'blank entry fails closed without suppressing its orphan',
    );
    is(
        $result->{diagnostics}[-1]{details}{problem},
        'REASON_MISSING',
        'information-separator reason is blank',
    );
};

subtest 'orphan-crate validator chooses closest empty BUILD then fixed name rank' => sub {
    my $result = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot({
        directories => ['code/packages/rust/demo/child'],
        manifests => [{ path => 'code/packages/rust/demo/child', kind => 'package' }],
        build_files => [
            { path => 'code/packages/rust/BUILD', state => 'empty' },
            { path => 'code/packages/rust/demo/BUILD_linux', state => 'empty' },
            { path => 'code/packages/rust/demo/BUILD', state => 'empty' },
            { path => 'code/packages/rust/demo2/BUILD', state => 'runnable' },
        ],
        exemptions => [],
    });

    is(
        $result->{diagnostics}[0]{details}{build_path},
        'code/packages/rust/demo/BUILD',
        'component ancestor and fixed BUILD filename order choose the diagnostic path',
    );
};

subtest 'orphan-crate validator reserves NFC full-fold identity before precedence' => sub {
    my $result = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot({
        directories => ['code/packages/rust/Straße'],
        manifests => [{ path => 'code/packages/rust/Straße', kind => 'package' }],
        build_files => [],
        exemptions => [
            { line => 7, kind => 'UNKNOWN', path => 'code/packages/rust/Straße', reason => 'first' },
            { line => 8, kind => 'PENDING', path => 'CODE/PACKAGES/RUST/STRASSE', reason => 'duplicate' },
        ],
    });
    my @invalid = map { $_->{details} }
        grep { $_->{code} eq 'ORPHAN_EXEMPTION_INVALID' } @{$result->{diagnostics}};

    is(
        \@invalid,
        [
            { line => 7, problem => 'UNKNOWN_KIND' },
            { line => 8, problem => 'DUPLICATE_PATH' },
        ],
        'the first portable spelling reserves the duplicate identity even when otherwise invalid',
    );
};

subtest 'orphan-crate validator uses ASCII JSON Unicode detail ordering' => sub {
    my $accented = 'code/packages/rust/é';
    my $emoji = 'code/packages/rust/😀';
    my $result = CodingAdventures::BuildTool::Validator::validate_orphan_crate_snapshot({
        directories => [],
        manifests => [],
        build_files => [],
        exemptions => [
            { line => 9, kind => 'EXCLUDED', path => 'code/packages/rust/z', reason => 'removed' },
            { line => 8, kind => 'EXCLUDED', path => $emoji, reason => 'removed' },
            { line => 7, kind => 'EXCLUDED', path => $accented, reason => 'removed' },
        ],
    });

    is(
        [map { $_->{details}{entry_path} } @{$result->{diagnostics}}],
        [$accented, $emoji, 'code/packages/rust/z'],
        'canonical ASCII JSON details define the portable final sort key',
    );
};

subtest 'tracked-artifact validator consumes every language-neutral fixture' => sub {
    my $repo_root = abs_path("$Bin/../../../../..");
    my $cases = "$repo_root/code/specs/fixtures/build-tool-v1/cases";

    for my $name (qw(clean forbidden aliases invalid unicode-boundaries)) {
        my $fixture = read_json("$cases/validation-tracked-artifacts-$name.json");
        my $snapshot = $fixture->{input}{options}{tracked_artifact_snapshot};
        my $actual = CodingAdventures::BuildTool::Validator::validate_tracked_artifact_snapshot(
            $snapshot->{entries},
            $snapshot->{unicode_version},
        );
        is($actual, $fixture->{expected}{diagnostics}, "$name diagnostics match");
    }
};

subtest 'tracked-artifact validator rejects Unicode version drift' => sub {
    like(
        dies {
            CodingAdventures::BuildTool::Validator::validate_tracked_artifact_snapshot(
                [],
                '16.0.0',
            );
        },
        qr/tracked artifact Unicode version must be 17\.0\.0/,
        'version mismatch fails closed',
    );
};

done_testing;
