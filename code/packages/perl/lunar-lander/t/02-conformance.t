use strict;
use warnings;
use utf8;
use Test2::V0;
use JSON::PP;
use FindBin;

use CodingAdventures::LunarLander qw(
    new_flight
    validate_burn
    step_burn
);

my @fixture_candidates = (
    "$FindBin::Bin/../../../../specs/fixtures/lunar-lander-v1/cases.json",
    "$FindBin::Bin/../../../specs/fixtures/lunar-lander-v1/cases.json",
    '../../../specs/fixtures/lunar-lander-v1/cases.json',
    '../../../../specs/fixtures/lunar-lander-v1/cases.json',
    'code/specs/fixtures/lunar-lander-v1/cases.json',
    '.claude/worktrees/game02-lunar-lander-tier2/code/specs/fixtures/lunar-lander-v1/cases.json',
);

my ($fixture_path) = grep { -f $_ } @fixture_candidates;
ok($fixture_path, "Found fixture cases.json at $fixture_path");

my $json_text = do {
    open my $fh, '<:raw', $fixture_path or die "Cannot open $fixture_path: $!";
    local $/;
    <$fh>;
};
my $corpus = decode_json($json_text);
is($corpus->{version}, 1, 'Corpus version is 1');
is($corpus->{suite}, 'lunar_lander_deterministic_flight', 'Suite is lunar_lander_deterministic_flight');

my $cases = $corpus->{cases};
is(scalar(@$cases), 39, 'Corpus has exactly 39 test cases');

for my $c (@$cases) {
    subtest $c->{id} => sub {
        my $initial = $c->{initial_state};

        if ($c->{expected_error}) {
            # Refusal test case
            my $cmd = $c->{command};

            my $err_val = dies { validate_burn($initial, $cmd) };
            ok($err_val, 'validate_burn dies on invalid state/command');
            is($err_val ? $err_val->kind() : '', $c->{expected_error}, "validate_burn error kind matches $c->{expected_error}");

            my $err_step = dies { step_burn($initial, $cmd) };
            ok($err_step, 'step_burn dies on invalid state/command');
            is($err_step ? $err_step->kind() : '', $c->{expected_error}, "step_burn error kind matches $c->{expected_error}");
        } elsif ($c->{turns}) {
            # Multi-turn trajectory integration
            my $state = $initial;
            for my $turn (@{ $c->{turns} }) {
                my $turn_num = $turn->{turn};
                my $cmd = $turn->{command};
                my $expected_state = $turn->{expected_state};

                my $valid = lives { validate_burn($state, $cmd) };
                ok($valid, "Turn $turn_num validate_burn lives");

                my $next = eval { step_burn($state, $cmd) };
                ok($next, "Turn $turn_num step_burn succeeds") or diag($@);

                is($next->{time_seconds}, $expected_state->{time_seconds}, "Turn $turn_num time_seconds match");
                is($next->{altitude_mft}, $expected_state->{altitude_mft}, "Turn $turn_num altitude_mft match");
                is($next->{velocity_mft_s}, $expected_state->{velocity_mft_s}, "Turn $turn_num velocity_mft_s match");
                is($next->{fuel_lbs}, $expected_state->{fuel_lbs}, "Turn $turn_num fuel_lbs match");
                is($next->{dry_mass_lbs}, $expected_state->{dry_mass_lbs}, "Turn $turn_num dry_mass_lbs match");
                is($next->{status}, $expected_state->{status}, "Turn $turn_num status match");

                if (defined $expected_state->{impact_velocity_mft_s}) {
                    is($next->{impact_velocity_mft_s}, $expected_state->{impact_velocity_mft_s}, "Turn $turn_num impact_velocity_mft_s match");
                } else {
                    ok(!defined $next->{impact_velocity_mft_s}, "Turn $turn_num impact_velocity_mft_s is undef");
                }

                $state = $next;
            }
        }
    };
}

done_testing;
