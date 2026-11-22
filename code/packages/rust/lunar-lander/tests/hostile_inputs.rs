//! Hostile-input and deterministic fuzz test suite for `lunar-lander`.

#![forbid(unsafe_code)]

use lunar_lander::{
    isqrt_u128, new_flight, render_telemetry, render_telemetry_ascii, step_burn, telemetry_scene,
    to_paint_scene, validate_burn, BurnCommand, FlightStatus, LanderError, LanderState,
    LandingOutcome, DRY_MASS_LBS, INITIAL_FUEL_LBS, MAX_ALTITUDE_MFT, MAX_BURN_RATE_LBS_S,
    MAX_DURATION_S, MAX_TIME_SECONDS, MAX_VELOCITY_MFT_S, MIN_VELOCITY_MFT_S,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    fn next_u128(&mut self) -> u128 {
        (u128::from(self.next_u64()) << 64) | u128::from(self.next_u64())
    }
}

#[test]
fn hostile_integer_extremes_and_isqrt() {
    assert_eq!(isqrt_u128(0), 0);
    assert_eq!(isqrt_u128(1), 1);
    assert_eq!(isqrt_u128(u128::MAX), u128::from(u64::MAX));
    assert_eq!(isqrt_u128(u128::MAX - 1), u128::from(u64::MAX));

    let base = new_flight();
    let hostile_cmds = [
        BurnCommand {
            burn_rate_lbs_s: u32::MAX,
            duration_seconds: u32::MAX,
        },
        BurnCommand {
            burn_rate_lbs_s: MAX_BURN_RATE_LBS_S + 1,
            duration_seconds: 1,
        },
        BurnCommand {
            burn_rate_lbs_s: 0,
            duration_seconds: 0,
        },
        BurnCommand {
            burn_rate_lbs_s: 0,
            duration_seconds: MAX_DURATION_S + 1,
        },
    ];
    for cmd in &hostile_cmds {
        assert!(validate_burn(&base, cmd).is_err());
        assert!(step_burn(&base, cmd).is_err());
    }

    let hostile_states = [
        LanderState {
            time_seconds: u32::MAX,
            altitude_mft: i64::MAX,
            velocity_mft_s: i64::MAX,
            fuel_lbs: u32::MAX,
            dry_mass_lbs: u32::MAX,
            status: FlightStatus::InFlight,
            impact_velocity_mft_s: Some(i64::MIN),
        },
        LanderState {
            time_seconds: MAX_TIME_SECONDS,
            altitude_mft: i64::MIN,
            velocity_mft_s: i64::MIN,
            fuel_lbs: 0,
            dry_mass_lbs: 0,
            status: FlightStatus::InFlight,
            impact_velocity_mft_s: None,
        },
        LanderState {
            time_seconds: MAX_TIME_SECONDS,
            altitude_mft: 1,
            velocity_mft_s: 0,
            fuel_lbs: INITIAL_FUEL_LBS,
            dry_mass_lbs: DRY_MASS_LBS,
            status: FlightStatus::InFlight,
            impact_velocity_mft_s: None,
        },
        LanderState {
            status: FlightStatus::Landed(LandingOutcome::Perfect),
            ..base
        },
    ];

    let valid_cmd = BurnCommand {
        burn_rate_lbs_s: 100,
        duration_seconds: 1,
    };
    for st in &hostile_states {
        assert!(validate_burn(st, &valid_cmd).is_err());
        assert!(step_burn(st, &valid_cmd).is_err());
        let _ = to_paint_scene(st, u32::MAX);
        let _ = telemetry_scene(st);
        let _ = render_telemetry_ascii(st, u32::MAX);
        let _ = render_telemetry(st);
    }

    // Specific boundary overflow guard
    let mut near_end = base;
    near_end.time_seconds = MAX_TIME_SECONDS;
    assert!(matches!(
        step_burn(&near_end, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    ));
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0x102A_471A_0000_0001);

    for i in 0..1_000 {
        let sqrt_in = match i % 5 {
            0 => 0,
            1 => u128::MAX,
            _ => rng.next_u128(),
        };
        let r = isqrt_u128(sqrt_in);
        if let Some(sq) = r.checked_mul(r) {
            assert!(sq <= sqrt_in);
        }

        let state = LanderState {
            time_seconds: if i % 7 == 0 {
                u32::MAX
            } else if i % 3 == 0 {
                rng.next_u32() % (MAX_TIME_SECONDS + 100)
            } else {
                rng.next_u32()
            },
            altitude_mft: match i % 6 {
                0 => i64::MIN,
                1 => i64::MAX,
                2 => (rng.next_i64() % (MAX_ALTITUDE_MFT + 10)).abs(),
                _ => rng.next_i64(),
            },
            velocity_mft_s: match i % 6 {
                0 => i64::MIN,
                1 => i64::MAX,
                2 => rng.next_i64().clamp(MIN_VELOCITY_MFT_S, MAX_VELOCITY_MFT_S),
                _ => rng.next_i64(),
            },
            fuel_lbs: if i % 5 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % (INITIAL_FUEL_LBS + 1_000)
            },
            dry_mass_lbs: if i % 4 == 0 {
                DRY_MASS_LBS
            } else {
                rng.next_u32()
            },
            status: if i % 8 == 0 {
                FlightStatus::Landed(LandingOutcome::Crater)
            } else {
                FlightStatus::InFlight
            },
            impact_velocity_mft_s: if i % 9 == 0 {
                Some(rng.next_i64())
            } else {
                None
            },
        };

        let cmd = BurnCommand {
            burn_rate_lbs_s: if i % 5 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % (MAX_BURN_RATE_LBS_S + 50)
            },
            duration_seconds: if i % 7 == 0 {
                u32::MAX
            } else {
                rng.next_u32() % (MAX_DURATION_S + 5)
            },
        };

        let _ = validate_burn(&state, &cmd);
        let _ = step_burn(&state, &cmd);
        let _ = to_paint_scene(&state, cmd.burn_rate_lbs_s);
        let _ = render_telemetry_ascii(&state, cmd.burn_rate_lbs_s);
    }
}
