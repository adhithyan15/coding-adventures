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

done_testing;
