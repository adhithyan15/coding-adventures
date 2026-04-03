#!/usr/bin/env perl

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;
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

done_testing;
