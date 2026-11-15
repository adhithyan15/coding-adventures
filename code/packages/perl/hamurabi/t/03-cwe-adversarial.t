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

# ============================================================================
# CWE-190: Integer Overflow Guards
# ============================================================================

subtest 'CWE-190: Integer overflow guards on orders and calculations' => sub {
    my $state = new_game(42);

    # 1. i32::MAX acres_delta to buy (i32::MAX * land_price must not wrap or truncate)
    my $huge_buy = {
        acres_delta     => 2147483647,
        bushels_to_feed => 0,
        acres_to_plant  => 0,
    };
    my $err_buy = dies { validate_order($state, $huge_buy) };
    ok($err_buy, 'huge buy order is refused');
    is($err_buy->kind(), 'InsufficientBushelsForLand', 'rejected cleanly by InsufficientBushelsForLand');

    # 2. u32::MAX acres_to_plant
    my $huge_plant = {
        acres_delta     => 0,
        bushels_to_feed => 0,
        acres_to_plant  => 4294967295,
    };
    my $err_plant = dies { validate_order($state, $huge_plant) };
    ok($err_plant, 'huge acres_to_plant is refused');
    is($err_plant->kind(), 'InsufficientAcresToPlant', 'rejected cleanly by InsufficientAcresToPlant');

    # 3. u32::MAX bushels_to_feed
    my $huge_feed = {
        acres_delta     => 0,
        bushels_to_feed => 4294967295,
        acres_to_plant  => 0,
    };
    my $err_feed = dies { validate_order($state, $huge_feed) };
    ok($err_feed, 'huge bushels_to_feed is refused');
    is($err_feed->kind(), 'InsufficientBushelsToFeed', 'rejected cleanly by InsufficientBushelsToFeed');

    # 4. Harvest potential overflow guard (900M acres * 5 bushels = 4.5B > u32::MAX)
    my $rich_state = {
        %$state,
        population => 500_000_000,
        acres      => 1_000_000_000,
        bushels    => 2_000_000_000,
    };
    my $overflow_plant = {
        acres_delta     => 0,
        bushels_to_feed => 0,
        acres_to_plant  => 900_000_000,
    };
    my $err_harvest = dies { validate_order($rich_state, $overflow_plant) };
    ok($err_harvest, 'harvest overflow potential is refused');
    is($err_harvest->kind(), 'InvalidOrderBounds', 'rejected by InvalidOrderBounds');

    # 5. Starvation cumulative totals saturate at u32::MAX without overflow
    my $sat_state = {
        %$state,
        starved_total       => 4294967295,
        percent_starved_sum => 4294967285, # u32::MAX - 10
    };
    my $order_starve = {
        acres_delta     => 0,
        bushels_to_feed => 1000, # starves 50 people
        acres_to_plant  => 0,
    };
    my $next_sat;
    ok(lives { $next_sat = step_year($sat_state, $order_starve) }, 'saturating state advance succeeds without panic');
    is($next_sat->{starved_total}, 4294967295, 'starved_total saturates at u32::MAX');
    is($next_sat->{percent_starved_sum}, 4294967295, 'percent_starved_sum saturates at u32::MAX');
};

# ============================================================================
# CWE-843: Type Confusion & Strict Integer Validation
# ============================================================================

subtest 'CWE-843: Rejection of non-integers, floats, garbage strings, undef, refs' => sub {
    my $state = new_game(1337);

    my @adversarial_inputs = (
        1.5,
        100.0,
        0.9999999999999999,
        "100 acres",
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

    for my $bad (@adversarial_inputs) {
        my $bad_desc = defined $bad ? (ref($bad) ? ref($bad) : "'$bad'") : 'undef';

        # In acres_delta
        my $order_delta = { acres_delta => $bad, bushels_to_feed => 2000, acres_to_plant => 100 };
        my $err1 = dies { validate_order($state, $order_delta) };
        ok($err1, "rejects adversarial acres_delta: $bad_desc");
        is($err1 ? $err1->kind() : '', 'InvalidOrderBounds', "error is InvalidOrderBounds");

        # In bushels_to_feed
        my $order_feed = { acres_delta => 0, bushels_to_feed => $bad, acres_to_plant => 100 };
        my $err2 = dies { validate_order($state, $order_feed) };
        ok($err2, "rejects adversarial bushels_to_feed: $bad_desc");
        is($err2 ? $err2->kind() : '', 'InvalidOrderBounds', "error is InvalidOrderBounds");

        # In acres_to_plant
        my $order_plant = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => $bad };
        my $err3 = dies { validate_order($state, $order_plant) };
        ok($err3, "rejects adversarial acres_to_plant: $bad_desc");
        is($err3 ? $err3->kind() : '', 'InvalidOrderBounds', "error is InvalidOrderBounds");
    }
};

# ============================================================================
# CWE-835 / CWE-400: Even rng_inc Rejection (PCG32 period collapse / infinite loop)
# ============================================================================

subtest 'CWE-835 / CWE-400: Reject even rng_inc to prevent infinite loop / period collapse' => sub {
    my $state = new_game(1);
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 100 };

    for my $even_inc (0, 2, 108, 4294967296) {
        my $bad_state = { %$state, rng_inc => $even_inc };
        my $err_val = dies { validate_order($bad_state, $order) };
        ok($err_val, "validate_order rejects even rng_inc = $even_inc");
        is($err_val->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');

        my $err_step = dies { step_year($bad_state, $order) };
        ok($err_step, "step_year rejects even rng_inc = $even_inc");
        is($err_step->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');
    }
};

# ============================================================================
# State Invariant Bounds (VER04m state_inv)
# ============================================================================

subtest 'State invariant bounds verification (VER04m state_inv)' => sub {
    my $state = new_game(1);
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 100 };

    # Hostile years
    for my $y (0, 11, 255, -1) {
        my $bad = { %$state, year => $y };
        my $err = dies { validate_order($bad, $order) };
        ok($err, "rejects hostile year = $y");
        is($err->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');
    }

    # Hostile land_price
    for my $p (0, 16, 27, 100, -5) {
        my $bad = { %$state, land_price => $p };
        my $err = dies { validate_order($bad, $order) };
        ok($err, "rejects hostile land_price = $p");
        is($err->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');
    }

    # Hostile harvest_per_acre
    for my $h (0, 6, 99, -1) {
        my $bad = { %$state, harvest_per_acre => $h };
        my $err = dies { validate_order($bad, $order) };
        ok($err, "rejects hostile harvest_per_acre = $h");
        is($err->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');
    }
};

# ============================================================================
# Immutability Guarantee (VER04m: input states are never mutated)
# ============================================================================

subtest 'VER04m: Input state and order records are never mutated' => sub {
    my $state = new_game(1337);
    my $state_copy = { %$state };
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 800 };
    my $order_copy = { %$order };

    my $next = step_year($state, $order);

    is($state, $state_copy, 'state record unmodified after successful step_year');
    is($order, $order_copy, 'order record unmodified after successful step_year');
    isnt($next, $state, 'next state is a distinct new object');

    # Also test error path leaves state completely unmodified
    my $bad_order = { acres_delta => 0, bushels_to_feed => 9999, acres_to_plant => 0 };
    my $bad_order_copy = { %$bad_order };
    eval { step_year($state, $bad_order) };
    is($state, $state_copy, 'state record unmodified after failed step_year');
    is($bad_order, $bad_order_copy, 'bad order unmodified after failed step_year');
};

# ============================================================================
# CWE-1333: ReDoS Protection
# ============================================================================

subtest 'CWE-1333: Linear regex evaluation on large digit inputs' => sub {
    my $state = new_game(1337);
    # Long string of digits with trailing letter
    my $long_payload = ("9" x 10000) . "x";
    my $order = { acres_delta => $long_payload, bushels_to_feed => 2000, acres_to_plant => 100 };

    my $start = time();
    my $err = dies { validate_order($state, $order) };
    my $elapsed = time() - $start;

    ok($err, 'rejects long payload with trailing garbage');
    is($err->kind(), 'InvalidOrderBounds', 'error is InvalidOrderBounds');
    ok($elapsed < 2, 'rejects payload linearly in under 2 seconds (no ReDoS)');
};

# ============================================================================
# Capability Cage: Zero Ambient Capabilities (VER08a, CAP001)
# ============================================================================

subtest 'Capability Cage: Zero ambient capabilities manifest' => sub {
    my @cap_candidates = (
        "$FindBin::Bin/../required_capabilities.json",
        'required_capabilities.json',
        'code/packages/perl/hamurabi/required_capabilities.json',
    );
    my ($cap_path) = grep { -f $_ } @cap_candidates;
    ok($cap_path, "Found required_capabilities.json at $cap_path");

    my $cap_text = do {
        open my $fh, '<', $cap_path or die $!;
        local $/;
        <$fh>;
    };
    my $caps = decode_json($cap_text);
    is(ref($caps), 'HASH', 'Capabilities manifest is a hash object');
    is($caps->{version}, 1, 'Manifest version is 1');
    is($caps->{package}, 'perl/hamurabi', 'Package identifier is perl/hamurabi');
    is(ref($caps->{capabilities}), 'ARRAY', 'Capabilities field is an array');
    is(scalar(@{$caps->{capabilities}}), 0, 'Capabilities manifest is strictly empty [] (pure simulation core)');
    ok(length($caps->{justification} // '') >= 10, 'Justification has sufficient explanation');
};

# ============================================================================
# Regression: new_game seed validation (reject undef, non-u64, out of bounds)
# ============================================================================

subtest 'Regression: new_game seed validation' => sub {
    # undef or missing arguments
    my $err_undef = dies { new_game(undef) };
    ok($err_undef, 'new_game(undef) dies');
    is($err_undef ? $err_undef->kind() : '', 'InvalidOrderBounds', 'new_game(undef) raises InvalidOrderBounds');

    my $err_noarg = dies { new_game() };
    ok($err_noarg, 'new_game() dies');
    is($err_noarg ? $err_noarg->kind() : '', 'InvalidOrderBounds', 'new_game() raises InvalidOrderBounds');

    # Adversarial / invalid seeds
    my @bad_seeds = (
        -1,
        '-1',
        1.5,
        100.0,
        '1.5',
        '100.0',
        0.9999999999999999,
        'bad_seed',
        ' 42 ',
        "42\n",
        'NaN',
        'Infinity',
        '-Inf',
        [],
        {},
        sub { 42 },
        \1,
        '18446744073709551616', # 2^64 (overflow u64)
        Math::BigInt->new(-1),
        Math::BigInt->new('18446744073709551616'),
        Math::BigInt->bnan(),
        Math::BigInt->binf(),
    );

    for my $bad (@bad_seeds) {
        my $desc = defined $bad ? (ref($bad) ? ref($bad) : "'$bad'") : 'undef';
        my $err = dies { new_game($bad) };
        ok($err, "new_game rejects bad seed $desc");
        is($err ? $err->kind() : '', 'InvalidOrderBounds', "error for $desc is InvalidOrderBounds");
    }

    # Valid seeds must succeed
    my $g1 = eval { new_game(0) };
    ok($g1 && $g1->{status} eq 'Playing', 'new_game(0) succeeds');

    my $g2 = eval { new_game(42) };
    ok($g2 && $g2->{status} eq 'Playing', 'new_game(42) succeeds');

    my $g3 = eval { new_game('18446744073709551615') };
    ok($g3 && $g3->{status} eq 'Playing', 'new_game(2^64-1) succeeds');
};

# ============================================================================
# Regression: Missing required state keys rejected with InvalidOrderBounds
# ============================================================================

subtest 'Regression: Missing required state keys rejected with InvalidOrderBounds' => sub {
    my $state = new_game(1337);
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 100 };

    my @required_keys = qw(
        year
        population
        bushels
        acres
        land_price
        harvest_per_acre
        rats_eaten
        starved_last_year
        starved_total
        percent_starved_sum
        immigrants_last_year
        plague_struck
        status
        rng_state
        rng_inc
    );

    for my $k (@required_keys) {
        # 1. Missing key entirely
        my $missing_state = { %$state };
        delete $missing_state->{$k};

        my $err_v1 = dies { validate_order($missing_state, $order) };
        ok($err_v1, "validate_order dies on missing key: $k");
        is($err_v1 ? $err_v1->kind() : '', 'InvalidOrderBounds', "missing key $k raises InvalidOrderBounds");

        my $err_s1 = dies { step_year($missing_state, $order) };
        ok($err_s1, "step_year dies on missing key: $k");
        is($err_s1 ? $err_s1->kind() : '', 'InvalidOrderBounds', "missing key $k in step_year raises InvalidOrderBounds");

        # 2. Key set to undef
        my $undef_state = { %$state, $k => undef };

        my $err_v2 = dies { validate_order($undef_state, $order) };
        ok($err_v2, "validate_order dies on undef key: $k");
        is($err_v2 ? $err_v2->kind() : '', 'InvalidOrderBounds', "undef key $k raises InvalidOrderBounds");

        my $err_s2 = dies { step_year($undef_state, $order) };
        ok($err_s2, "step_year dies on undef key: $k");
        is($err_s2 ? $err_s2->kind() : '', 'InvalidOrderBounds', "undef key $k in step_year raises InvalidOrderBounds");
    }
};

# ============================================================================
# Regression: Invalid rng_state rejected without infinite loop / NaN
# ============================================================================

subtest 'Regression: Invalid rng_state rejected with InvalidOrderBounds' => sub {
    my $state = new_game(1337);
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 100 };

    my @bad_rng_states = (
        undef,
        -1,
        '-1',
        1.5,
        100.0,
        '1.5',
        '100.0',
        0.9999999999999999,
        'bad_rng_state',
        ' 12345 ',
        "12345\n",
        'NaN',
        'Infinity',
        '-Inf',
        [],
        {},
        sub { 42 },
        \1,
        '18446744073709551616', # 2^64 (overflow u64)
        Math::BigInt->new(-1),
        Math::BigInt->new('18446744073709551616'),
        Math::BigInt->bnan(),
        Math::BigInt->binf(),
    );

    for my $bad_rng (@bad_rng_states) {
        my $desc = defined $bad_rng ? (ref($bad_rng) ? ref($bad_rng) : "'$bad_rng'") : 'undef';
        my $bad_state = { %$state, rng_state => $bad_rng };

        my $err_val = dies { validate_order($bad_state, $order) };
        ok($err_val, "validate_order rejects bad rng_state: $desc");
        is($err_val ? $err_val->kind() : '', 'InvalidOrderBounds', "error is InvalidOrderBounds for $desc");

        my $err_step = dies { step_year($bad_state, $order) };
        ok($err_step, "step_year rejects bad rng_state: $desc");
        is($err_step ? $err_step->kind() : '', 'InvalidOrderBounds', "step_year error is InvalidOrderBounds for $desc");
    }
};

# ============================================================================
# Regression: Validation of status and plague_struck fields
# ============================================================================

subtest 'Regression: Validation of status and plague_struck fields' => sub {
    my $state = new_game(1337);
    my $order = { acres_delta => 0, bushels_to_feed => 2000, acres_to_plant => 100 };

    # Invalid status strings raise InvalidOrderBounds
    for my $bad_status ('InvalidStatus', '', 'PlayingExtra', 'GameOver', 'Won', 'Impeached', [], {}) {
        my $desc = ref($bad_status) ? ref($bad_status) : "'$bad_status'";
        my $bad_state = { %$state, status => $bad_status };
        my $err = dies { validate_order($bad_state, $order) };
        ok($err, "validate_order rejects bad status $desc");
        is($err ? $err->kind() : '', 'InvalidOrderBounds', "error for status $desc is InvalidOrderBounds");
    }

    # Valid ended statuses raise GameAlreadyEnded
    for my $ended_status ('GameOverExtinction', 'Won(Charlemagne)', 'Won(Acre)', 'Won(Nero)', 'Won(Disaster)', 'Impeached(starved=50, percent=50)') {
        my $ended_state = { %$state, status => $ended_status };
        my $err = dies { validate_order($ended_state, $order) };
        ok($err, "validate_order rejects ended status '$ended_status'");
        is($err ? $err->kind() : '', 'GameAlreadyEnded', "error for '$ended_status' is GameAlreadyEnded");
    }

    # Invalid plague_struck values raise InvalidOrderBounds
    for my $bad_plague (2, -1, 100, 1.5, 'true', 'false', 'yes', [], {}) {
        my $desc = ref($bad_plague) ? ref($bad_plague) : "'$bad_plague'";
        my $bad_state = { %$state, plague_struck => $bad_plague };
        my $err = dies { validate_order($bad_state, $order) };
        ok($err, "validate_order rejects bad plague_struck $desc");
        is($err ? $err->kind() : '', 'InvalidOrderBounds', "error for plague_struck $desc is InvalidOrderBounds");
    }

    # Valid plague_struck values succeed
    for my $good_plague (0, 1, JSON::PP::true, JSON::PP::false, \0, \1) {
        my $good_state = { %$state, plague_struck => $good_plague };
        my $valid = eval { validate_order($good_state, $order) };
        ok($valid, "validate_order accepts valid plague_struck");
    }
};

done_testing;
