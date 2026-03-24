#!/usr/bin/env perl

# t/06-reporter.t -- Tests for CodingAdventures::BuildTool::Reporter
# ===================================================================
#
# 5 test cases covering report formatting.

use strict;
use warnings;
use FindBin qw($Bin);
use lib "$Bin/../lib";

use Test2::V0;

use CodingAdventures::BuildTool::Reporter;

# Capture STDOUT output from a block.
# Uses select() to redirect the default output handle without touching STDOUT.
sub capture_stdout {
    my ($code) = @_;
    my $output = '';
    open(my $capture, '>', \$output) or die "Cannot open capture handle: $!";
    my $old_fh = select $capture;
    eval { $code->() };
    my $err = $@;
    select $old_fh;
    close $capture;
    die $err if $err;
    return $output;
}

my @all_pass = (
    { package => 'perl/logic-gates', status => 'pass', duration => 2.1, output => '' },
    { package => 'perl/arithmetic',  status => 'pass', duration => 1.5, output => '' },
);

my @some_fail = (
    { package => 'perl/logic-gates', status => 'pass', duration => 2.1, output => 'ok' },
    { package => 'perl/arithmetic',  status => 'fail', duration => 3.4, output => 'FAILED: test.t' },
    { package => 'perl/bitset',      status => 'skip', duration => 0,   output => 'dep failed' },
);

# ---------------------------------------------------------------------------
# Test 1: All pass — summary shows 0 failures
# ---------------------------------------------------------------------------
subtest 'all pass summary' => sub {
    my $r = CodingAdventures::BuildTool::Reporter->new(colour => 0);
    my $out = capture_stdout(sub { $r->summary(\@all_pass) });

    like($out, qr/Passed:\s+2/, 'shows 2 passed');
    like($out, qr/Failed:\s+0/, 'shows 0 failed');
    unlike($out, qr/Failed packages:/, 'no failed packages list');
};

# ---------------------------------------------------------------------------
# Test 2: Some fail — failed packages listed
# ---------------------------------------------------------------------------
subtest 'some fail lists failed packages' => sub {
    my $r = CodingAdventures::BuildTool::Reporter->new(colour => 0);
    my $out = capture_stdout(sub {
        $r->report(\@some_fail);
        $r->summary(\@some_fail);
    });

    like($out, qr/FAIL.*arithmetic/, 'arithmetic shown as FAIL');
    like($out, qr/Failed packages:/,  'failed packages section present');
    like($out, qr/arithmetic/,        'arithmetic listed in failures');
};

# ---------------------------------------------------------------------------
# Test 3: Dep-skipped packages shown as SKIP
# ---------------------------------------------------------------------------
subtest 'skipped packages shown' => sub {
    my $r   = CodingAdventures::BuildTool::Reporter->new(colour => 0);
    my $out = capture_stdout(sub { $r->report(\@some_fail) });

    like($out, qr/SKIP.*bitset/, 'bitset shown as SKIP');
    like($out, qr/\[SKIP\]/,    'SKIP label appears in report output');
};

# ---------------------------------------------------------------------------
# Test 4: Timing shown per package
# ---------------------------------------------------------------------------
subtest 'timing shown per package' => sub {
    my $r   = CodingAdventures::BuildTool::Reporter->new(colour => 0);
    my $out = capture_stdout(sub { $r->report(\@all_pass) });

    like($out, qr/2\.1s/, 'duration 2.1s shown');
    like($out, qr/1\.5s/, 'duration 1.5s shown');
};

# ---------------------------------------------------------------------------
# Test 5: Empty results → "Nothing to build"
# ---------------------------------------------------------------------------
subtest 'empty results prints nothing to build' => sub {
    my $r   = CodingAdventures::BuildTool::Reporter->new(colour => 0);
    my $out = capture_stdout(sub { $r->report([]) });

    like($out, qr/nothing to build/i, '"Nothing to build" message');
};

done_testing();
