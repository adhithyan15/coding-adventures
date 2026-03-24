#!/usr/bin/env perl

# t/07-starlark.t -- Tests for CodingAdventures::BuildTool::StarlarkEval
# ========================================================================
#
# 8 test cases for Starlark detection and command generation.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;

use CodingAdventures::BuildTool::StarlarkEval;

my $se = CodingAdventures::BuildTool::StarlarkEval->new();

# ---------------------------------------------------------------------------
# Test 1: Detect perl_library as Starlark
# ---------------------------------------------------------------------------
subtest 'perl_library detected as starlark' => sub {
    my $content = <<'END';
perl_library(
  name = "logic-gates",
  srcs = glob(["lib/**/*.pm"]),
)
END
    ok($se->is_starlark($content), 'perl_library triggers Starlark detection');
};

# ---------------------------------------------------------------------------
# Test 2: load() statement detected as Starlark
# ---------------------------------------------------------------------------
subtest 'load() statement detected' => sub {
    my $content = <<'END';
load("//tools/rules:perl.bzl", "perl_library")

perl_library(name = "foo")
END
    ok($se->is_starlark($content), 'load() triggers Starlark detection');
};

# ---------------------------------------------------------------------------
# Test 3: Plain shell BUILD is not Starlark
# ---------------------------------------------------------------------------
subtest 'plain shell build is not starlark' => sub {
    my $content = <<'END';
cpanm --installdeps --quiet .
prove -l -v t/
END
    ok(!$se->is_starlark($content), 'shell commands not Starlark');
};

# ---------------------------------------------------------------------------
# Test 4: perl_library generates correct commands
# ---------------------------------------------------------------------------
subtest 'perl_library generates cpanm + prove' => sub {
    my @cmds = $se->generate_commands('perl_library');
    is(scalar @cmds, 2, 'two commands');
    like($cmds[0], qr/cpanm/, 'first command uses cpanm');
    like($cmds[1], qr/prove/,  'second command uses prove');
};

# ---------------------------------------------------------------------------
# Test 5: py_library generates uv + pytest
# ---------------------------------------------------------------------------
subtest 'py_library generates uv + pytest' => sub {
    my @cmds = $se->generate_commands('py_library');
    is(scalar @cmds, 2, 'two commands');
    like($cmds[0], qr/uv/,     'first command uses uv');
    like($cmds[1], qr/pytest/, 'second command uses pytest');
};

# ---------------------------------------------------------------------------
# Test 6: go_library generates go build + test + vet
# ---------------------------------------------------------------------------
subtest 'go_library generates go commands' => sub {
    my @cmds = $se->generate_commands('go_library');
    ok(grep { /go build/ } @cmds, 'go build present');
    ok(grep { /go test/  } @cmds, 'go test present');
    ok(grep { /go vet/   } @cmds, 'go vet present');
};

# ---------------------------------------------------------------------------
# Test 7: extract_targets parses name, srcs, deps
# ---------------------------------------------------------------------------
subtest 'extract_targets parses rule correctly' => sub {
    my $content = <<'END';
perl_library(
  name = "logic-gates",
  srcs = glob(["lib/**/*.pm", "lib/*.pm"]),
  deps = ["//code/packages/perl/arithmetic:arithmetic"],
)
END

    my @targets = $se->extract_targets($content);
    is(scalar @targets, 1, 'one target extracted');
    is($targets[0]{rule}, 'perl_library',  'rule is perl_library');
    is($targets[0]{name}, 'logic-gates',   'name is logic-gates');
    is(scalar @{ $targets[0]{srcs} }, 2,   'two srcs patterns');
    is(scalar @{ $targets[0]{deps} }, 1,   'one dep');
};

# ---------------------------------------------------------------------------
# Test 8: Commented rules are ignored
# ---------------------------------------------------------------------------
subtest 'commented rules are ignored' => sub {
    my $content = <<'END';
# perl_library(name = "commented-out")
prove -l -v t/
END

    ok(!$se->is_starlark($content), 'commented rule does not trigger Starlark');
};

done_testing();
