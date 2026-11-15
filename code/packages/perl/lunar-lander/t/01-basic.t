use strict;
use warnings;
use utf8;
use Test2::V0;
use Math::BigInt;

use CodingAdventures::LunarLander qw(
    new_flight
    validate_burn
    check_burn
    step_burn
    div_euclid
    isqrt_u128
    to_paint_scene
    telemetry_scene
    render_telemetry_ascii
    render_telemetry
    format_with_commas
    G_LUNAR_MFT_S2
    DRY_MASS_LBS
    INITIAL_FUEL_LBS
    TOTAL_INITIAL_MASS_LBS
    INITIAL_ALTITUDE_MFT
    INITIAL_VELOCITY_MFT_S
    EXHAUST_VELOCITY_MFT_S
    MAX_BURN_RATE_LBS_S
    MIN_DURATION_S
    MAX_DURATION_S
    MAX_TIME_SECONDS
    MAX_ALTITUDE_MFT
    MIN_VELOCITY_MFT_S
    MAX_VELOCITY_MFT_S
);

subtest 'Initial flight invariants' => sub {
    my $flight = new_flight();
    is($flight->{time_seconds}, 0, 'time_seconds is 0');
    is($flight->{altitude_mft}, 120000000, 'initial altitude is 120,000,000 mft');
    is($flight->{velocity_mft_s}, -3000000, 'initial velocity is -3,000,000 mft/s');
    is($flight->{fuel_lbs}, 16500, 'initial fuel is 16,500 lbs');
    is($flight->{dry_mass_lbs}, 16500, 'dry mass is 16,500 lbs');
    is($flight->{status}, 'InFlight', 'status is InFlight');
    ok(!defined $flight->{impact_velocity_mft_s}, 'impact_velocity_mft_s is undef');
};

subtest 'div_euclid mathematical correctness' => sub {
    # Non-negative remainder Euclidean division
    is(div_euclid(7, 3), 2, '7 / 3 == 2');
    is(div_euclid(-7, 3), -3, '-7 / 3 == -3 (remainder 2)');
    is(div_euclid(7, -3), -2, '7 / -3 == -2 (remainder 1)');
    is(div_euclid(-7, -3), 3, '-7 / -3 == 3 (remainder 2)');

    # Key touchdown solver vector from Turn 4 fixture
    is(div_euclid(-46070, -5310), 9, 'div_euclid(-46070, -5310) == 9');

    # Division by zero
    my $err_zero = dies { div_euclid(10, 0) };
    ok($err_zero, 'div_euclid dies on zero divisor');
    is($err_zero->kind(), 'ArithmeticOverflow', 'division by zero gives ArithmeticOverflow');
};

subtest 'isqrt_u128 exactness' => sub {
    is(isqrt_u128(0), 0, 'isqrt(0) == 0');
    is(isqrt_u128(1), 1, 'isqrt(1) == 1');
    is(isqrt_u128(2), 1, 'isqrt(2) == 1');
    is(isqrt_u128(3), 1, 'isqrt(3) == 1');
    is(isqrt_u128(4), 2, 'isqrt(4) == 2');
    is(isqrt_u128(15), 3, 'isqrt(15) == 3');
    is(isqrt_u128(16), 4, 'isqrt(16) == 4');
    is(isqrt_u128(10274400000000), 3205370, 'isqrt(10274400000000) == 3205370');

    # 128-bit edge cases
    my $u64_max_sq = Math::BigInt->new('18446744073709551615')->bpow(2);
    is("" . isqrt_u128($u64_max_sq), '18446744073709551615', 'isqrt(u64::MAX ^ 2) == u64::MAX');

    my $u128_max = Math::BigInt->new('340282366920938463463374607431768211455');
    is("" . isqrt_u128($u128_max), '18446744073709551615', 'isqrt(u128::MAX) == u64::MAX');
};

subtest 'All four error arms and canonical precedence' => sub {
    my $state = new_flight();
    my $valid_cmd = { burn_rate_lbs_s => 100, duration_seconds => 5 };

    # Arm 1: FlightAlreadyEnded
    {
        my $ended_state = { %$state, status => 'Landed(Perfect)' };
        my $err = dies { validate_burn($ended_state, $valid_cmd) };
        ok($err, 'validate_burn dies on ended flight');
        is($err->kind(), 'FlightAlreadyEnded', 'Arm 1: FlightAlreadyEnded');

        my $err_step = dies { step_burn($ended_state, $valid_cmd) };
        ok($err_step, 'step_burn dies on ended flight');
        is($err_step->kind(), 'FlightAlreadyEnded', 'step_burn Arm 1: FlightAlreadyEnded');
    }

    # Arm 2: ExcessiveBurnRate
    {
        my $bad_cmd = { burn_rate_lbs_s => 201, duration_seconds => 5 };
        my $err = dies { validate_burn($state, $bad_cmd) };
        ok($err, 'validate_burn dies on excessive burn rate');
        is($err->kind(), 'ExcessiveBurnRate', 'Arm 2: ExcessiveBurnRate');
    }

    # Arm 3: InvalidDuration
    {
        my $zero_dur = { burn_rate_lbs_s => 100, duration_seconds => 0 };
        my $err0 = dies { validate_burn($state, $zero_dur) };
        ok($err0, 'validate_burn dies on 0 duration');
        is($err0->kind(), 'InvalidDuration', 'Arm 3a: InvalidDuration (0)');

        my $excess_dur = { burn_rate_lbs_s => 100, duration_seconds => 11 };
        my $err11 = dies { validate_burn($state, $excess_dur) };
        ok($err11, 'validate_burn dies on 11 duration');
        is($err11->kind(), 'InvalidDuration', 'Arm 3b: InvalidDuration (11)');
    }

    # Arm 4: ArithmeticOverflow (corrupted state or time overflow)
    {
        my $bad_mass = { %$state, dry_mass_lbs => 0 };
        my $err_mass = dies { validate_burn($bad_mass, $valid_cmd) };
        ok($err_mass, 'validate_burn dies on corrupted mass');
        is($err_mass->kind(), 'ArithmeticOverflow', 'Arm 4a: ArithmeticOverflow (corrupted dry mass)');

        my $time_state = { %$state, time_seconds => 86400 };
        my $step_cmd = { burn_rate_lbs_s => 0, duration_seconds => 1 };
        my $err_time = dies { validate_burn($time_state, $step_cmd) };
        ok($err_time, 'validate_burn dies on time overflow');
        is($err_time->kind(), 'ArithmeticOverflow', 'Arm 4b: ArithmeticOverflow (time overflow)');
    }
};

subtest 'check_burn non-throwing interface' => sub {
    my $state = new_flight();
    my $good_cmd = { burn_rate_lbs_s => 50, duration_seconds => 5 };
    my ($ok1, $err1) = check_burn($state, $good_cmd);
    ok($ok1, 'valid burn returns truthy ok');
    ok(!defined $err1, 'valid burn has undef err');

    my $bad_cmd = { burn_rate_lbs_s => 250, duration_seconds => 5 };
    my ($ok2, $err2) = check_burn($state, $bad_cmd);
    ok(!$ok2, 'invalid burn returns falsey ok');
    is($err2->kind(), 'ExcessiveBurnRate', 'invalid burn returns LanderError');
};

subtest 'format_with_commas helper' => sub {
    is(format_with_commas(0), '0', '0');
    is(format_with_commas(5), '5', '5');
    is(format_with_commas(100), '100', '100');
    is(format_with_commas(1000), '1,000', '1,000');
    is(format_with_commas(120000), '120,000', '120,000');
    is(format_with_commas(-3000), '-3,000', '-3,000');
    is(format_with_commas(-12345678), '-12,345,678', '-12,345,678');
};

subtest 'PaintScene and render_telemetry_ascii 5x76 character grid' => sub {
    my $state = new_flight();
    my $scene = to_paint_scene($state, 0);
    is($scene->{width}, 76.0, 'scene width is 76');
    is($scene->{height}, 5.0, 'scene height is 5');

    my $hud = render_telemetry_ascii($state, 0);
    my @lines = split /\n/, $hud;
    is(scalar(@lines), 5, 'HUD has exactly 5 lines');

    for my $i (0 .. $#lines) {
        my $line = $lines[$i];
        # With utf8 enabled, length() returns character count
        is(length($line), 76, "Line $i character count is exactly 76");
    }

    like($hud, qr/LUNAR LANDER TELEMETRY/, 'header present');
    like($hud, qr/TIME:/, 'TIME: present');
    like($hud, qr/ALTITUDE:/, 'ALTITUDE: present');
    like($hud, qr/VELOCITY:/, 'VELOCITY: present');
    like($hud, qr/FUEL:/, 'FUEL: present');
    like($hud, qr/BURN:/, 'BURN: present');
    like($hud, qr/RATE:/, 'RATE: present');
    like($hud, qr/DESCENT/, 'DESCENT present');
    like($hud, qr/█/, 'solid bar present');

    # Mid-flight state with partial gauge
    my $mid_state = {
        %$state,
        fuel_lbs     => 8250,
        altitude_mft => 60000000,
    };
    my $hud_mid = render_telemetry_ascii($mid_state, 100);
    like($hud_mid, qr/█/, 'partial gauge contains solid bar');
    like($hud_mid, qr/░/, 'partial gauge contains empty track');

    # Convenience helpers
    my $scene2 = telemetry_scene($state);
    is($scene2->{width}, 76.0, 'telemetry_scene width is 76');
    my $hud2 = render_telemetry($state);
    is($hud2, $hud, 'render_telemetry matches render_telemetry_ascii(state, 0)');
};

subtest 'Telemetry states and landing outcomes' => sub {
    my $state = new_flight();

    # Ascent
    my $ascent_state = { %$state, velocity_mft_s => 50000 };
    my $hud_ascent = render_telemetry_ascii($ascent_state, 100);
    like($hud_ascent, qr/ASCENT/, 'ascent shows ASCENT');
    like($hud_ascent, qr/▲/, 'ascent shows ▲');

    # Hover
    my $hover_state = { %$state, velocity_mft_s => 0 };
    my $hud_hover = render_telemetry_ascii($hover_state, 50);
    like($hud_hover, qr/HOVER/, 'hover shows HOVER');
    like($hud_hover, qr/◆/, 'hover shows ◆');

    # Descent
    my $descent_state = { %$state, velocity_mft_s => -50000 };
    my $hud_descent = render_telemetry_ascii($descent_state, 50);
    like($hud_descent, qr/DESCENT/, 'descent shows DESCENT');
    like($hud_descent, qr/▼/, 'descent shows ▼');

    # Landed outcomes
    my @outcomes = (
        ['Perfect', qr/PERFECT/],
        ['Hard',    qr/HARD/],
        ['Crash',   qr/CRASH/],
        ['Fatal',   qr/FATAL/],
        ['Crater',  qr/CRATER/],
    );

    for my $row (@outcomes) {
        my ($outcome, $expected_pattern) = @$row;
        my $landed_state = {
            %$state,
            status                => "Landed($outcome)",
            impact_velocity_mft_s => -1000,
        };
        my $hud_landed = render_telemetry_ascii($landed_state, 0);
        like($hud_landed, qr/STATUS:/, "$outcome HUD contains STATUS:");
        like($hud_landed, $expected_pattern, "$outcome HUD contains outcome text");
        like($hud_landed, qr/★/, "$outcome HUD contains ★ symbol");
    }
};

done_testing;
