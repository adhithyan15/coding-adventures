use strict;
use warnings;
use utf8;
use Test2::V0;
use JSON::PP;
use Math::BigInt;

use CodingAdventures::LunarLander qw(
    new_flight
    validate_burn
    check_burn
    step_burn
    MAX_BURN_RATE_LBS_S
    MAX_DURATION_S
    MIN_DURATION_S
    MAX_TIME_SECONDS
    MAX_ALTITUDE_MFT
    MIN_VELOCITY_MFT_S
    MAX_VELOCITY_MFT_S
    INITIAL_FUEL_LBS
    DRY_MASS_LBS
);

# ============================================================================
# CWE-190: Integer Overflow and Boundary Guards
# ============================================================================

subtest 'CWE-190: Integer overflow and coordinate boundary guards' => sub {
    my $state = new_flight();
    my $valid_cmd = { burn_rate_lbs_s => 100, duration_seconds => 5 };

    # 1. Hostile altitude boundaries
    for my $bad_alt (
        MAX_ALTITUDE_MFT + 1,
        '9223372036854775807',
        '-9223372036854775808',
        Math::BigInt->new('9223372036854775808'),
        -1,
        0,
    ) {
        my $bad_state = { %$state, altitude_mft => $bad_alt };
        my $err = dies { validate_burn($bad_state, $valid_cmd) };
        ok($err, "altitude $bad_alt is refused");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "altitude $bad_alt raises ArithmeticOverflow");
    }

    # 2. Hostile velocity boundaries
    for my $bad_vel (
        MIN_VELOCITY_MFT_S - 1,
        MAX_VELOCITY_MFT_S + 1,
        '9223372036854775807',
        '-9223372036854775808',
        Math::BigInt->new('9223372036854775808'),
    ) {
        my $bad_state = { %$state, velocity_mft_s => $bad_vel };
        my $err = dies { validate_burn($bad_state, $valid_cmd) };
        ok($err, "velocity $bad_vel is refused");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "velocity $bad_vel raises ArithmeticOverflow");
    }

    # 3. Hostile fuel boundaries
    for my $bad_fuel (
        INITIAL_FUEL_LBS + 1,
        4294967295,
        -1,
        Math::BigInt->new('4294967296'),
    ) {
        my $bad_state = { %$state, fuel_lbs => $bad_fuel };
        my $err = dies { validate_burn($bad_state, $valid_cmd) };
        ok($err, "fuel $bad_fuel is refused");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "fuel $bad_fuel raises ArithmeticOverflow");
    }

    # 4. Hostile time boundaries
    for my $bad_time (
        MAX_TIME_SECONDS + 1,
        4294967295,
        -1,
    ) {
        my $bad_state = { %$state, time_seconds => $bad_time };
        my $err = dies { validate_burn($bad_state, $valid_cmd) };
        ok($err, "time $bad_time is refused");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "time $bad_time raises ArithmeticOverflow");
    }

    # Time sum overflow
    {
        my $near_end_state = { %$state, time_seconds => MAX_TIME_SECONDS };
        my $add_1s = { burn_rate_lbs_s => 0, duration_seconds => 1 };
        my $err = dies { validate_burn($near_end_state, $add_1s) };
        ok($err, 'time sum overflow beyond 86400 is refused');
        is($err ? $err->kind() : '', 'ArithmeticOverflow', 'time sum overflow raises ArithmeticOverflow');
    }

    # 5. Hostile command boundaries
    {
        my $cmd_huge_burn = { burn_rate_lbs_s => 201, duration_seconds => 5 };
        my $err_b = dies { validate_burn($state, $cmd_huge_burn) };
        ok($err_b, 'burn rate > 200 refused');
        is($err_b ? $err_b->kind() : '', 'ExcessiveBurnRate', 'burn rate 201 raises ExcessiveBurnRate');

        my $cmd_zero_dur = { burn_rate_lbs_s => 50, duration_seconds => 0 };
        my $err_d0 = dies { validate_burn($state, $cmd_zero_dur) };
        ok($err_d0, 'duration 0 refused');
        is($err_d0 ? $err_d0->kind() : '', 'InvalidDuration', 'duration 0 raises InvalidDuration');

        my $cmd_huge_dur = { burn_rate_lbs_s => 50, duration_seconds => 11 };
        my $err_dh = dies { validate_burn($state, $cmd_huge_dur) };
        ok($err_dh, 'duration 11 refused');
        is($err_dh ? $err_dh->kind() : '', 'InvalidDuration', 'duration 11 raises InvalidDuration');

        my $cmd_u32_dur = { burn_rate_lbs_s => 50, duration_seconds => 4294967295 };
        my $err_du32 = dies { validate_burn($state, $cmd_u32_dur) };
        ok($err_du32, 'duration u32::MAX refused');
        is($err_du32 ? $err_du32->kind() : '', 'InvalidDuration', 'duration u32::MAX raises InvalidDuration');
    }
};

# ============================================================================
# CWE-369: Division by Zero Prevention
# ============================================================================

subtest 'CWE-369: Division by zero prevention' => sub {
    my $valid_cmd = { burn_rate_lbs_s => 100, duration_seconds => 5 };

    # Corrupted dry mass 0
    my $zero_mass_state = {
        time_seconds          => 0,
        altitude_mft          => 100000000,
        velocity_mft_s        => -2000000,
        fuel_lbs              => 0,
        dry_mass_lbs          => 0,
        status                => 'InFlight',
        impact_velocity_mft_s => undef,
    };
    my $err1 = dies { validate_burn($zero_mass_state, $valid_cmd) };
    ok($err1, 'dry_mass == 0 is refused by validate_burn');
    is($err1 ? $err1->kind() : '', 'ArithmeticOverflow', 'corrupted mass raises ArithmeticOverflow');

    my $err2 = dies { step_burn($zero_mass_state, $valid_cmd) };
    ok($err2, 'dry_mass == 0 is refused by step_burn');
    is($err2 ? $err2->kind() : '', 'ArithmeticOverflow', 'step_burn with mass 0 raises ArithmeticOverflow');
};

# ============================================================================
# CWE-843: Type Confusion & Strict Integer Validation
# ============================================================================

subtest 'CWE-843: Rejection of floats, strings, undef, refs, boolean objects' => sub {
    my $state = new_flight();
    my $cmd = { burn_rate_lbs_s => 50, duration_seconds => 5 };

    my @adversarial_inputs = (
        1.5,
        100.0,
        0.9999999999999999,
        "100 lbs",
        "100\0malicious",
        " 100 ",
        "100\n",
        "NaN",
        "Infinity",
        "-Inf",
        undef,
        [],
        {},
        sub { 42 },
        \1,
        JSON::PP::true,
        JSON::PP::false,
    );

    # Test state fields
    for my $field (qw(time_seconds altitude_mft velocity_mft_s fuel_lbs dry_mass_lbs)) {
        for my $bad (@adversarial_inputs) {
            my $desc = defined $bad ? (ref($bad) ? ref($bad) : "'$bad'") : 'undef';
            my $bad_state = { %$state, $field => $bad };

            my $err = dies { validate_burn($bad_state, $cmd) };
            ok($err, "rejects bad state field $field: $desc");
            is($err ? $err->kind() : '', 'ArithmeticOverflow', "bad state field $field raises ArithmeticOverflow");
        }
    }

    # Test command fields: burn_rate_lbs_s
    for my $bad (@adversarial_inputs) {
        my $desc = defined $bad ? (ref($bad) ? ref($bad) : "'$bad'") : 'undef';
        my $bad_cmd = { %$cmd, burn_rate_lbs_s => $bad };

        my $err = dies { validate_burn($state, $bad_cmd) };
        ok($err, "rejects bad burn_rate_lbs_s: $desc");
    }

    # Test command fields: duration_seconds
    for my $bad (@adversarial_inputs) {
        my $desc = defined $bad ? (ref($bad) ? ref($bad) : "'$bad'") : 'undef';
        my $bad_cmd = { %$cmd, duration_seconds => $bad };

        my $err = dies { validate_burn($state, $bad_cmd) };
        ok($err, "rejects bad duration_seconds: $desc");
    }

    # Non-hash state or command
    ok(dies { validate_burn(undef, $cmd) }, 'undef state is rejected');
    ok(dies { validate_burn("state", $cmd) }, 'string state is rejected');
    ok(dies { validate_burn($state, undef) }, 'undef cmd is rejected');
    ok(dies { validate_burn($state, "cmd") }, 'string cmd is rejected');

    # Missing state keys
    for my $key (qw(time_seconds altitude_mft velocity_mft_s fuel_lbs dry_mass_lbs status)) {
        my $missing_state = { %$state };
        delete $missing_state->{$key};
        my $err = dies { validate_burn($missing_state, $cmd) };
        ok($err, "missing state key $key is rejected");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "missing state key $key raises ArithmeticOverflow");
    }

    # Status validation
    for my $bad_status ('InvalidStatus', '', 'InFlightExtra', 'Landed', 'Landed()', [], {}) {
        my $desc = ref($bad_status) ? ref($bad_status) : "'$bad_status'";
        my $bad_state = { %$state, status => $bad_status };
        my $err = dies { validate_burn($bad_state, $cmd) };
        ok($err, "rejects bad status $desc");
        is($err ? $err->kind() : '', 'ArithmeticOverflow', "status $desc raises ArithmeticOverflow");
    }

    # Valid landed statuses raise FlightAlreadyEnded
    for my $outcome (qw(Perfect Hard Crash Fatal Crater)) {
        my $ended_state = { %$state, status => "Landed($outcome)" };
        my $err = dies { validate_burn($ended_state, $cmd) };
        ok($err, "ended status Landed($outcome) is rejected");
        is($err ? $err->kind() : '', 'FlightAlreadyEnded', "ended status raises FlightAlreadyEnded");
    }
};

# ============================================================================
# VER04m: State Immutability
# ============================================================================

subtest 'VER04m: step_burn does not mutate input state or command' => sub {
    my $state = new_flight();
    my $state_copy = { %$state };
    my $cmd = { burn_rate_lbs_s => 50, duration_seconds => 10 };
    my $cmd_copy = { %$cmd };

    my $next = step_burn($state, $cmd);

    # Original state must be completely untouched
    is($state, $state_copy, 'state hashref was not mutated in place');
    is($cmd, $cmd_copy, 'cmd hashref was not mutated in place');

    # Returned state is a distinct new hashref
    ok($next != $state, 'next state is a new reference');
    ok($next->{time_seconds} > $state->{time_seconds}, 'next time has advanced');
    ok($next->{fuel_lbs} < $state->{fuel_lbs}, 'fuel was consumed');
};

done_testing;
