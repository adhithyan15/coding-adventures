use strict;
use warnings;
use Test2::V0;
use JSON::PP;
use FindBin;

use CodingAdventures::Hamurabi qw(
    new_game
    validate_order
    step_year
    format_report
);

my @fixture_candidates = (
    "$FindBin::Bin/../../../../specs/fixtures/hamurabi-v1/cases.json",
    "$FindBin::Bin/../../../specs/fixtures/hamurabi-v1/cases.json",
    '../../../specs/fixtures/hamurabi-v1/cases.json',
    '../../../../specs/fixtures/hamurabi-v1/cases.json',
    'code/specs/fixtures/hamurabi-v1/cases.json',
    '.claude/worktrees/game01-hamurabi-tier2/code/specs/fixtures/hamurabi-v1/cases.json',
);

my ($fixture_path) = grep { -f $_ } @fixture_candidates;
ok($fixture_path, "Found fixture cases.json at $fixture_path");

my $json_text = do {
    open my $fh, '<', $fixture_path or die "Cannot open $fixture_path: $!";
    local $/;
    <$fh>;
};
my $corpus = decode_json($json_text);
is($corpus->{version}, 1, 'Corpus version is 1');
is($corpus->{suite}, 'hamurabi_deterministic_reign', 'Suite is hamurabi_deterministic_reign');

my $cases = $corpus->{cases};
ok(scalar(@$cases) >= 31, 'Corpus has at least 31 test cases');

for my $c (@$cases) {
    subtest $c->{id} => sub {
        my $initial = $c->{initial_state};

        if ($c->{expected_error}) {
            # Refusal case
            my $err_val = dies { validate_order($initial, $c->{order}) };
            ok($err_val, 'validate_order fails on invalid order');
            is($err_val->kind(), $c->{expected_error}, "validate_order error kind is $c->{expected_error}");

            my $err_step = dies { step_year($initial, $c->{order}) };
            ok($err_step, 'step_year fails on invalid order');
            is($err_step->kind(), $c->{expected_error}, "step_year error kind is $c->{expected_error}");
        } elsif ($c->{turns}) {
            # Multi-turn or single-turn game progression
            my $state = $initial;
            for my $turn (@{ $c->{turns} }) {
                my $turn_num = $turn->{turn};
                my $order = $turn->{order};
                my $expected_state = $turn->{expected_state};

                my $valid = lives { validate_order($state, $order) };
                ok($valid, "Turn $turn_num validate_order lives");

                my $next = step_year($state, $order);

                # Compare all state fields
                for my $k (sort keys %$expected_state) {
                    if ($k eq 'rng_state' || $k eq 'rng_inc') {
                        is("$next->{$k}", "$expected_state->{$k}", "Turn $turn_num $k bit-exact match");
                    } elsif ($k eq 'plague_struck') {
                        is($next->{$k} ? 1 : 0, $expected_state->{$k} ? 1 : 0, "Turn $turn_num plague_struck match");
                    } else {
                        is($next->{$k}, $expected_state->{$k}, "Turn $turn_num $k match");
                    }
                }

                # Compare report formatting if present
                if (defined $turn->{expected_report}) {
                    my $report = format_report($next);
                    is($report, $turn->{expected_report}, "Turn $turn_num teletype report matches exactly");
                }

                $state = $next;
            }
        }
    };
}

done_testing;
