package CodingAdventures::Hamurabi;

use strict;
use warnings;
use Math::BigInt;
use B;
use Exporter 'import';

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(new_game validate_order check_order step_year format_report);

# ============================================================================
# CodingAdventures::Hamurabi::HamurabiError
# Structured closed refusal enum
# ============================================================================

package CodingAdventures::Hamurabi::HamurabiError;

use strict;
use warnings;
use overload
    '""'     => sub { $_[0]->{kind} },
    fallback => 1;

sub new {
    my ($class, $kind, $message) = @_;
    return bless {
        kind    => $kind,
        message => $message // $kind,
    }, $class;
}

sub kind    { $_[0]->{kind} }
sub message { $_[0]->{message} }

# ============================================================================
# Simulation Core
# ============================================================================

package CodingAdventures::Hamurabi;

# PRNG constants
use constant HAMURABI_STREAM_ID => 54;
use constant DEFAULT_INC        => 109; # (54 << 1) | 1
use constant U32_MAX            => 4294967295;
use constant I32_MIN            => -2147483648;
use constant I32_MAX            => 2147483647;

my $_PCG_MULTIPLIER = Math::BigInt->new('6364136223846793005');
my $_MASK64         = Math::BigInt->new('18446744073709551615');

sub _err {
    my ($kind, $msg) = @_;
    die CodingAdventures::Hamurabi::HamurabiError->new($kind, $msg);
}

# Strict integer validation:
# - Rejects undef, non-Math::BigInt refs
# - Rejects floats (NV without IV flag)
# - Rejects strings with trailing garbage via zero-backtracking anchored regex \A-?[0-9]+\z
# - Rejects floats masquerading as ints through int($val) == $val
sub _is_strict_int {
    my ($val) = @_;
    return 0 unless defined $val;
    if (ref($val)) {
        return 0 unless eval { $val->isa('Math::BigInt') };
        return 0 if $val->is_nan() || $val->is_inf();
        return 1;
    }
    my $flags = B::svref_2object(\$val)->FLAGS;
    return 0 if ($flags & B::SVf_NOK) && !($flags & B::SVf_IOK);
    my $str = "$val";
    return 0 unless $str =~ /\A-?[0-9]+\z/;
    return 0 unless $val == int($val);
    return 1;
}

sub _to_bigint {
    my ($val) = @_;
    return $val->copy() if ref($val) && eval { $val->isa('Math::BigInt') };
    return Math::BigInt->new("$val");
}

sub _is_valid_uint64 {
    my ($val) = @_;
    return 0 unless defined $val;
    if (ref($val)) {
        return 0 unless eval { $val->isa('Math::BigInt') };
        return 0 if $val->is_nan() || $val->is_inf();
        return 0 if $val < 0 || $val > $_MASK64;
        return 1;
    }
    my $flags = B::svref_2object(\$val)->FLAGS;
    return 0 if ($flags & B::SVf_NOK) && !($flags & B::SVf_IOK);
    my $str = "$val";
    return 0 unless $str =~ /\A[0-9]+\z/;
    my $b = eval { Math::BigInt->new($str) };
    return 0 if !defined $b || $b->is_nan() || $b->is_inf();
    return 0 if $b < 0 || $b > $_MASK64;
    return 1;
}

sub _is_valid_bool {
    my ($val) = @_;
    return 0 unless defined $val;
    if (ref($val)) {
        return 1 if eval { $val->isa('JSON::PP::Boolean') };
        return 1 if ref($val) eq 'SCALAR' && ($$val eq '0' || $$val eq '1');
        return 0;
    }
    return 1 if _is_strict_int($val) && ($val == 0 || $val == 1);
    return 0;
}

sub _is_valid_status {
    my ($status) = @_;
    return 0 unless defined $status && !ref($status);
    return 1 if $status eq 'Playing';
    return 1 if $status eq 'GameOverExtinction';
    return 1 if $status =~ /\AImpeached\(starved=\d+,\s*percent=\d+\)\z/;
    return 1 if $status =~ /\AWon\((Charlemagne|Acre|Nero|Disaster)\)\z/;
    return 0;
}

# ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

sub _pcg32_step {
    my ($state_ref, $inc) = @_;
    my $old = $$state_ref->copy();
    $$state_ref = ($old * $_PCG_MULTIPLIER + $inc)->band($_MASK64);

    my $xorshifted_full = $old->copy()->brsft(18)->bxor($old)->brsft(27);
    my $xorshifted = ($xorshifted_full->band(0xFFFFFFFF))->numify();
    my $rot = ($old->copy()->brsft(59))->numify();

    return (($xorshifted >> $rot) | (($xorshifted << ((32 - $rot) & 31)) & 0xFFFFFFFF)) & 0xFFFFFFFF;
}

sub _pcg32_next_int_in_range {
    my ($state_ref, $inc, $min, $max) = @_;
    my $span = $max - $min + 1;
    my $threshold = (4294967296 - $span) % $span;
    while (1) {
        my $r = _pcg32_step($state_ref, $inc);
        if ($r >= $threshold) {
            return $min + ($r % $span);
        }
    }
}

# ============================================================================
# Public API
# ============================================================================

sub new_game {
    my ($seed) = @_;
    _err('InvalidOrderBounds', 'Seed cannot be undef') unless defined $seed;
    _err('InvalidOrderBounds', 'Invalid seed') unless _is_valid_uint64($seed);
    my $seed_b = _to_bigint($seed);

    my $inc = Math::BigInt->new(DEFAULT_INC);
    my $rng_state = Math::BigInt->new(0);

    # Standard PCG initseq warm-up
    $rng_state = ($rng_state * $_PCG_MULTIPLIER + $inc)->band($_MASK64);
    $rng_state = ($rng_state + $seed_b)->band($_MASK64);
    $rng_state = ($rng_state * $_PCG_MULTIPLIER + $inc)->band($_MASK64);

    # Initial land price draw for Year 1 (range 17..=26)
    my $land_price = _pcg32_next_int_in_range(\$rng_state, $inc, 17, 26);

    return {
        year                => 1,
        population          => 100,
        bushels             => 2800,
        acres               => 1000,
        land_price          => $land_price,
        harvest_per_acre    => 3,
        rats_eaten          => 200,
        starved_last_year   => 0,
        starved_total       => 0,
        percent_starved_sum => 0,
        immigrants_last_year=> 5,
        plague_struck       => 0,
        status              => 'Playing',
        rng_state           => $rng_state->bstr(),
        rng_inc             => $inc->numify(),
    };
}

sub _validate_order_impl {
    my ($state, $order) = @_;
    return 'InvalidOrderBounds' unless defined $state && ref($state) eq 'HASH';
    return 'InvalidOrderBounds' unless defined $order && ref($order) eq 'HASH';

    # 1. Status Check
    return 'InvalidOrderBounds' unless exists $state->{status} && _is_valid_status($state->{status});
    return 'GameAlreadyEnded' unless $state->{status} eq 'Playing';

    # State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
    # - Enforce odd rng_inc to prevent infinite loop / period collapse in PCG32
    return 'InvalidOrderBounds' unless exists $state->{rng_inc} && _is_valid_uint64($state->{rng_inc});
    my $inc_b = _to_bigint($state->{rng_inc});
    return 'InvalidOrderBounds' if ($inc_b->copy()->band(1))->is_zero();

    # - RNG state validation: must exist and be valid u64 in [0, 2^64-1]
    return 'InvalidOrderBounds' unless exists $state->{rng_state} && _is_valid_uint64($state->{rng_state});

    # - Plague struck boolean validation
    return 'InvalidOrderBounds' unless exists $state->{plague_struck} && _is_valid_bool($state->{plague_struck});

    # - Reign year in [1, 10]
    return 'InvalidOrderBounds' unless exists $state->{year};
    my $year = $state->{year};
    return 'InvalidOrderBounds' unless defined $year && _is_strict_int($year) && $year >= 1 && $year <= 10;

    # - Land price in [17, 26]
    return 'InvalidOrderBounds' unless exists $state->{land_price};
    my $land_price = $state->{land_price};
    return 'InvalidOrderBounds' unless defined $land_price && _is_strict_int($land_price) && $land_price >= 17 && $land_price <= 26;

    # - Harvest yield in [1, 5]
    return 'InvalidOrderBounds' unless exists $state->{harvest_per_acre};
    my $harvest_per_acre = $state->{harvest_per_acre};
    return 'InvalidOrderBounds' unless defined $harvest_per_acre && _is_strict_int($harvest_per_acre) && $harvest_per_acre >= 1 && $harvest_per_acre <= 5;

    # - State field bounds (all 8 remaining u32 fields required)
    for my $f (qw(population bushels acres starved_last_year starved_total percent_starved_sum immigrants_last_year rats_eaten)) {
        return 'InvalidOrderBounds' unless exists $state->{$f};
        my $v = $state->{$f};
        return 'InvalidOrderBounds' unless defined $v && _is_strict_int($v);
        my $b = _to_bigint($v);
        return 'InvalidOrderBounds' if $b < 0 || $b > U32_MAX;
    }

    # Order bounds
    return 'InvalidOrderBounds' unless exists $order->{acres_delta};
    my $acres_delta = $order->{acres_delta};
    return 'InvalidOrderBounds' unless defined $acres_delta && _is_strict_int($acres_delta);
    my $delta_b = _to_bigint($acres_delta);
    return 'InvalidOrderBounds' if $delta_b < I32_MIN || $delta_b > I32_MAX;

    return 'InvalidOrderBounds' unless exists $order->{bushels_to_feed};
    my $bushels_to_feed = $order->{bushels_to_feed};
    return 'InvalidOrderBounds' unless defined $bushels_to_feed && _is_strict_int($bushels_to_feed);
    my $feed_b = _to_bigint($bushels_to_feed);
    return 'InvalidOrderBounds' if $feed_b < 0 || $feed_b > U32_MAX;

    return 'InvalidOrderBounds' unless exists $order->{acres_to_plant};
    my $acres_to_plant = $order->{acres_to_plant};
    return 'InvalidOrderBounds' unless defined $acres_to_plant && _is_strict_int($acres_to_plant);
    my $plant_b = _to_bigint($acres_to_plant);
    return 'InvalidOrderBounds' if $plant_b < 0 || $plant_b > U32_MAX;

    my $acres_b   = _to_bigint($state->{acres});
    my $bushels_b = _to_bigint($state->{bushels});
    my $price_b   = _to_bigint($land_price);
    my $pop_b     = _to_bigint($state->{population});

    # 2. Bounds & Overflow Guard on Land Trade
    if ($delta_b > 0) {
        my $cost = $delta_b->copy()->bmul($price_b);
        return 'InsufficientBushelsForLand' if $cost > $bushels_b;
    } elsif ($delta_b < 0) {
        my $sell_count = $delta_b->copy()->babs();
        return 'InsufficientAcresToSell' if $sell_count > $acres_b;
    }

    # 3. Land Trade Balances
    my $acres_after_trade;
    my $bushels_after_trade;

    if ($delta_b >= 0) {
        $acres_after_trade = $acres_b->copy()->badd($delta_b);
        return 'InvalidOrderBounds' if $acres_after_trade > U32_MAX;
    } else {
        my $sell_count = $delta_b->copy()->babs();
        $acres_after_trade = $acres_b->copy()->bsub($sell_count);
    }

    if ($delta_b > 0) {
        my $cost = $delta_b->copy()->bmul($price_b);
        $bushels_after_trade = $bushels_b->copy()->bsub($cost);
    } elsif ($delta_b < 0) {
        my $sell_count = $delta_b->copy()->babs();
        my $revenue = $sell_count->copy()->bmul($price_b);
        $bushels_after_trade = $bushels_b->copy()->badd($revenue);
        return 'InvalidOrderBounds' if $bushels_after_trade > U32_MAX;
    } else {
        $bushels_after_trade = $bushels_b->copy();
    }

    # 4. Food Allocation Check
    return 'InsufficientBushelsToFeed' if $feed_b > $bushels_after_trade;
    my $bushels_after_feed = $bushels_after_trade->copy()->bsub($feed_b);

    # 5. Planting Land & Labor Checks
    return 'InsufficientAcresToPlant' if $plant_b > $acres_after_trade;
    my $max_labor_acres = $pop_b->copy()->bmul(10);
    return 'InsufficientPopulationToPlant' if $plant_b > $max_labor_acres;

    # 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
    my $seed_needed = ($plant_b->copy()->badd(1))->bdiv(2);
    return 'InsufficientGrainForSeed' if $seed_needed > $bushels_after_feed;
    my $bushels_after_seed = $bushels_after_feed->copy()->bsub($seed_needed);

    # 7. Maximum Harvest Overflow Guard (CWE-190)
    my $max_harvest = $plant_b->copy()->bmul(5);
    my $max_stored = $bushels_after_seed->copy()->badd($max_harvest);
    return 'InvalidOrderBounds' if $max_stored > U32_MAX;

    return undef;
}

sub validate_order {
    my ($state, $order) = @_;
    my $err = _validate_order_impl($state, $order);
    if (defined $err) {
        _err($err);
    }
    return 1;
}

sub check_order {
    my ($state, $order) = @_;
    my $err = _validate_order_impl($state, $order);
    if (defined $err) {
        return (0, CodingAdventures::Hamurabi::HamurabiError->new($err));
    }
    return (1, undef);
}

sub step_year {
    my ($state, $order) = @_;
    validate_order($state, $order);

    my $pcg_state = _to_bigint($state->{rng_state});
    my $pcg_inc   = _to_bigint($state->{rng_inc});

    my $acres_b   = _to_bigint($state->{acres});
    my $bushels_b = _to_bigint($state->{bushels});
    my $price_b   = _to_bigint($state->{land_price});
    my $pop_b     = _to_bigint($state->{population});
    my $delta_b   = _to_bigint($order->{acres_delta});
    my $feed_b    = _to_bigint($order->{bushels_to_feed});
    my $plant_b   = _to_bigint($order->{acres_to_plant});

    # Intermediate balances from validated trade
    my $acres_after_trade;
    my $bushels_after_trade;

    if ($delta_b >= 0) {
        $acres_after_trade = ($acres_b + $delta_b)->numify();
    } else {
        $acres_after_trade = ($acres_b - $delta_b->copy()->babs())->numify();
    }

    if ($delta_b > 0) {
        my $cost = $delta_b * $price_b;
        $bushels_after_trade = ($bushels_b - $cost)->numify();
    } elsif ($delta_b < 0) {
        my $revenue = $delta_b->copy()->babs() * $price_b;
        $bushels_after_trade = ($bushels_b + $revenue)->numify();
    } else {
        $bushels_after_trade = $bushels_b->numify();
    }

    my $bushels_after_feed = $bushels_after_trade - $feed_b->numify();
    my $seed_needed = int(($plant_b->numify() + 1) / 2);
    my $bushels_after_seed = $bushels_after_feed - $seed_needed;

    # Step 1: Harvest Yield Draw
    # Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (u32)
    my $harvest_per_acre = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 1, 5);
    my $total_harvest = $plant_b->numify() * $harvest_per_acre;
    my $bushels_stored = $bushels_after_seed + $total_harvest;

    # Step 2: Rat Infestation Draws
    # Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (u32)
    my $rat_roll = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 1, 100);
    my $rats_eaten = 0;
    if ($rat_roll <= 40) {
        # Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (u32)
        my $rat_factor = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 1, 5);
        $rats_eaten = int($bushels_stored / $rat_factor);
    }
    my $bushels_final = $bushels_stored - $rats_eaten;

    # Step 3: Plague Draw
    # Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (u32)
    my $plague_roll = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 1, 100);
    my $plague_struck = ($plague_roll <= 15) ? 1 : 0;

    # Step 4: Starvation & Impeachment Evaluation
    my $people_fed = int($feed_b->numify() / 20);
    my ($starved, $survivors);
    if ($people_fed >= $pop_b->numify()) {
        $starved = 0;
        $survivors = $pop_b->numify();
    } else {
        $starved = $pop_b->numify() - $people_fed;
        $survivors = $people_fed;
    }

    my $percent_starved = ($pop_b->numify() > 0) ? int(($starved * 100) / $pop_b->numify()) : 0;
    $percent_starved = 100 if $percent_starved > 100;

    my $new_starved_total = ($state->{starved_total} // 0) + $starved;
    $new_starved_total = U32_MAX if $new_starved_total > U32_MAX;

    my $new_percent_sum = ($state->{percent_starved_sum} // 0) + $percent_starved;
    $new_percent_sum = U32_MAX if $new_percent_sum > U32_MAX;

    my $new_status;
    if ($starved * 100 > $pop_b->numify() * 45) {
        $new_status = "Impeached(starved=$starved, percent=$percent_starved)";
    } elsif ($survivors == 0) {
        $new_status = 'GameOverExtinction';
    } else {
        $new_status = 'Playing';
    }

    # Step 5: Plague Resolution
    if ($plague_struck && $new_status eq 'Playing') {
        $survivors = int($survivors / 2);
    }

    # Step 6: Immigration Draw
    # Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (u32)
    my $imm_factor = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 1, 5);
    my $immigrants = 0;
    if ($starved == 0 && $new_status eq 'Playing') {
        my $acres_term = Math::BigInt->new($acres_after_trade)->bmul(20);
        my $inner = $acres_term->badd($bushels_final);
        my $numerator = Math::BigInt->new($imm_factor)->bmul($inner);
        my $denominator = Math::BigInt->new(100)->bmul($pop_b);
        if (!$denominator->is_zero()) {
            my $q = $numerator->bdiv($denominator);
            $immigrants = ($q->badd(1))->numify();
            $immigrants = U32_MAX if $immigrants > U32_MAX;
        }
    }

    my $final_population = $survivors + $immigrants;
    $final_population = U32_MAX if $final_population > U32_MAX;

    # Step 7: Next Year Land Price Draw
    # Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (u32)
    my $next_land_price = _pcg32_next_int_in_range(\$pcg_state, $pcg_inc, 17, 26);

    # Step 8: Turn Advance & Rating
    my $next_year = $state->{year} + 1;
    if ($next_year == 11 && $new_status eq 'Playing') {
        my $avg_starved = int($new_percent_sum / 10);
        my $acres_per_person = ($final_population > 0) ? int($acres_after_trade / $final_population) : 0;
        my $rating;
        if ($avg_starved <= 3 && $acres_per_person >= 10) {
            $rating = 'Charlemagne';
        } elsif ($avg_starved <= 10 && $acres_per_person >= 9) {
            $rating = 'Acre';
        } elsif ($avg_starved <= 33 && $acres_per_person >= 7) {
            $rating = 'Nero';
        } else {
            $rating = 'Disaster';
        }
        $new_status = "Won($rating)";
    }

    return {
        year                => $next_year,
        population          => $final_population,
        bushels             => $bushels_final,
        acres               => $acres_after_trade,
        land_price          => $next_land_price,
        harvest_per_acre    => $harvest_per_acre,
        rats_eaten          => $rats_eaten,
        starved_last_year   => $starved,
        starved_total       => $new_starved_total,
        percent_starved_sum => $new_percent_sum,
        immigrants_last_year=> $immigrants,
        plague_struck       => $plague_struck ? 1 : 0,
        status              => $new_status,
        rng_state           => $pcg_state->bstr(),
        rng_inc             => $pcg_inc->bstr() =~ /\A[0-9]+\z/ && $pcg_inc <= U32_MAX ? $pcg_inc->numify() : $pcg_inc->bstr(),
    };
}

sub format_report {
    my ($state) = @_;
    my $status = $state->{status} // '';

    if ($status =~ /\AImpeached\(starved=(\d+),\s*percent=(\d+)\)\z/) {
        my ($starved_this_turn, $percent) = ($1, $2);
        return "YOU STARVED $starved_this_turn PEOPLE IN ONE YEAR!!!\n"
             . "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n";
    }

    if ($status eq 'GameOverExtinction') {
        return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n";
    }

    if ($status =~ /\AWon\((\w+)\)\z/) {
        my $rating = $1;
        my $avg_starved = int(($state->{percent_starved_sum} // 0) / 10);
        my $pop = $state->{population} // 0;
        my $acres_per_person = ($pop > 0) ? int(($state->{acres} // 0) / $pop) : 0;
        my $starved_total = $state->{starved_total} // 0;

        my $rating_msg = '';
        if ($rating eq 'Charlemagne') {
            $rating_msg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!";
        } elsif ($rating eq 'Acre') {
            $rating_msg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.";
        } elsif ($rating eq 'Nero') {
            $rating_msg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.";
        } else {
            $rating_msg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!";
        }

        return "IN YOUR 10-YEAR TERM OF OFFICE, $avg_starved PERCENT OF THE\n"
             . "POPULATION STARVED PER YEAR ON AVERAGE.\n"
             . "A TOTAL OF $starved_total PEOPLE DIED!!\n"
             . "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n"
             . "$acres_per_person ACRES PER PERSON.\n\n"
             . "$rating_msg\n";
    }

    # Default: Playing
    my $out = "HAMURABI: I BEG TO REPORT TO YOU,\n"
            . "IN YEAR $state->{year}, $state->{starved_last_year} PEOPLE STARVED, $state->{immigrants_last_year} CAME TO THE CITY.\n";
    if ($state->{plague_struck}) {
        $out .= "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n";
    }
    $out .= "POPULATION IS NOW $state->{population}.\n"
         . "THE CITY NOW OWNS $state->{acres} ACRES.\n"
         . "YOU HARVESTED $state->{harvest_per_acre} BUSHELS PER ACRE.\n"
         . "RATS ATE $state->{rats_eaten} BUSHELS.\n"
         . "YOU NOW HAVE $state->{bushels} BUSHELS IN STORE.\n"
         . "LAND IS TRADING AT $state->{land_price} BUSHELS PER ACRE.\n";

    return $out;
}

1;

__END__

=pod

=head1 NAME

CodingAdventures::Hamurabi - Mainframe Turn-Based Resource Allocation Simulation Core

=head1 SYNOPSIS

    use CodingAdventures::Hamurabi qw(new_game validate_order step_year format_report);

    my $state = new_game(1337);
    my $order = {
        acres_delta     => 0,
        bushels_to_feed => 2000,
        acres_to_plant  => 800,
    };

    validate_order($state, $order);
    my $next_state = step_year($state, $order);
    print format_report($next_state);

=head1 DESCRIPTION

Pure, deterministic simulation core for the seminal 1968 mainframe resource
allocation game in pure Perl. Strictly zero ambient system authority with
C<capabilities: []>.

=cut
