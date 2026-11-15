use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Hamurabi qw(
    new_game
    validate_order
    check_order
    step_year
    format_report
);

subtest 'Initial state verification' => sub {
    my $state = new_game(1337);
    is($state->{year}, 1, 'year is 1');
    is($state->{population}, 100, 'initial population is 100');
    is($state->{bushels}, 2800, 'initial bushels is 2800');
    is($state->{acres}, 1000, 'initial acres is 1000');
    ok($state->{land_price} >= 17 && $state->{land_price} <= 26, 'land_price in [17, 26]');
    is($state->{harvest_per_acre}, 3, 'initial harvest_per_acre is 3');
    is($state->{rats_eaten}, 200, 'initial rats_eaten is 200');
    is($state->{starved_last_year}, 0, 'starved_last_year is 0');
    is($state->{starved_total}, 0, 'starved_total is 0');
    is($state->{percent_starved_sum}, 0, 'percent_starved_sum is 0');
    is($state->{immigrants_last_year}, 5, 'immigrants_last_year is 5');
    ok(!$state->{plague_struck}, 'plague_struck is false');
    is($state->{status}, 'Playing', 'status is Playing');
    is($state->{rng_inc} & 1, 1, 'rng_inc is odd');
};

subtest 'All eight error arms in canonical order' => sub {
    my $state = new_game(1337);

    my $normal_order = {
        acres_delta     => 0,
        bushels_to_feed => 2000,
        acres_to_plant  => 100,
    };

    # Arm 1: GameAlreadyEnded
    {
        my $ended_state = { %$state, status => 'Won(Charlemagne)' };
        my $err1 = dies { validate_order($ended_state, $normal_order) };
        ok($err1, 'validate_order dies on ended game');
        is($err1->kind(), 'GameAlreadyEnded', 'Arm 1: GameAlreadyEnded');

        my $err2 = dies { step_year($ended_state, $normal_order) };
        ok($err2, 'step_year dies on ended game');
        is($err2->kind(), 'GameAlreadyEnded', 'Arm 1: step_year GameAlreadyEnded');
    }

    # Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
    {
        my $bad_state = { %$state, land_price => 10 }; # valid is 17..=26
        my $err = dies { validate_order($bad_state, $normal_order) };
        ok($err, 'validate_order dies on invalid land price');
        is($err->kind(), 'InvalidOrderBounds', 'Arm 2: InvalidOrderBounds (corrupted land_price)');

        my $rich_state = { %$state, acres => 2_000_000_000, bushels => 4_000_000_000 };
        my $overflow_sell = {
            acres_delta     => -1_000_000_000,
            bushels_to_feed => 0,
            acres_to_plant  => 0,
        };
        my $err_rev = dies { validate_order($rich_state, $overflow_sell) };
        ok($err_rev, 'validate_order dies on revenue overflow');
        is($err_rev->kind(), 'InvalidOrderBounds', 'Arm 2: InvalidOrderBounds (revenue overflow)');
    }

    # Arm 3: InsufficientBushelsForLand
    {
        my $buy_too_much = {
            acres_delta     => 200, # 200 * >=17 = 3400 > 2800
            bushels_to_feed => 0,
            acres_to_plant  => 0,
        };
        my $err = dies { validate_order($state, $buy_too_much) };
        ok($err, 'validate_order dies on insufficient bushels for land');
        is($err->kind(), 'InsufficientBushelsForLand', 'Arm 3: InsufficientBushelsForLand');
    }

    # Arm 4: InsufficientAcresToSell
    {
        my $sell_too_much = {
            acres_delta     => -1001, # owns 1000 acres
            bushels_to_feed => 0,
            acres_to_plant  => 0,
        };
        my $err = dies { validate_order($state, $sell_too_much) };
        ok($err, 'validate_order dies on selling more acres than owned');
        is($err->kind(), 'InsufficientAcresToSell', 'Arm 4: InsufficientAcresToSell');
    }

    # Arm 5: InsufficientBushelsToFeed
    {
        my $feed_too_much = {
            acres_delta     => 0,
            bushels_to_feed => 2801, # only 2800 available
            acres_to_plant  => 0,
        };
        my $err = dies { validate_order($state, $feed_too_much) };
        ok($err, 'validate_order dies on feeding more than available');
        is($err->kind(), 'InsufficientBushelsToFeed', 'Arm 5: InsufficientBushelsToFeed');
    }

    # Arm 6: InsufficientAcresToPlant
    {
        my $plant_too_much_acres = {
            acres_delta     => 0,
            bushels_to_feed => 1000,
            acres_to_plant  => 1001, # only 1000 acres owned
        };
        my $err = dies { validate_order($state, $plant_too_much_acres) };
        ok($err, 'validate_order dies on planting more acres than owned');
        is($err->kind(), 'InsufficientAcresToPlant', 'Arm 6: InsufficientAcresToPlant');
    }

    # Arm 7: InsufficientPopulationToPlant
    {
        my $small_pop_state = { %$state, population => 50 }; # tends at most 500 acres
        my $plant_too_much_pop = {
            acres_delta     => 0,
            bushels_to_feed => 1000,
            acres_to_plant  => 501,
        };
        my $err = dies { validate_order($small_pop_state, $plant_too_much_pop) };
        ok($err, 'validate_order dies on planting more acres than labor capacity');
        is($err->kind(), 'InsufficientPopulationToPlant', 'Arm 7: InsufficientPopulationToPlant');
    }

    # Arm 8: InsufficientGrainForSeed
    {
        # 800 acres needs 400 bushels seed. Bushels left after feed: 2800 - 2500 = 300.
        my $seed_starved = {
            acres_delta     => 0,
            bushels_to_feed => 2500,
            acres_to_plant  => 800,
        };
        my $err = dies { validate_order($state, $seed_starved) };
        ok($err, 'validate_order dies on insufficient grain for seed');
        is($err->kind(), 'InsufficientGrainForSeed', 'Arm 8: InsufficientGrainForSeed');
    }
};

subtest 'check_order non-throwing interface' => sub {
    my $state = new_game(1337);
    my $good_order = {
        acres_delta     => 0,
        bushels_to_feed => 2000,
        acres_to_plant  => 100,
    };
    my ($ok1, $err1) = check_order($state, $good_order);
    ok($ok1, 'valid order returns truthy ok');
    ok(!defined $err1, 'valid order has undef err');

    my $bad_order = {
        acres_delta     => 0,
        bushels_to_feed => 9999,
        acres_to_plant  => 0,
    };
    my ($ok2, $err2) = check_order($state, $bad_order);
    ok(!$ok2, 'invalid order returns falsey ok');
    is($err2->kind(), 'InsufficientBushelsToFeed', 'invalid order returns HamurabiError');
};

subtest 'Teletype report formatting' => sub {
    my $state = new_game(1337);
    my $report = format_report($state);
    like($report, qr/HAMURABI: I BEG TO REPORT TO YOU/, 'header present');
    like($report, qr/IN YEAR 1, 0 PEOPLE STARVED, 5 CAME TO THE CITY/, 'year 1 summary');
    like($report, qr/POPULATION IS NOW 100/, 'population');
    like($report, qr/THE CITY NOW OWNS 1000 ACRES/, 'acres');
    like($report, qr/YOU HARVESTED 3 BUSHELS PER ACRE/, 'harvest');
    like($report, qr/RATS ATE 200 BUSHELS/, 'rats');
    like($report, qr/YOU NOW HAVE 2800 BUSHELS IN STORE/, 'bushels');

    # Plague report
    my $plague_state = { %$state, plague_struck => 1 };
    my $plague_rep = format_report($plague_state);
    like($plague_rep, qr/A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED/, 'plague notice');

    # Impeachment report
    my $impeach_state = { %$state, status => 'Impeached(starved=48, percent=48)' };
    my $impeach_rep = format_report($impeach_state);
    like($impeach_rep, qr/YOU STARVED 48 PEOPLE IN ONE YEAR!!!/, 'impeach count');
    like($impeach_rep, qr/DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED/, 'impeach message');

    # Extinction report
    my $extinct_state = { %$state, status => 'GameOverExtinction' };
    is(format_report($extinct_state), "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", 'extinction report');

    # Won ratings reports
    my @ratings = (
        ['Charlemagne', qr/A FANTASTIC PERFORMANCE!/],
        ['Acre',        qr/YOUR PERFORMANCE COULD HAVE BEEN WORSE/],
        ['Nero',        qr/NERO WOULD HAVE BEEN PROUD OF YOU/],
        ['Disaster',    qr/OFFICIAL NATIONAL DISASTER/],
    );
    for my $r (@ratings) {
        my ($rating, $expected_pattern) = @$r;
        my $won_state = {
            %$state,
            year                => 11,
            percent_starved_sum => 20,
            starved_total       => 5,
            acres               => 1000,
            population          => 100,
            status              => "Won($rating)",
        };
        my $won_rep = format_report($won_state);
        like($won_rep, qr/IN YOUR 10-YEAR TERM OF OFFICE/, "$rating summary header");
        like($won_rep, $expected_pattern, "$rating message");
    }
};

subtest 'Deterministic step year advance' => sub {
    my $state = new_game(1337);
    my $order = {
        acres_delta     => 0,
        bushels_to_feed => 2000,
        acres_to_plant  => 800,
    };
    my $next = step_year($state, $order);
    is($next->{year}, 2, 'advances to year 2');
    is($next->{acres}, 1000, 'acres unchanged');
    is($next->{starved_last_year}, 0, 'zero starvation');
    is($next->{status}, 'Playing', 'still playing');
};

subtest 'Immediate impeachment on 100% starvation' => sub {
    my $state = new_game(100);
    my $starve_order = {
        acres_delta     => 0,
        bushels_to_feed => 0,
        acres_to_plant  => 0,
    };
    my $next = step_year($state, $starve_order);
    is($next->{status}, 'Impeached(starved=100, percent=100)', 'status is Impeached');
    my $rep = format_report($next);
    like($rep, qr/YOU STARVED 100 PEOPLE IN ONE YEAR!!!/, 'report states 100 starved');
    like($rep, qr/IMPEACHED AND THROWN OUT OF OFFICE/, 'report states impeachment');
};

done_testing;
