use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ProgressBar;

# ============================================================================
# Tests for CodingAdventures::ProgressBar
# ============================================================================
#
# We test:
#   1. Constants (STARTED, FINISHED, SKIPPED)
#   2. percentage() utility function
#   3. bar_string() utility function
#   4. _format_activity() internal formatting
#   5. Tracker: new, start, send, stop behavior
#   6. Tracker: child/parent hierarchical mode
#   7. Output capture tests

# ============================================================================
# Constants
# ============================================================================

subtest 'Constants: STARTED, FINISHED, SKIPPED are defined strings' => sub {
    is(CodingAdventures::ProgressBar::STARTED,  'STARTED',  'STARTED constant');
    is(CodingAdventures::ProgressBar::FINISHED, 'FINISHED', 'FINISHED constant');
    is(CodingAdventures::ProgressBar::SKIPPED,  'SKIPPED',  'SKIPPED constant');
};

# ============================================================================
# percentage() utility
# ============================================================================

subtest 'percentage: 0 of N is 0%' => sub {
    is(CodingAdventures::ProgressBar->percentage(0, 10), 0, '0/10 = 0%');
};

subtest 'percentage: N of N is 100%' => sub {
    is(CodingAdventures::ProgressBar->percentage(10, 10), 100, '10/10 = 100%');
};

subtest 'percentage: 7 of 21 is 33%' => sub {
    is(CodingAdventures::ProgressBar->percentage(7, 21), 33, '7/21 = 33%');
};

subtest 'percentage: 1 of 3 is 33%' => sub {
    is(CodingAdventures::ProgressBar->percentage(1, 3), 33, '1/3 = 33%');
};

subtest 'percentage: zero total returns 0' => sub {
    is(CodingAdventures::ProgressBar->percentage(0, 0), 0, '0/0 returns 0 (no div-by-zero)');
};

subtest 'percentage: capped at 100' => sub {
    is(CodingAdventures::ProgressBar->percentage(200, 100), 100, '200/100 capped at 100');
};

# ============================================================================
# bar_string() utility
# ============================================================================

subtest 'bar_string: 0% is all empty' => sub {
    my $bar = CodingAdventures::ProgressBar->bar_string(0, 10, '#', '.');
    is($bar, '..........', '0% bar is all empty chars');
    is(length($bar), 10, 'bar length is 10');
};

subtest 'bar_string: 100% is all filled' => sub {
    my $bar = CodingAdventures::ProgressBar->bar_string(100, 10, '#', '.');
    is($bar, '##########', '100% bar is all filled chars');
};

subtest 'bar_string: 50% fills half' => sub {
    my $bar = CodingAdventures::ProgressBar->bar_string(50, 10, '#', '.');
    is($bar, '#####.....', '50% bar half filled');
};

subtest 'bar_string: uses Unicode blocks by default' => sub {
    my $bar = CodingAdventures::ProgressBar->bar_string(50, 4);
    # Default fill = U+2588, empty = U+2591
    is(length($bar), 4, 'default bar has width 4 chars');
};

# ============================================================================
# _format_activity internal function
# ============================================================================

subtest '_format_activity: no items, not done' => sub {
    my $activity = CodingAdventures::ProgressBar::_format_activity({}, 0, 10);
    is($activity, 'waiting...', 'no in-flight, not done => waiting...');
};

subtest '_format_activity: no items, done' => sub {
    my $activity = CodingAdventures::ProgressBar::_format_activity({}, 10, 10);
    is($activity, 'done', 'no in-flight, all complete => done');
};

subtest '_format_activity: 1 item building' => sub {
    my $activity = CodingAdventures::ProgressBar::_format_activity({'pkg-a' => 1}, 0, 10);
    is($activity, 'Building: pkg-a', 'one item shows name');
};

subtest '_format_activity: 3 items building (max shown)' => sub {
    my $building = { 'a' => 1, 'b' => 1, 'c' => 1 };
    my $activity = CodingAdventures::ProgressBar::_format_activity($building, 0, 10);
    is($activity, 'Building: a, b, c', 'three items all shown');
};

subtest '_format_activity: 4+ items uses "+N more" suffix' => sub {
    my $building = { 'a' => 1, 'b' => 1, 'c' => 1, 'd' => 1 };
    my $activity = CodingAdventures::ProgressBar::_format_activity($building, 0, 10);
    like($activity, qr/Building: a, b, c \+1 more/, '4 items: shows 3 + "+1 more"');
};

subtest '_format_activity: names sorted alphabetically' => sub {
    my $building = { 'z-pkg' => 1, 'a-pkg' => 1 };
    my $activity = CodingAdventures::ProgressBar::_format_activity($building, 0, 10);
    is($activity, 'Building: a-pkg, z-pkg', 'names sorted alphabetically');
};

# ============================================================================
# Tracker: basic lifecycle
# ============================================================================

subtest 'Tracker: new creates tracker with correct initial state' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(10, \$output, '');
    is($t->{total},     10,    'total == 10');
    is($t->{completed}, 0,     'completed starts at 0');
    is($t->{label},     '',    'label is empty');
    is($t->{_started},  0,     'not started yet');
};

subtest 'Tracker: send before start is a no-op' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(5, \$output, '');
    # Don't call start() — send should be no-op
    $t->send({ type => CodingAdventures::ProgressBar::STARTED, name => 'pkg' });
    is($output, '', 'no output before start() is called');
    is($t->{completed}, 0, 'completed unchanged');
};

subtest 'Tracker: STARTED increments building set' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(5, \$output, '');
    $t->start;
    $t->send({ type => CodingAdventures::ProgressBar::STARTED, name => 'pkg-a' });
    ok($t->{building}{'pkg-a'}, 'pkg-a in building set');
    is($t->{completed}, 0, 'completed unchanged after STARTED');
};

subtest 'Tracker: FINISHED increments completed and removes from building' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(5, \$output, '');
    $t->start;
    $t->send({ type => CodingAdventures::ProgressBar::STARTED,  name => 'pkg-a' });
    $t->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'pkg-a', status => 'built' });
    ok(!$t->{building}{'pkg-a'}, 'pkg-a removed from building after FINISHED');
    is($t->{completed}, 1, 'completed == 1 after FINISHED');
};

subtest 'Tracker: SKIPPED increments completed without touching building' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(5, \$output, '');
    $t->start;
    $t->send({ type => CodingAdventures::ProgressBar::SKIPPED, name => 'pkg-b' });
    is($t->{completed}, 1, 'completed == 1 after SKIPPED');
    ok(!$t->{building}{'pkg-b'}, 'pkg-b never entered building set');
};

subtest 'Tracker: stop writes a newline' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(1, \$output, '');
    $t->start;
    $t->stop;
    like($output, qr/\n$/, 'output ends with newline after stop()');
};

subtest 'Tracker: output contains progress numbers' => sub {
    my $output = '';
    my $t = CodingAdventures::ProgressBar->new(5, \$output, '');
    $t->start;
    $t->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'p1', status => 'built' });
    $t->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'p2', status => 'built' });
    like($output, qr/2\/5/, 'output contains "2/5"');
};

# ============================================================================
# Tracker: hierarchical (child) mode
# ============================================================================

subtest 'Tracker: child() creates sub-tracker' => sub {
    my $output = '';
    my $parent = CodingAdventures::ProgressBar->new(3, \$output, 'Level');
    $parent->start;
    my $child = $parent->child(7, 'Package');
    is($child->{total},   7, 'child total == 7');
    is($child->{label},   'Package', 'child label == Package');
    is($child->{_started}, 1, 'child is immediately started');
};

subtest 'Tracker: child finish() advances parent' => sub {
    my $output = '';
    my $parent = CodingAdventures::ProgressBar->new(3, \$output, 'Level');
    $parent->start;
    my $child = $parent->child(2, 'Pkg');
    $child->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'a' });
    $child->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'b' });
    is($child->{completed}, 2, 'child completed == 2');
    $child->finish;
    is($parent->{completed}, 1, 'parent completed advanced to 1 by child finish()');
};

done_testing;
