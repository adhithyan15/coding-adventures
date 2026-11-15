package CodingAdventures::LunarLander;

use strict;
use warnings;
use utf8;
use Math::BigInt;
use B ();
use Exporter 'import';

our $VERSION   = '0.1.0';
our @EXPORT_OK = qw(
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

# ============================================================================
# Physical Constants & Apollo LM Parameters
# ============================================================================

use constant G_LUNAR_MFT_S2         => 5310;
use constant DRY_MASS_LBS           => 16500;
use constant INITIAL_FUEL_LBS       => 16500;
use constant TOTAL_INITIAL_MASS_LBS => 33000;
use constant INITIAL_ALTITUDE_MFT   => 120000000;
use constant INITIAL_VELOCITY_MFT_S => -3000000;
use constant EXHAUST_VELOCITY_MFT_S => 8000000;
use constant MAX_BURN_RATE_LBS_S    => 200;
use constant MIN_DURATION_S         => 1;
use constant MAX_DURATION_S         => 10;
use constant MAX_TIME_SECONDS       => 86400;
use constant MAX_ALTITUDE_MFT       => 200000000;
use constant MIN_VELOCITY_MFT_S     => -10000000;
use constant MAX_VELOCITY_MFT_S     => 10000000;

# ============================================================================
# CodingAdventures::LunarLander::LanderError
# Structured closed refusal enum
# ============================================================================

package CodingAdventures::LunarLander::LanderError;

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

package CodingAdventures::LunarLander;

sub _err {
    my ($kind, $msg) = @_;
    die CodingAdventures::LunarLander::LanderError->new($kind, $msg);
}

# Strict integer validation:
# - Rejects undef, non-Math::BigInt refs (CWE-843)
# - Rejects floats (NV without IV flag) via B::svref_2object
# - Rejects strings with non-digits or formatting characters via anchored regex
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
    return 0 unless ($flags & B::SVf_IOK);
    return 0 if ($flags & B::SVf_NOK) && !($flags & B::SVf_IOK);
    my $str = "$val";
    return 0 unless $str =~ /\A-?[0-9]+\z/;
    return 0 unless $val == int($val);
    return 1;
}

# ============================================================================
# Math Primitives
# ============================================================================

# Euclidean division (div_euclid):
# Ensures non-negative remainder r in [0, |b|), matching Rust i64::div_euclid.
# div_euclid(-46070, -5310) == 9.
sub div_euclid {
    my ($a, $b) = @_;
    _err('ArithmeticOverflow', 'Division by zero in div_euclid') if $b == 0;
    if (ref($a) || ref($b)) {
        my $ba = (ref($a) && eval { $a->isa('Math::BigInt') }) ? $a->copy() : Math::BigInt->new("$a");
        my $bb = (ref($b) && eval { $b->isa('Math::BigInt') }) ? $b->copy() : Math::BigInt->new("$b");
        my $q = $ba->copy()->bdiv($bb);
        my $r = $ba - $q * $bb;
        if ($r < 0) {
            $q = ($bb > 0) ? ($q - 1) : ($q + 1);
        }
        return $q <= 9223372036854775807 && $q >= -9223372036854775808 ? $q->numify() : $q;
    }
    use integer;
    my $q = int($a / $b);
    my $r = $a - $q * $b;
    if ($r < 0) {
        return ($b > 0) ? ($q - 1) : ($q + 1);
    }
    return $q;
}

# Exact integer square root floor(sqrt(n)) for 128-bit unsigned integers.
sub isqrt_u128 {
    my ($n) = @_;
    return 0 unless defined $n;
    my $bn = (ref($n) && eval { $n->isa('Math::BigInt') }) ? $n->copy() : Math::BigInt->new("$n");
    return 0 if $bn->is_zero() || $bn->is_negative();
    my $root = $bn->bsqrt();
    if ($root <= 9223372036854775807) {
        return $root->numify();
    }
    return $root->bstr();
}

# ============================================================================
# Core Simulation Functions
# ============================================================================

# Initializes a new Lunar Module descent flight at standard initial conditions.
sub new_flight {
    return {
        time_seconds          => 0,
        altitude_mft          => INITIAL_ALTITUDE_MFT,
        velocity_mft_s        => INITIAL_VELOCITY_MFT_S,
        fuel_lbs              => INITIAL_FUEL_LBS,
        dry_mass_lbs          => DRY_MASS_LBS,
        status                => 'InFlight',
        impact_velocity_mft_s => undef,
    };
}

# Validates candidate burn command against state and physical invariants.
# Canonical Precedence:
# 1. Lifecycle Check: craft must be InFlight.
# 2. State Invariants: dry mass integrity, fuel capacity, altitude bounds,
#    velocity bounds, mission time limit, impact velocity absence.
# 3. Command Bounds: maximum burn rate (<= 200), duration (1..=10), mission time non-overflow.
sub validate_burn {
    my ($state, $cmd) = @_;

    _err('ArithmeticOverflow', 'Invalid state or command object')
        unless defined $state && ref($state) eq 'HASH'
            && defined $cmd && ref($cmd) eq 'HASH';

    # 1. Lifecycle Check
    my $status = $state->{status};
    _err('ArithmeticOverflow', 'Missing or invalid flight status')
        unless defined $status && !ref($status);

    if ($status ne 'InFlight') {
        if ($status =~ /\ALanded\((Perfect|Hard|Crash|Fatal|Crater)\)\z/) {
            _err('FlightAlreadyEnded', "Flight has already ended with status: $status");
        }
        _err('ArithmeticOverflow', "Unrecognized flight status: $status");
    }

    # 2. State Invariants
    _err('ArithmeticOverflow', 'Corrupted dry mass')
        unless _is_strict_int($state->{dry_mass_lbs})
            && $state->{dry_mass_lbs} == DRY_MASS_LBS;

    _err('ArithmeticOverflow', 'Invalid fuel capacity')
        unless _is_strict_int($state->{fuel_lbs})
            && $state->{fuel_lbs} >= 0
            && $state->{fuel_lbs} <= INITIAL_FUEL_LBS;

    _err('ArithmeticOverflow', 'Altitude out of bounds')
        unless _is_strict_int($state->{altitude_mft})
            && $state->{altitude_mft} > 0
            && $state->{altitude_mft} <= MAX_ALTITUDE_MFT;

    _err('ArithmeticOverflow', 'Velocity out of bounds')
        unless _is_strict_int($state->{velocity_mft_s})
            && $state->{velocity_mft_s} >= MIN_VELOCITY_MFT_S
            && $state->{velocity_mft_s} <= MAX_VELOCITY_MFT_S;

    _err('ArithmeticOverflow', 'Time seconds out of bounds')
        unless _is_strict_int($state->{time_seconds})
            && $state->{time_seconds} >= 0
            && $state->{time_seconds} <= MAX_TIME_SECONDS;

    _err('ArithmeticOverflow', 'Impact velocity must be undef while in flight')
        if defined $state->{impact_velocity_mft_s};

    # 3. Command Bounds
    _err('ArithmeticOverflow', 'Burn rate must be strict non-negative integer')
        unless _is_strict_int($cmd->{burn_rate_lbs_s})
            && $cmd->{burn_rate_lbs_s} >= 0
            && $cmd->{burn_rate_lbs_s} <= 4294967295;

    _err('ExcessiveBurnRate', "Burn rate $cmd->{burn_rate_lbs_s} exceeds maximum engine capacity " . MAX_BURN_RATE_LBS_S)
        if $cmd->{burn_rate_lbs_s} > MAX_BURN_RATE_LBS_S;

    _err('InvalidDuration', 'Duration must be between 1 and 10 seconds')
        unless _is_strict_int($cmd->{duration_seconds})
            && $cmd->{duration_seconds} >= MIN_DURATION_S
            && $cmd->{duration_seconds} <= MAX_DURATION_S;

    my $t_next = $state->{time_seconds} + $cmd->{duration_seconds};
    _err('ArithmeticOverflow', 'Mission time overflow beyond 24 hours')
        if $t_next > MAX_TIME_SECONDS;

    return 1;
}

# Non-throwing validation helper returning ($ok, $err).
sub check_burn {
    my ($state, $cmd) = @_;
    my $ok = eval { validate_burn($state, $cmd); 1 };
    return ($ok, undef) if $ok;
    return (0, $@);
}

# Integrates a single discrete simulation turn under the commanded burn rate and duration.
sub step_burn {
    my ($state, $cmd) = @_;
    validate_burn($state, $cmd);

    my $dt = $cmd->{duration_seconds};
    my $fuel_req = $cmd->{burn_rate_lbs_s} * $dt;
    my $delta_f = $state->{fuel_lbs} < $fuel_req ? $state->{fuel_lbs} : $fuel_req;
    my $fuel_next = $state->{fuel_lbs} - $delta_f;

    use integer;
    my $avg_mass = $state->{dry_mass_lbs} + $state->{fuel_lbs} - ($delta_f / 2);
    my $impulse = $delta_f * EXHAUST_VELOCITY_MFT_S;
    my $a_thrust = $impulse / ($dt * $avg_mass);
    my $a_net = $a_thrust - G_LUNAR_MFT_S2;

    my $v_next = $state->{velocity_mft_s} + $a_net * $dt;
    my $y_next = $state->{altitude_mft} + $state->{velocity_mft_s} * $dt + div_euclid($a_net * $dt * $dt, 2);
    my $t_next = $state->{time_seconds} + $cmd->{duration_seconds};
    no integer;

    if ($y_next > MAX_ALTITUDE_MFT || $v_next < MIN_VELOCITY_MFT_S || $v_next > MAX_VELOCITY_MFT_S) {
        _err('ArithmeticOverflow', 'Kinematic bounds exceeded after integration step');
    }

    if ($y_next > 0) {
        return {
            time_seconds          => $t_next,
            altitude_mft          => $y_next,
            velocity_mft_s        => $v_next,
            fuel_lbs              => $fuel_next,
            dry_mass_lbs          => $state->{dry_mass_lbs},
            status                => 'InFlight',
            impact_velocity_mft_s => undef,
        };
    } else {
        # Ground touchdown solver
        my $a_half = div_euclid($a_net, 2);
        my $delta_t_raw;
        if ($a_half == 0) {
            _err('ArithmeticOverflow', 'Zero net acceleration with zero velocity at touchdown')
                if $state->{velocity_mft_s} == 0;
            $delta_t_raw = div_euclid(-$state->{altitude_mft}, $state->{velocity_mft_s});
        } else {
            my $v128 = Math::BigInt->new("$state->{velocity_mft_s}");
            my $y128 = Math::BigInt->new("$state->{altitude_mft}");
            my $a128 = Math::BigInt->new("$a_half");
            my $disc = $v128 * $v128 - 4 * $a128 * $y128;
            my $root = ($disc < 0) ? 0 : $disc->bsqrt()->numify();
            $delta_t_raw = div_euclid(-$state->{velocity_mft_s} - $root, 2 * $a_half);
        }

        my $delta_t = $delta_t_raw < 1 ? 1 : ($delta_t_raw > $dt ? $dt : $delta_t_raw);
        my $v_touchdown = $state->{velocity_mft_s} + $a_net * $delta_t;
        my $t_final = $state->{time_seconds} + $delta_t;

        my $fuel_touchdown_req = $delta_t * $cmd->{burn_rate_lbs_s};
        my $fuel_burned_touchdown = $state->{fuel_lbs} < $fuel_touchdown_req ? $state->{fuel_lbs} : $fuel_touchdown_req;
        my $fuel_final = $state->{fuel_lbs} - $fuel_burned_touchdown;

        my $speed = abs($v_touchdown);
        my $outcome;
        if ($speed <= 5000) {
            $outcome = 'Perfect';
        } elsif ($speed <= 15000) {
            $outcome = 'Hard';
        } elsif ($speed <= 30000) {
            $outcome = 'Crash';
        } elsif ($speed <= 60000) {
            $outcome = 'Fatal';
        } else {
            $outcome = 'Crater';
        }

        return {
            time_seconds          => $t_final,
            altitude_mft          => 0,
            velocity_mft_s        => $v_touchdown,
            fuel_lbs              => $fuel_final,
            dry_mass_lbs          => $state->{dry_mass_lbs},
            status                => "Landed($outcome)",
            impact_velocity_mft_s => $v_touchdown,
        };
    }
}

# ============================================================================
# ASCII Telemetry & PaintScene Presentation Bridge
# ============================================================================

sub format_with_commas {
    my ($n) = @_;
    my $neg = $n < 0;
    my $abs = abs($n);
    my $s = "$abs";
    $s =~ s/(?<=\d)(?=(\d{3})+(?!\d))/,/g;
    return $neg ? "-$s" : "$s";
}

# Translates flight state into a declarative 76x5 PaintScene intermediate representation.
sub to_paint_scene {
    my ($state, $last_burn_rate_lbs_s) = @_;
    $last_burn_rate_lbs_s //= 0;

    my $alt_mft = $state->{altitude_mft};
    my $alt_clamped = $alt_mft < 0 ? 0 : ($alt_mft > INITIAL_ALTITUDE_MFT ? INITIAL_ALTITUDE_MFT : $alt_mft);
    use integer;
    my $alt_pct = ($alt_clamped * 100) / INITIAL_ALTITUDE_MFT;
    my $alt_bars = ($alt_clamped * 20) / INITIAL_ALTITUDE_MFT;

    my $fuel_lbs = $state->{fuel_lbs};
    my $fuel_clamped = $fuel_lbs > INITIAL_FUEL_LBS ? INITIAL_FUEL_LBS : ($fuel_lbs < 0 ? 0 : $fuel_lbs);
    my $fuel_pct = ($fuel_clamped * 100) / INITIAL_FUEL_LBS;
    my $fuel_bars = ($fuel_clamped * 20) / INITIAL_FUEL_LBS;

    my $alt_ft = $state->{altitude_mft} / 1000;
    my $vel_fps = $state->{velocity_mft_s} / 1000;
    no integer;

    my $alt_str = format_with_commas($alt_ft);
    my $fuel_str = format_with_commas($fuel_lbs);

    my ($rate_label, $rate_text, $rate_symbol);
    my $status = $state->{status} // '';
    if ($status eq 'InFlight') {
        if ($state->{velocity_mft_s} < 0) {
            ($rate_label, $rate_text, $rate_symbol) = ('RATE:', 'DESCENT', '▼');
        } elsif ($state->{velocity_mft_s} > 0) {
            ($rate_label, $rate_text, $rate_symbol) = ('RATE:', 'ASCENT', '▲');
        } else {
            ($rate_label, $rate_text, $rate_symbol) = ('RATE:', 'HOVER', '◆');
        }
    } elsif ($status =~ /\ALanded\((Perfect|Hard|Crash|Fatal|Crater)\)\z/) {
        my $outcome = uc($1);
        ($rate_label, $rate_text, $rate_symbol) = ('STATUS:', $outcome, '★');
    } else {
        ($rate_label, $rate_text, $rate_symbol) = ('STATUS:', 'UNKNOWN', '?');
    }

    my $row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐";
    my $left1 = sprintf(" TIME: %6d SEC    ", $state->{time_seconds});
    my $right1 = sprintf(" ALTITUDE: %7s FT [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", $alt_str, $alt_pct);
    my $row1 = "│${left1}│${right1}│";

    my $left2 = sprintf(" VELOCITY: %5d FT/S", $vel_fps);
    my $right2 = sprintf(" FUEL:    %7s LBS [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", $fuel_str, $fuel_pct);
    my $row2 = "│${left2}│${right2}│";

    my $left3 = sprintf(" BURN:     %4d LB/S ", $last_burn_rate_lbs_s);
    my $rate_col = sprintf("%-7s %-8s %-1s", $rate_label, $rate_text, $rate_symbol);
    my $row3 = sprintf("│%s│ %-51s│", $left3, $rate_col);
    my $row4 = "└──────────────────────────────────────────────────────────────────────────┘";

    my $full_text = "$row0\n$row1\n$row2\n$row3\n$row4";

    my @instructions = (
        {
            kind       => 'text',
            x          => 0.0,
            y          => 0.0,
            text       => $full_text,
            font_size  => 1.0,
            fill       => '#ffffff',
            text_align => 'left',
        }
    );

    if ($alt_bars > 0) {
        push @instructions, {
            kind   => 'rect',
            x      => 46.0,
            y      => 1.0,
            width  => ($alt_bars - 1) + 0.0,
            height => 0.0,
            fill   => '#00ff00',
        };
    }

    if ($fuel_bars > 0) {
        push @instructions, {
            kind   => 'rect',
            x      => 46.0,
            y      => 2.0,
            width  => ($fuel_bars - 1) + 0.0,
            height => 0.0,
            fill   => '#00ff00',
        };
    }

    return {
        width        => 76.0,
        height       => 5.0,
        background   => '#000000',
        instructions => \@instructions,
    };
}

# Convenience helper translating flight state into a PaintScene with zero burn rate.
sub telemetry_scene {
    my ($state) = @_;
    return to_paint_scene($state, 0);
}

# Renders a flight state telemetry dashboard into a deterministic ASCII string.
sub render_telemetry_ascii {
    my ($state, $last_burn_rate_lbs_s) = @_;
    $last_burn_rate_lbs_s //= 0;
    my $scene = to_paint_scene($state, $last_burn_rate_lbs_s);

    my $text_inst = $scene->{instructions}[0];
    my @lines = split /\n/, $text_inst->{text};
    my @grid = map { [ split //, $_ ] } @lines;

    for my $inst (@{ $scene->{instructions} }) {
        if ($inst->{kind} eq 'rect') {
            my $r = int($inst->{y});
            my $c_start = int($inst->{x});
            my $c_end = int($inst->{x} + $inst->{width});
            for my $c ($c_start .. $c_end) {
                if ($r >= 0 && $r < 5 && $c >= 0 && $c < 76) {
                    $grid[$r][$c] = '█';
                }
            }
        }
    }

    return join("\n", map { my $line = join('', @$_); $line =~ s/\s+\z//; $line } @grid);
}

# Convenience helper rendering a flight state telemetry dashboard with zero burn rate.
sub render_telemetry {
    my ($state) = @_;
    return render_telemetry_ascii($state, 0);
}

1;

__END__

=pod

=head1 NAME

CodingAdventures::LunarLander - Pure Discrete Fixed-Point Flight Simulation Core (GAME02)

=head1 SYNOPSIS

    use CodingAdventures::LunarLander qw(
        new_flight
        validate_burn
        step_burn
        render_telemetry_ascii
    );

    my $state = new_flight();
    my $cmd = { burn_rate_lbs_s => 50, duration_seconds => 10 };

    validate_burn($state, $cmd);
    my $next = step_burn($state, $cmd);
    print render_telemetry_ascii($next, 50);

=head1 DESCRIPTION

Pure, deterministic simulation core and ASCII telemetry for the classic
discrete lunar landing flight simulation (GAME02). Declares C<capabilities: []>.

=cut
